# PocketCW

Free replacement firmware for the HamCube Morse code trainer. It runs on both variants: the Mini, and the Pro, which adds a decoder board for radio audio.

PocketCW is not made by HamCube, and is not affiliated with HamCube or Nintendo.

## What it does

- **Keyer:** iambic A and B paddles, straight keys and sideswipers, with a live decoder and a timing tape.
- **Timing:** element timing graphs and grades for your sending.
- **Receive:** copy drills, and ragchews: whole contacts between two stations, with a quiz.
- **POTA, SOTA and Hunt:** practice park and summit contacts, both as the activator and as the hunter.
- **Letters and Progress:** accuracy for each letter, common mixups, and session history.
- **USB keyboard:** types what you key on a computer, phone or tablet, or keys VBand.
- **Radio:** decodes a radio's audio through the HamCube decoder board, when one is fitted.

## Install

On a Mac, paste this line into Terminal, and follow the steps it prints:

```bash
curl -fsSL https://kvmet.github.io/k5mme_pocketcw/install.sh | sh
```

On Windows and Linux, put the device in update mode and copy files from the newest release to its drive: `F000400.bin` on a first install, then `A000014.bin`. The install never changes the device's bootloader, so you can always install again or go back to the original firmware.

The [user manual](docs/index.md) has the full install steps, how to go back to stock, and a guide to every screen.

## Build from source

You need Rust with the `thumbv7em-none-eabihf` target and `llvm-tools`:

```bash
rustup target add thumbv7em-none-eabihf
```

```bash
rustup component add llvm-tools
```

Then build the image and install it:

```bash
sh tools/mkimage.sh
```

`mkimage.sh` writes `firmware/target/A000014.bin` and prints the `pocketcw-flash` command that installs it. The [developer docs](docs/index.md) cover the hardware, the code layout and the release steps.

## License

PocketCW is licensed under the [GNU General Public License v3.0 or later](LICENSE).

The firmware draws text with [B612 Mono](https://github.com/polarsys/b612), licensed under the [SIL Open Font License 1.1](firmware/src/B612-OFL.txt).

Vendor firmware files are not part of this project. To go back to stock, get them from [admvip/CW-Pokemon-Infomation](https://github.com/admvip/CW-Pokemon-Infomation).
