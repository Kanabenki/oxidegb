# OxideGB

OxideGB is a Game Boy emulator written in Rust, mainly for learning purposes.
A lot of features are currently work in progress, most games work, but expect some bugs!

The emulator is implemented as a lib with a simple CLI frontend using `pixels` for display and `winit` for handling input.
The MSRV is tied to the latest Rust stable release.

## Usage

The CLI is as follows:

```text
Usage: oxidegb [OPTIONS] <FILE>

Arguments:
  <FILE>  The rom file to load

Options:
  -s, --save-file <SAVE_FILE>        The save file path to use. By default, oxidegb will load and save from a sav file with the same base name as the rom file
  -b, --bootrom-file <BOOTROM_FILE>  The bootrom file to load
  -i, --info                         Display rom header info
  -d, --debug                        Enable the debugger
  -f, --fast-forward                 Do not limit fps
  -h, --help                         Print help
  -V, --version                      Print version
```

Controls are hardcoded for now to the following keys:

- D-Pad: Keyboard keys
- Buttons:
  - A: K
  - B: J
  - Start: I
  - Select: U
- Save states: 1-10 to save in the corresponding slot, Caps + 1-10 to load
- Fast forward toggle: F
- Start debugger: P (type help for a list of commands)

## Progress status

- What's working
  - CPU and PPU implementation
  - Most mappers (Rom only, MBC1, MBC2, MBC3 with RTC, MBC5)
  - Basic command line debugger
  - Cartridge RAM save, including RTC data
  - Save states
- What's not:
  - APU (in progress)
  - Game Boy Color mode

## Validations ROMs

| Blargg Rom     | Status |
|----------------|--------|
| cgb_sound      | ❌      |
| cpu_instrs     | ✔️      |
| dmg_sound      | ❌      |
| halt_bug       | ❌      |
| instr_timing   | ✔️      |
| interrupt_time | ❌      |
| mem_timing     | ✔️      |
| mem_timing_2   | ✔️      |
| oam_bug        | ❌      |
