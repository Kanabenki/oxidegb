use std::{fs::read, path::PathBuf};

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
}

impl Emulator {
    fn new(rom: Vec<u8>, bootrom: Option<Vec<u8>>) -> Result<Self, anyhow::Error> {
        let event_loop = EventLoop::new();

        let window = WindowBuilder::new()
            .with_title("Oxidegb")
            .with_inner_size(LogicalSize::new(128, 128))
            .build(&event_loop)
            .unwrap();

        let pixels = {
            let window_size = window.inner_size();
            let surface_texture =
                SurfaceTexture::new(window_size.width, window_size.height, &window);
            Pixels::new(128, 128, surface_texture).unwrap()
        };

        let gameboy = Gameboy::new(rom, bootrom)?;
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
            *control_flow = ControlFlow::Wait;

            match event {
                Event::WindowEvent {
                    window_id,
                    event: WindowEvent::CloseRequested,
                } if window_id == self.window.id() => *control_flow = ControlFlow::Exit,
                Event::MainEventsCleared => self.window.request_redraw(),
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
}

fn main() -> Result<(), anyhow::Error> {
    let arguments = Arguments::from_args();
    let rom = read(arguments.file)?;
    let bootrom = arguments
        .bootrom_file
        .map_or(Ok(None), |bootrom_file| read(bootrom_file).map(Some))?;
    let emulator = Emulator::new(rom, bootrom)?;
    emulator.run();
}
