# Code layout

PocketCW is two Rust crates. Both are `no_std`.

| Crate | Path | Builds for | Holds |
| - | - | - | - |
| `pocketcw-logic` | `logic/` | Any target | Keyer, decoders, drills, bots, sound synthesis, settings and records. No hardware access. |
| `pocketcw-firmware` | `firmware/` | `thumbv7em-none-eabihf` only | Drivers, the screens, and the RTIC app in `main.rs`. |

## Why two crates

Everything in `logic/` runs with `cargo test` on your computer, with no device and no probe. The rules that decide what the device does live there: keying, decoding, scoring, the QSO and hunt bots, the battery gauge, and the byte layout of each EEPROM record. They are the parts most likely to have bugs that a test can catch.

`firmware/` holds what cannot run on a computer: register access, interrupts, the screen, and the order of startup. It stays thin. It reads inputs, passes them to `logic`, and draws or outputs the result.

When you add code, put it in `logic` unless it needs a register, a pin or the screen.

## logic modules

| Module | What it does |
| - | - |
| `keyboard` | USB keyboard output: the key for each decoded character, the typing queue, and the Ctrl keys for VBand. |
| `keyer` | The iambic and straight keyer, the key jack debounce and stuck contact rules, and the func jack output lines. |
| `decoder` | Decodes your own keying into characters. |
| `listen` | Decodes a keyed line at any speed from 5 to 60 WPM: straight key keying, and the decoder board's tone line. |
| `timing`, `decode_lines` | Grades marks and gaps, keeps keying statistics, and lays out decoded text in lines. |
| `morse`, `text` | The Morse table, prosigns, fixed size strings, and word wrap. |
| `player` | Sends text as Morse, with optional human timing errors. |
| `synth`, `volume` | Mixes the sidetone, up to four stations, band noise and fading into one signal. Maps volume levels to codec registers. |
| `adpcm` | Decodes voice clips and raises their sample rate by a whole factor. |
| `drill`, `letters`, `ragchew` | Receive drills, Koch levels, letter statistics, and ragchew QSOs. |
| `qso`, `hunt` | The bots for activating and for hunting. |
| `progress` | Practice history, one slot per session. |
| `memories`, `settings` | The memories and the settings, with their EEPROM records. |
| `record` | The EEPROM record frame and its CRC. |
| `sfdp` | Reads the table a serial flash chip gives about itself: size, page size, erase types and times. |
| `battery`, `panel`, `board` | The battery gauge, the choice of screen type and its memory read framing, and the decoder board protocol. |
| `serial` | Commands from the computer on the USB serial port. |
| `config` | Settings and memories by name, for the serial port's GET and SET. |
| `sender`, `winkeyer` | The sender that keys text from the serial port, and the WinKeyer host protocol. |
| `buttons`, `input`, `ticks`, `rng`, `math` | Button debounce and events, button actions, tick arithmetic, random numbers, and math helpers. |
| `dial` | What the dial sets on each screen, and the hold on dial turns while you key. |

## firmware modules

| Module | What it does |
| - | - |
| `main.rs` | The RTIC app: startup, the tasks, saving, and power off. |
| `pins` | Every pin number. See [Pins](hardware/pins.md). |
| `clock`, `gpio`, `power`, `fault` | Clock setup, pin setup, power hold and standby, and the fault screen. |
| `lcd`, `backlight`, `gfx`, `font`, `fonts`, `theme`, `boot` | The screen driver, drawing, fonts, styles and palettes, and the boot screen. |
| `audio`, `codec`, `es8388`, `wm8978` | The PA8 audio output and the two audio codecs. |
| `i2c`, `eeprom`, `store` | The I2C bus, the EEPROM, and where each record lives. |
| `spi_flash` | The 16 MB SPI flash, which shares SPI1 with the screen. |
| `battery` | ADC readings of the battery and the supply. |
| `link` | The serial link to the decoder board. |
| `usb` | The USB device peripheral for `stm32-usbd`, and the USB keyboard and serial port. |
| `ui/` | One file per screen, plus shared parts: `dial` for the header values, `exchange` and `transcript` for the QSO screens, `tape` for the timing tape, `serial` for the serial commands the screen answers. |
| `text` | A re-export of the fixed size string type. |
| `bin/probe.rs`, `bin/usbprobe.rs`, `bin/voiceprobe.rs`, `bin/flashprobe.rs` | Separate hardware, USB, voice and SPI flash probe images. See [Build and flash](building.md). |

## Tasks

`main.rs` is an RTIC 2 app. A higher priority interrupts a lower one.

| Task | Runs on | Priority | Work |
| - | - | - | - |
| `sample` | TMR1 overflow, 17,578 times a second while sound plays | 4 | The next audio sample. |
| `tick` | TMR3, every 1 ms | 3 | Key jack debounce, the keyer, the func jack and LED, the four players and the sidetone, both decoders, and button debounce. |
| `board_frame` | USART1 idle line | 2 | Parses a frame from the decoder board and answers it. |
| `usb_events` | USB interrupt, and a pend from `tick` | 2 | Answers the USB host and sends the next keyboard report. |
| `ui` | Async, software task | 1 | The boot screen, all drawing, settings changes, saves, battery polls, and power off. |

`tick` and `board_frame` send messages to `ui` on a channel 64 messages deep. When the channel is full, a message is lost and the status line shows `n INPUTS LOST`.

Keying and audio never wait for the screen. A full redraw takes about 70 ms, so each event redraws only what it changed.
