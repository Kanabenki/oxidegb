use std::{fs, path::PathBuf};

use clap::Parser;
use color_eyre::eyre::eyre;
use pixels::{Pixels, PixelsBuilder, SurfaceTexture};
use winit::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

use oxidegb::gameboy::Gameboy;

struct Emulator {
    event_loop: Option<EventLoop<()>>,
    window: Window,
    pixels: Pixels,
    gameboy: Gameboy,
    _scale: u32,
}

impl Emulator {
    fn new(rom: Vec<u8>, bootrom: Option<Vec<u8>>, scale: u32) -> color_eyre::Result<Self> {
        if !(1..=8).contains(&scale) {
            return Err(eyre!("Scale must be between 1 and 8"));
        }

        let event_loop = EventLoop::new();

        let window = WindowBuilder::new()
            .with_title("Oxidegb")
            .with_inner_size(LogicalSize::new(160 * scale, 144 * scale))
            .with_resizable(false)
            .build(&event_loop)?;

        let pixels = {
            let window_size = window.inner_size();
            let surface_texture =
                SurfaceTexture::new(window_size.width, window_size.height, &window);
            PixelsBuilder::new(160, 144, surface_texture)
                .enable_vsync(false)
                .build()?
        };

        let gameboy = Gameboy::new(rom, bootrom)?;
        let event_loop = Some(event_loop);

        Ok(Self {
            event_loop,
            window,
            pixels,
            gameboy,
            _scale: scale,
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
    /// The scale of the emulator window
    #[arg(short, long, default_value = "4")]
    scale: u32,
    /// Display rom header info
    #[arg(short, long)]
    info: bool,
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let arguments = Arguments::parse();
    let rom = fs::read(arguments.file)?;
    let bootrom = arguments
        .bootrom_file
        .map_or(Ok(None), |bootrom_file| fs::read(bootrom_file).map(Some))?;
    let scale = arguments.scale;
    let emulator = Emulator::new(rom, bootrom, scale)?;
    if arguments.info {
        println!("{:?}", emulator.gameboy.rom_header());
    }
    emulator.run();
}
