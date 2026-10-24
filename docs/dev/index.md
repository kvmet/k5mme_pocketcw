# Developer docs

These pages are for people who build PocketCW from source, change it, or want to know how the hardware works.

## Pages

- [Code layout](code-layout.md): the two crates, the tasks, and where new code goes.
- [Build and flash](building.md): tools, the image, the hardware probe, and the tests.
- [Fonts](fonts.md): how the screen fonts are made.
- [Release](release.md): how to build and publish a release.
- [Hardware reference](hardware/index.md): the MCU, pins, jacks, screen, audio, storage, battery, power, the decoder board, and the differences between boards.

## The repository

| Path | What it holds |
| - | - |
| `firmware/` | The `pocketcw-firmware` crate: drivers, the UI, and the RTIC app. It builds only for the device. |
| `logic/` | The `pocketcw-logic` crate: everything that does not touch hardware. It builds and tests on your computer. |
| `tools/` | Build, install and release scripts, and the font generator. |
| `docs/` | This documentation. |

PocketCW is licensed under the GPL-3.0-or-later. The fonts come from B612 Mono, under the SIL Open Font License 1.1.
