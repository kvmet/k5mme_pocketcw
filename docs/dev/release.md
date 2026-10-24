# Release

A release is a git tag `vX.Y.Z` and a GitHub release with two files:

| File | Contents |
| - | - |
| `A000014.bin` | The firmware image. |
| `pocketcw-vX.Y.Z-mac.zip` | The macOS install bundle. It unzips to a folder `pocketcw-vX.Y.Z`. |

The bundle holds `A000014.bin`, `flash.sh`, `iapwrite.py`, `nomount`, `README.txt`, `LICENSE`, `B612-OFL.txt` and `THIRD-PARTY.txt`.

## The version number

The version comes from `version` in `firmware/Cargo.toml`. The firmware reads it at build time, and shows it on the SYSTEM screen in the FIRMWARE row and on the boot screen.

Both crates have the same version. `firmware/Cargo.toml` also names the version of its `pocketcw-logic` dependency.

## Steps

Run these on a Mac. `release.sh` needs `clang` with the macOS frameworks, and `ditto`.

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

5. Make a GitHub release for the tag, and upload `dist/A000014.bin` and `dist/pocketcw-vX.Y.Z-mac.zip`. With the GitHub CLI:

   ```bash
   gh release create vX.Y.Z dist/A000014.bin dist/pocketcw-vX.Y.Z-mac.zip
   ```

## What release.sh does

1. Stops with an error when `firmware/Cargo.toml` does not hold the version you gave, or when the working tree has changes that are not committed. Then the files always match the tag.
2. Runs `tools/mkimage.sh` and copies the image to `dist/A000014.bin`.
3. Builds `nomount` for Apple and Intel Macs, macOS 10.15 or later.
4. Copies `flash.sh`, `iapwrite.py`, the image, `LICENSE` and `firmware/src/B612-OFL.txt` into the bundle folder.
5. Writes `THIRD-PARTY.txt` with `tools/notices.py`: the MIT notice of every crate linked into the firmware.
6. Writes `README.txt` from `tools/install-readme.txt`, with the version in place of `@VERSION@`.
7. Zips the folder with `ditto`, which keeps the execute bits and leaves out macOS metadata files.
8. Lists the zip and prints the tag and upload steps.

Git ignores `dist/` and `tools/nomount`.
