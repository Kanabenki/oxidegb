use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Seek, SeekFrom, Write},
    mem,
    path::PathBuf,
};

use blip_buf::BlipBuf;
use clap::Parser;
use color_eyre::{
    eyre::{self, eyre, WrapErr},
    Report,
};
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
    // TODO: Remove option once the surface recreation problem is fixed by the pixels crate, or fast forward is handled differently.
    pixels: Option<Pixels>,
    shift_held: bool,
    tmp_sound_buf: [i16; 2048],
    resampling_bufs: (BlipBuf, BlipBuf),
    sound_prod: HeapProducer<i16>,
    _sound_stream: Stream,
    gameboy: Gameboy,
    rom_path: PathBuf,
    save_file: Option<File>,
    delta: u64,
    fast_forward: bool,
}

impl Emulator {
    fn create_pixels(
        window_width: u32,
        window_height: u32,
        window: &Window,
        vsync: bool,
    ) -> color_eyre::Result<Pixels> {
        Ok(PixelsBuilder::new(
            160,
            144,
            SurfaceTexture::new(window_width, window_height, window),
        )
        .enable_vsync(vsync)
        .build()?)
    }

    fn new(
        rom: Vec<u8>,
        bootrom: Option<Vec<u8>>,
        rom_path: PathBuf,
        save_path: Option<PathBuf>,
        fast_forward: bool,
        debug: bool,
    ) -> color_eyre::Result<Self> {
        let event_loop = EventLoop::new();

        let window = WindowBuilder::new()
            .with_title("Oxidegb")
            .build(&event_loop)?;

        let window_size = window.inner_size();
        let pixels = Some(Self::create_pixels(
            window_size.width,
            window_size.height,
            &window,
            !fast_forward,
        )?);

        let save_path = save_path.unwrap_or_else(|| rom_path.with_extension("sav"));
        let mut save_file = OpenOptions::new();
        let file_res = save_file.read(true).write(true).open(&save_path);
        let (save_data, save_file) = match file_res {
            Ok(mut save_file) => {
                let mut save_data = vec![];
                save_file.read_to_end(&mut save_data)?;
                (Some(save_data), Some(save_file))
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => (None, None),
            Err(error) => return Err(error.into()),
        };

        let gameboy = Gameboy::new(rom, bootrom, save_data, debug)?;

        let save_file = if gameboy.can_save() {
            if save_file.is_some() {
                save_file
            } else {
                let mut save_file = OpenOptions::new();
                Some(
                    save_file
                        .read(true)
                        .write(true)
                        .create(true)
                        .open(&save_path)?,
                )
            }
        } else {
            None
        };

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
            move |error| {
                eprintln!(
                    "{:?}",
                    Report::from(error).wrap_err("Error occurred in audio stream")
                )
            },
            None,
        )?;

        sound_stream.play()?;

        Ok(Self {
            event_loop,
            window,
            pixels,
            shift_held: false,
            fast_forward,
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
                    for (i, pixel) in self
                        .pixels
                        .as_mut()
                        .unwrap()
                        .frame_mut()
                        .chunks_exact_mut(4)
                        .enumerate()
                    {
                        let color: [u8; 4] = screen[i].into();
                        pixel.copy_from_slice(&color);
                    }
                    if self.pixels.as_ref().unwrap().render().is_err() {
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
                        KeyCode::KeyF if set => {
                            self.fast_forward = !self.fast_forward;
                            let window_size = self.window.inner_size();
                            mem::take(&mut self.pixels);
                            self.pixels = Some(
                                Self::create_pixels(
                                    window_size.width,
                                    window_size.height,
                                    &self.window,
                                    !self.fast_forward,
                                )
                                .unwrap(),
                            );
                        }
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
                            let load_res: Result<(), eyre::Error> = (|| {
                                let savestate = File::open(savestate_path).wrap_err_with(|| {
                                    format!("Cannot load savestate {index} file")
                                })?;
                                let gameboy =
                                    ciborium::from_reader(savestate).wrap_err_with(|| {
                                        format!("Cannot load savestate {index} content")
                                    })?;
                                self.gameboy.reinit(gameboy)?;
                                Ok(())
                            })(
                            );
                            match load_res {
                                Ok(()) => self.window.request_redraw(),
                                Err(error) => eprintln!("{error:?}"),
                            }
                        } else {
                            let save_res: Result<(), eyre::Error> = (|| {
                                ciborium::into_writer(
                                    &self.gameboy,
                                    File::create(savestate_path).wrap_err_with(|| {
                                        format!("Cannot create savestate {index} file")
                                    })?,
                                )
                                .wrap_err_with(|| {
                                    format!("Cannot save savestate {index} content")
                                })?;
                                Ok(())
                            })(
                            );
                            if let Err(error) = save_res {
                                eprintln!("{error:?}");
                            }
                        }
                    }
                }
                Event::WindowEvent {
                    window_id,
                    event: WindowEvent::Resized(size),
                } if window_id == self.window.id() => {
                    self.pixels
                        .as_mut()
                        .unwrap()
                        .resize_surface(size.width, size.height)
                        .unwrap();
                }
                Event::WindowEvent {
                    window_id,
                    event: WindowEvent::CloseRequested,
                } if window_id == self.window.id() => {
                    if let Some(save_data) = self.gameboy.save_data() {
                        if let Some(save_file) = self.save_file.as_mut() {
                            let len = save_data.len();

                            if let Err(error) = (|| {
                                save_file.seek(SeekFrom::Start(0))?;
                                save_file.write_all(save_data)?;
                                save_file.set_len(len as u64)
                            })() {
                                eprintln!("{error:?}");
                            }
                        }
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

    let emulator = Emulator::new(
        rom,
        bootrom,
        arguments.file,
        arguments.save_file,
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
