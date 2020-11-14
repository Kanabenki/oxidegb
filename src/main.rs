use std::{fs, path::PathBuf};

use pixels::{Pixels, SurfaceTexture};
use structopt::StructOpt;
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
    pixels: Pixels<Window>,
    gameboy: Gameboy,
    _scale: u32,
}

impl Emulator {
    fn new(rom: Vec<u8>, bootrom: Option<Vec<u8>>, scale: u32) -> Result<Self, anyhow::Error> {
        if !(1..=8).contains(&scale) {
            return Err(anyhow::anyhow!("Scale must be between 1 and 8"));
        }

        let event_loop = EventLoop::new();

        let window = WindowBuilder::new()
            .with_title("Oxidegb")
            .with_inner_size(LogicalSize::new(160 * scale, 144 * scale))
            .with_resizable(false)
            .build(&event_loop)
            .unwrap();

        let pixels = {
            let window_size = window.inner_size();
            let surface_texture =
                SurfaceTexture::new(window_size.width, window_size.height, &window);
            Pixels::new(160, 144, surface_texture)?
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
                    self.pixels
                        .get_frame()
                        .chunks_exact_mut(4)
                        .next()
                        .unwrap()
                        .copy_from_slice(&[0xFF, 0x00, 0xFF, 0xFF]);
                    if self.pixels.render().is_err() {
                        *control_flow = ControlFlow::Exit;
                    }
                }
                Event::WindowEvent {
                    window_id,
                    event: WindowEvent::CloseRequested,
                } if window_id == self.window.id() => *control_flow = ControlFlow::Exit,
                Event::MainEventsCleared => {
                    self.gameboy.tick();
                    self.window.request_redraw();
                }
                _ => (),
            }
        })
    }
}

#[derive(StructOpt, Debug)]
struct Arguments {
    /// The rom file to load.
    #[structopt(short = "f", long, parse(from_os_str))]
    file: PathBuf,
    /// The bootrom file to load.
    #[structopt(short = "b", long, parse(from_os_str))]
    bootrom_file: Option<PathBuf>,
    #[structopt(default_value = "4", short = "s", long)]
    scale: u32,
}

fn main() -> Result<(), anyhow::Error> {
    let arguments = Arguments::from_args();
    let rom = fs::read(arguments.file)?;
    let bootrom = arguments
        .bootrom_file
        .map_or(Ok(None), |bootrom_file| fs::read(bootrom_file).map(Some))?;
    let scale = arguments.scale;
    let emulator = Emulator::new(rom, bootrom, scale)?;
    emulator.run();
}
