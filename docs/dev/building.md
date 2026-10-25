# Build and flash

## What you need

- Rust from [rustup](https://rustup.rs), version 1.88 or later. The code uses let chains, from the 2024 edition.
- The target for the AT32F403A, a Cortex-M4F:

  ```bash
  rustup target add thumbv7em-none-eabihf
  ```

- The LLVM tools. `mkimage.sh` takes `llvm-objcopy` and `llvm-objdump` from the rustup toolchain folder.

  ```bash
  rustup component add llvm-tools
  ```

- To flash from macOS: `clang` from the Xcode command line tools.

## Build the image

```bash
sh tools/mkimage.sh
```

The script builds `firmware/` in release mode and writes `firmware/target/A000014.bin`. `firmware/.cargo/config.toml` sets the target, so a plain `cargo build` in `firmware/` also builds for the device.

The script stops with an error in two cases:

- The image does not start at 0x08005000. The bootloader is below that address. See [Flash layout](hardware/flash-layout.md).
- The image is larger than 236 KB, the app area.

At the end, the script prints the command that flashes the image.

The linker script `firmware/memory.x` puts the app at 0x08005000, with 236 KB of flash and 64 KB of RAM. `firmware/build.rs` passes the `nmagic` option to the linker. Without it, the linker maps the ELF headers at 0x08000000, inside the bootloader, and `mkimage.sh` refuses the image.

## Flash the image

The device installs updates through its bootloader, which shows a USB drive. macOS must not mount that drive. See [Flash layout](hardware/flash-layout.md) for the reason.

1. Build the install tool once. `mkimage.sh` prints a command that runs it from `tools/`:

   ```bash
   clang -O2 -arch arm64 -arch x86_64 -mmacosx-version-min=10.15 -o tools/pocketcw-flash tools/pocketcw-flash.c -framework DiskArbitration -framework CoreFoundation
   ```

2. Run the command that `mkimage.sh` printed:

   ```bash
   tools/pocketcw-flash firmware/target/A000014.bin
   ```

3. Follow the steps the tool prints. macOS asks for your Mac login password, because the tool writes to the raw disk through `authopen`.

`pocketcw-flash` refuses a file that would write below 0x08005000, or past the end of the 256 KB flash. The [install guide](../user/install.md) and [troubleshooting](../user/troubleshooting.md) pages cover the steps on the device.

## Test

The tests run on your computer:

```bash
cd logic
cargo test
```

`logic/` has no target setting, so it builds for your computer. The firmware crate has no tests.

## Hardware probe

`firmware/src/bin/probe.rs` is a separate image for checking a board. It shows the chip and board IDs, reads the LCD memory back, scans the I2C bus, shows the live inputs, and tests the audio codec, the PA8 audio output and standby.

```bash
sh tools/mkimage.sh probe
```

The probe image also goes to `firmware/target/A000014.bin`. Flash it the same way, and flash the app again after.

## USB probe

`firmware/src/bin/usbprobe.rs` is a separate image for checking the USB device. It shows up on the computer as a USB keyboard and serial port named "PocketCW USB probe".

```bash
sh tools/mkimage.sh usbprobe
```

| Input | Sends |
| - | - |
| Dit contact (tip) | Holds Left Ctrl, as VBand reads a dit. |
| Dah contact (ring) | Holds Right Ctrl, as VBand reads a dah. |
| F1 | Types `cq test de pocketcw` and Enter. |
| Hold the dial 1.5 s | Turns the device off. |

The screen shows the clock source, the USB state, the bus reset count, and the reports the computer took. CONFIGURED means the computer accepted the keyboard. macOS may open its Keyboard Setup Assistant the first time; close it.

The image goes to `firmware/target/A000014.bin`. Flash it the same way, and flash the app again after.

## Remote control and screenshots

`tools/pocketcw.py` drives the device over USB serial. It presses the buttons, keys text, shows what the device decodes and plays, and saves the screen as a PNG. It needs Python 3 on macOS or Linux, with no other packages. [Serial interface](serial.md) describes the commands and the event lines.

1. Set USB OUT to SERIAL, so the device is on USB and types nothing.
2. Plug the device into the computer.
3. Run the steps, such as:

```bash
python3 -I tools/pocketcw.py F4 RIGHT CLICK shot timing.png
```

| Step | Does |
| - | - |
| `F1` to `F4`, `LEFT`, `RIGHT`, `CLICK` | Presses and releases that button. `LEFT` and `RIGHT` turn the dial one step. |
| `shot FILE.png` | Saves the screen. |
| `wait MS` | Pauses, for a screen that changes on its own. |
| `send TEXT` | Keys the text and waits until it is sent, then shows what the device decoded. Put text with spaces in quotes. |
| `expect REGEX` | Waits up to 60 s for an event line that matches from its start, such as `expect "PLAY "`. |
| `listen MS` | Shows the event lines that arrive in that time. |
| `shots DIR` | Saves every screen into the folder. See [Every screen](#every-screen). |

Event lines show as they arrive, after `< `. The script finds the serial port by itself when there is only one; else give it with `--port PORT` before the steps.

A Python script can load the file and use its `Device` class: `press`, `shot`, `send`, `expect`, `listen` and `drain` do what the steps do.

### Every screen

The `shots DIR` step saves a screenshot of every screen into a folder: KEYER and its MEM row, the menu, TIMING, the five receive drills, POTA, SOTA, HUNT, LETTERS, PROGRESS, SYSTEM, and each settings tab. Start it with the device on the KEYER screen; it ends there.

```bash
python3 -I tools/pocketcw.py shots shots
```

It keys back one group in each of the Koch, words and calls drills, so each shows a scored row. The ragchews run until one over ends, and each QSO runs for 4 s. All of them add to the practice history. The run takes about 3 minutes. RADIO is left out, because it shows only with the decoder board on.
