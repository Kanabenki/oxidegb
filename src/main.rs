use std::{
    fs::{self, File},
    io::{Read, Seek, SeekFrom, Write},
    path::PathBuf,
};

use blip_buf::BlipBuf;
use clap::Parser;
use color_eyre::{eyre::eyre, Report};
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    SampleFormat, SampleRate, Stream, StreamConfig,
};
use pixels::{Pixels, PixelsBuilder, SurfaceTexture};
use ringbuf::{HeapProducer, HeapRb};
use winit::{
    event::{ElementState, Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::KeyCode,
    window::{Window, WindowBuilder},
};

use oxidegb::gameboy::{Button, Gameboy};

struct Emulator {
    event_loop: Option<EventLoop<()>>,
    window: Window,
    pixels: Pixels,
    shift_held: bool,
    tmp_sound_buf: [i16; 2048],
    resampling_bufs: (BlipBuf, BlipBuf),
    sound_prod: HeapProducer<i16>,
    _sound_stream: Stream,
    gameboy: Gameboy,
    rom_path: PathBuf,
    save_file: File,
    delta: u64,
}

impl Emulator {
    fn new(
        rom: Vec<u8>,
        bootrom: Option<Vec<u8>>,
        rom_path: PathBuf,
        mut save_file: File,
        fast_forward: bool,
        debug: bool,
    ) -> color_eyre::Result<Self> {
        let event_loop = EventLoop::new();

        let window = WindowBuilder::new()
            .with_title("Oxidegb")
            .build(&event_loop)?;

        let pixels = {
            let window_size = window.inner_size();
            let surface_texture =
                SurfaceTexture::new(window_size.width, window_size.height, &window);
            PixelsBuilder::new(160, 144, surface_texture)
                .enable_vsync(!fast_forward)
                .build()?
        };

        let mut save_data = vec![];
        save_file.read_to_end(&mut save_data)?;
        let save_data = if !save_data.is_empty() {
            Some(save_data)
        } else {
            None
        };
        let gameboy = Gameboy::new(rom, bootrom, save_data, debug)?;
        let event_loop = Some(event_loop);

        let sample_rate_out = 44100;
        let sound_host = cpal::default_host();
        let sound_device = sound_host
            .default_output_device()
            .ok_or(eyre!("Could not find an audio device"))?;
        let stream_config: StreamConfig = sound_device
            .supported_input_configs()?
            .find(|config| {
                config.channels() == 2
                    && config.sample_format() == SampleFormat::I16
                    && config.min_sample_rate().0 <= sample_rate_out
                    && config.max_sample_rate().0 >= sample_rate_out
            })
            .ok_or(eyre!("Could not find a compatible audio configuration"))?
            .with_sample_rate(SampleRate(44100))
            .into();

        let blip_buf = || {
            let mut buf = BlipBuf::new(512);
            buf.set_rates(
                (Gameboy::CYCLES_PER_SECOND / 4) as f64,
                sample_rate_out as f64,
            );
            buf
        };

        let resampling_bufs = (blip_buf(), blip_buf());

        let (sound_prod, mut sound_cons) = HeapRb::new(2048).split();

        let sound_stream = sound_device.build_output_stream(
            &stream_config,
            move |data: &mut [i16], _| {
                sound_cons.pop_slice(data);
            },
            move |error| eprintln!("Error occurred in audio stream: {}", Report::from(error)),
            None,
        )?;

        sound_stream.play()?;

        Ok(Self {
            event_loop,
            window,
            pixels,
            shift_held: false,
            sound_prod,
            tmp_sound_buf: [0; 2048],
            _sound_stream: sound_stream,
            resampling_bufs,
            gameboy,
            rom_path,
            save_file,
            delta: 0,
        })
    }

    fn run(mut self) -> ! {
        let event_loop = self.event_loop.take().unwrap();
        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Poll;

            match event {
                Event::RedrawRequested(_) => {
                    let screen = self.gameboy.screen();
                    for (i, pixel) in self.pixels.frame_mut().chunks_exact_mut(4).enumerate() {
                        let color: [u8; 4] = screen[i].into();
                        pixel.copy_from_slice(&color);
                    }
                    if self.pixels.render().is_err() {
                        *control_flow = ControlFlow::Exit;
                    }
                }
                Event::WindowEvent {
                    event: WindowEvent::ModifiersChanged(modifiers),
                    ..
                } => {
                    self.shift_held = modifiers.state().shift_key();
                }
                Event::WindowEvent {
                    event: WindowEvent::KeyboardInput { event, .. },
                    ..
                } => {
                    let set: bool = match event.state {
                        ElementState::Pressed => true,
                        ElementState::Released => false,
                    };
                    // TODO: Inputs are hardcoded for now.
                    // TODO: Check how to handle input being pressed during gb frame loop instead of waiting for the end of the frame
                    match event.physical_key {
                        KeyCode::KeyP if set => self.gameboy.debug_break(),
                        KeyCode::ArrowUp => self.gameboy.set_button(Button::Up, set),
                        KeyCode::ArrowDown => self.gameboy.set_button(Button::Down, set),
                        KeyCode::ArrowLeft => self.gameboy.set_button(Button::Left, set),
                        KeyCode::ArrowRight => self.gameboy.set_button(Button::Right, set),
                        KeyCode::KeyJ => self.gameboy.set_button(Button::B, set),
                        KeyCode::KeyK => self.gameboy.set_button(Button::A, set),
                        KeyCode::KeyU => self.gameboy.set_button(Button::Select, set),
                        KeyCode::KeyI => self.gameboy.set_button(Button::Start, set),
                        _ => {}
                    }

                    let savestate_index = match event.physical_key {
                        KeyCode::Digit0 if set => Some(0),
                        KeyCode::Digit1 if set => Some(1),
                        KeyCode::Digit2 if set => Some(2),
                        KeyCode::Digit3 if set => Some(3),
                        KeyCode::Digit4 if set => Some(4),
                        KeyCode::Digit5 if set => Some(5),
                        KeyCode::Digit6 if set => Some(6),
                        KeyCode::Digit7 if set => Some(7),
                        KeyCode::Digit8 if set => Some(8),
                        KeyCode::Digit9 if set => Some(9),
                        _ => None,
                    };

                    if let Some(index) = savestate_index {
                        let mut savestate_filename = self.rom_path.file_stem().unwrap().to_owned();
                        savestate_filename.push("_");
                        savestate_filename.push(index.to_string());
                        let savestate_path = self
                            .rom_path
                            .with_file_name(savestate_filename)
                            .with_extension("oxidegb");
                        if self.shift_held {
                            match File::open(savestate_path) {
                                Ok(savestate) => match ciborium::from_reader(savestate) {
                                    Ok(gameboy) => match self.gameboy.reinit(gameboy) {
                                        Ok(()) => self.window.request_redraw(),
                                        Err(error) => eprintln!(
                                            "Cannot load savestate {} content: {}",
                                            index,
                                            Report::from(error)
                                        ),
                                    },
                                    Err(error) => eprintln!(
                                        "Cannot decode savestate {} content: {}",
                                        index,
                                        Report::from(error)
                                    ),
                                },
                                Err(error) => eprintln!(
                                    "Cannot load savestate {} file: {}",
                                    index,
                                    Report::from(error)
                                ),
                            }
                        } else if let Err(error) = ciborium::into_writer(
                            &self.gameboy,
                            File::create(savestate_path).unwrap(),
                        ) {
                            eprintln!(
                                "Cannot save savestate {} file: {}",
                                index,
                                Report::from(error)
                            );
                        }
                    }
                }
                Event::WindowEvent {
                    window_id,
                    event: WindowEvent::Resized(size),
                } if window_id == self.window.id() => {
                    self.pixels.resize_surface(size.width, size.height).unwrap();
                }
                Event::WindowEvent {
                    window_id,
                    event: WindowEvent::CloseRequested,
                } if window_id == self.window.id() => {
                    if let Some(save_data) = self.gameboy.save_data() {
                        let len = save_data.len();
                        // TODO log any error
                        self.save_file.seek(SeekFrom::Start(0)).unwrap();
                        self.save_file.write_all(save_data).unwrap();
                        self.save_file.set_len(len as u64).unwrap();
                    }
                    *control_flow = ControlFlow::Exit;
                }
                Event::MainEventsCleared => {
                    let refresh_rate = self
                        .window
                        .current_monitor()
                        .unwrap()
                        .refresh_rate_millihertz()
                        .unwrap() as f32;
                    let ticks = (1000.0 * Gameboy::CYCLES_PER_SECOND as f32 / refresh_rate) as u64;

                    let mut total_cycles = 0;
                    self.delta = loop {
                        total_cycles += self.gameboy.run_instruction();
                        // Handle audio.
                        {
                            let (left, right, count) = self.gameboy.samples();
                            let (left_buf, right_buf) = &mut self.resampling_bufs;
                            let mut offset = 0;
                            for (&left, &right) in left[..count].iter().zip(&right[..count]) {
                                left_buf.add_delta(offset, left as i32);
                                right_buf.add_delta(offset, right as i32);
                                offset += 4;
                            }
                            left_buf.end_frame(offset);
                            right_buf.end_frame(offset);
                            let available = left_buf.samples_avail();
                            if available > 0 {
                                // TODO: Interleave directly in the ring buffer?
                                left_buf.read_samples(&mut self.tmp_sound_buf, true);
                                right_buf.read_samples(&mut self.tmp_sound_buf[1..], true);
                                self.sound_prod.push_slice(&self.tmp_sound_buf[..]);
                            }
                        }
                        if total_cycles >= ticks - self.delta {
                            break total_cycles - (ticks - self.delta);
                        }
                    };
                    self.window.request_redraw();
                }
                _ => (),
            }
        })
    }
}

/// Rust Gameboy emulator
#[derive(clap::Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Arguments {
    /// The rom file to load.
    file: PathBuf,
    /// The save file path to use. By default, oxidegb will load and save from a sav file with the same base name as the rom file.
    #[arg(short, long)]
    save_file: Option<PathBuf>,
    /// The bootrom file to load.
    #[arg(short, long)]
    bootrom_file: Option<PathBuf>,
    /// Display rom header info.
    #[arg(short, long)]
    info: bool,
    /// Enable the debugger.
    #[arg(short, long)]
    debug: bool,
    /// Do not limit fps.
    #[arg(short, long)]
    fast_forward: bool,
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let arguments = Arguments::parse();
    let rom = fs::read(&arguments.file)?;
    let bootrom = arguments
        .bootrom_file
        .map_or(Ok(None), |bootrom_file| fs::read(bootrom_file).map(Some))?;
    let mut save_open_options = fs::OpenOptions::new();
    save_open_options.read(true).write(true).create(true);
    let save = arguments.save_file.map_or_else(
        || save_open_options.open(arguments.file.with_extension("sav")),
        |save_file| save_open_options.open(save_file),
    )?;

    let emulator = Emulator::new(
        rom,
        bootrom,
        arguments.file,
        save,
        arguments.fast_forward,
        arguments.debug,
    )?;
    if arguments.info {
        println!(
            "{:?}\n{:?}",
            emulator.gameboy.rom_header(),
            emulator.gameboy.mapper()
        );
    }
    emulator.run();
}
