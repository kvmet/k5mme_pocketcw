# Release

A release is a git tag `vX.Y.Z` and a GitHub release with these files:

| File | Contents |
| - | - |
| `A000014.bin` | The firmware image. |
| `F000400.bin` | The PocketCW SPI flash file, installed before the firmware on a first install. See [Flash layout](hardware/flash-layout.md). |
| `pocketcw-flash` | The macOS install tool, for Apple and Intel Macs. |
| `LICENSE` | The PocketCW license. |
| `B612-OFL.txt` | The license of the B612 Mono font. |
| `THIRD-PARTY.txt` | The MIT notice of every crate linked into the firmware. |

The install script `docs/install.sh` downloads `A000014.bin`, `F000400.bin` and `pocketcw-flash` from the newest release. The docs site serves the script, so a change to it goes live when it reaches `main`.

## The version number

The version comes from `version` in `firmware/Cargo.toml`. The firmware reads it at build time, and shows it on the SYSTEM screen in the FIRMWARE row and on the boot screen.

Both crates have the same version. `firmware/Cargo.toml` also names the version of its `pocketcw-logic` dependency.

## Steps

Run these on a Mac. `release.sh` needs `clang` with the macOS frameworks.

1. Set the new version in three places: `version` in `firmware/Cargo.toml`, `version` in `logic/Cargo.toml`, and the `pocketcw-logic` dependency in `firmware/Cargo.toml`.
2. Build once, so the `Cargo.lock` files take the new version. Commit everything.
3. Build the release:

   ```bash
   sh tools/release.sh vX.Y.Z
   ```

4. Tag the commit and push the tag:

   ```bash
   git tag vX.Y.Z
   git push origin vX.Y.Z
   ```

5. Make a GitHub release for the tag, and upload every file in `dist/`. With the GitHub CLI:

   ```bash
   gh release create vX.Y.Z dist/*
   ```

6. Push `main` last. The docs site serves `install.sh` from `main`, and the script downloads files from the newest release. So a script that needs a new release file must not go live before that release.

## What release.sh does

1. Stops with an error when `firmware/Cargo.toml` does not hold the version you gave, or when the working tree has changes that are not committed. Then the files always match the tag.
2. Runs `tools/mkimage.sh` and copies the image to `dist/A000014.bin`.
3. Writes `dist/F000400.bin`.
4. Copies `LICENSE` and `firmware/src/B612-OFL.txt` to `dist/`.
5. Builds `dist/pocketcw-flash` for Apple and Intel Macs, macOS 10.15 or later.
6. Writes `dist/THIRD-PARTY.txt` with `tools/notices.py`.
7. Lists `dist/` and prints the tag and upload steps.

Git ignores `dist/` and `tools/pocketcw-flash`.
