use std::{fs, path::PathBuf};

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
    event::{ElementState, Event, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

use oxidegb::gameboy::{Button, Gameboy};

struct Emulator {
    event_loop: Option<EventLoop<()>>,
    window: Window,
    pixels: Pixels,
    tmp_sound_buf: [i16; 2048],
    resampling_bufs: (BlipBuf, BlipBuf),
    sound_prod: HeapProducer<i16>,
    _sound_stream: Stream,
    gameboy: Gameboy,
    delta: u64,
}

impl Emulator {
    fn new(
        rom: Vec<u8>,
        bootrom: Option<Vec<u8>>,
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

        let gameboy = Gameboy::new(rom, bootrom, debug)?;
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
        )?;

        sound_stream.play()?;

        std::thread::sleep(std::time::Duration::from_millis(1000));

        Ok(Self {
            event_loop,
            window,
            pixels,
            sound_prod,
            tmp_sound_buf: [0; 2048],
            _sound_stream: sound_stream,
            resampling_bufs,
            gameboy,
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
                    event: WindowEvent::KeyboardInput { input, .. },
                    ..
                } => {
                    if let Some(key) = input.virtual_keycode {
                        let set: bool = match input.state {
                            ElementState::Pressed => true,
                            ElementState::Released => false,
                        };
                        // TODO: Inputs are hardcoded for now.
                        // TODO: Check how to handle input being pressed during gb frame loop instead of waiting for the end of the frame
                        match key {
                            VirtualKeyCode::P if set => self.gameboy.debug_break(),
                            VirtualKeyCode::Up => self.gameboy.set_button(Button::Up, set),
                            VirtualKeyCode::Down => self.gameboy.set_button(Button::Down, set),
                            VirtualKeyCode::Left => self.gameboy.set_button(Button::Left, set),
                            VirtualKeyCode::Right => self.gameboy.set_button(Button::Right, set),
                            VirtualKeyCode::J => self.gameboy.set_button(Button::B, set),
                            VirtualKeyCode::K => self.gameboy.set_button(Button::A, set),
                            VirtualKeyCode::U => self.gameboy.set_button(Button::Select, set),
                            VirtualKeyCode::I => self.gameboy.set_button(Button::Start, set),
                            _ => {}
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
                } if window_id == self.window.id() => *control_flow = ControlFlow::Exit,
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
    /// The rom file to load
    file: PathBuf,
    /// The bootrom file to load
    #[arg(short, long)]
    bootrom_file: Option<PathBuf>,
    /// Display rom header info
    #[arg(short, long)]
    info: bool,
    /// Enable the debugger
    #[arg(short, long)]
    debug: bool,
    /// Do not limit fps
    #[arg(short, long)]
    fast_forward: bool,
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let arguments = Arguments::parse();
    let rom = fs::read(arguments.file)?;
    let bootrom = arguments
        .bootrom_file
        .map_or(Ok(None), |bootrom_file| fs::read(bootrom_file).map(Some))?;
    let emulator = Emulator::new(rom, bootrom, arguments.fast_forward, arguments.debug)?;
    if arguments.info {
        println!(
            "{:?}\n{:?}",
            emulator.gameboy.rom_header(),
            emulator.gameboy.mapper()
        );
    }
    emulator.run();
}
