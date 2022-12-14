use std::{fs, path::PathBuf};

use clap::Parser;
use pixels::{Pixels, PixelsBuilder, SurfaceTexture};
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
    gameboy: Gameboy,
}

impl Emulator {
    fn new(rom: Vec<u8>, bootrom: Option<Vec<u8>>, debug: bool) -> color_eyre::Result<Self> {
        let event_loop = EventLoop::new();

        let window = WindowBuilder::new()
            .with_title("Oxidegb")
            .build(&event_loop)?;

        let pixels = {
            let window_size = window.inner_size();
            let surface_texture =
                SurfaceTexture::new(window_size.width, window_size.height, &window);
            PixelsBuilder::new(160, 144, surface_texture)
                .enable_vsync(false)
                .build()?
        };

        let gameboy = Gameboy::new(rom, bootrom, debug)?;
        let event_loop = Some(event_loop);

        Ok(Self {
            event_loop,
            window,
            pixels,
            gameboy,
        })
    }

    fn run(mut self) -> ! {
        let event_loop = self.event_loop.take().unwrap();
        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Poll;

            match event {
                Event::RedrawRequested(_) => {
                    let screen = self.gameboy.screen();
                    for (i, pixel) in self.pixels.get_frame_mut().chunks_exact_mut(4).enumerate() {
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
                    self.pixels.resize_surface(size.width, size.height)
                }
                Event::WindowEvent {
                    window_id,
                    event: WindowEvent::CloseRequested,
                } if window_id == self.window.id() => *control_flow = ControlFlow::Exit,
                Event::MainEventsCleared => {
                    self.gameboy.run_frame(0);
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
    #[arg(short, long)]
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
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let arguments = Arguments::parse();
    let rom = fs::read(arguments.file)?;
    let bootrom = arguments
        .bootrom_file
        .map_or(Ok(None), |bootrom_file| fs::read(bootrom_file).map(Some))?;
    let emulator = Emulator::new(rom, bootrom, arguments.debug)?;
    if arguments.info {
        println!("{:?}", emulator.gameboy.rom_header());
    }
    emulator.run();
}
