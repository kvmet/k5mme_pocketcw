# Release

A release is a git tag `vX.Y.Z` and a GitHub release with these files:

| File | Contents |
| - | - |
| `A000014.bin` | The firmware image. |
| `F000400.bin` | The content pack: words and voice clips for the SPI flash, installed before the firmware on a first install or when the pack changed. See [Content pack](building.md#content-pack). |
| `pocketcw-flash` | The macOS install tool, for Apple and Intel Macs. |
| `LICENSE` | The PocketCW license. |
| `B612-OFL.txt` | The license of the B612 Mono font. |
| `THIRD-PARTY.txt` | The MIT notice of every crate linked into the firmware, then the content pack's credits from `tools/pack/pack-notice.txt`. |

The install script `docs/install.sh` downloads `A000014.bin`, `F000400.bin` and `pocketcw-flash` from the newest release. The docs site serves the script, so a change to it goes live when it reaches `main`.

## The version number

The version comes from `version` in `firmware/Cargo.toml`. The firmware reads it at build time, and shows it on the SYSTEM screen in the FIRMWARE row and on the boot screen.

Both crates have the same version. `firmware/Cargo.toml` also names the version of its `pocketcw-logic` dependency.

## Steps

Run these on a Mac. `release.sh` needs `clang` with the macOS frameworks, and what `tools/pack/pack.py` needs to build the content pack: a Python 3.12 with the voice tools, and the Kokoro model files. See [Content pack](building.md#content-pack). With a full clip cache in `firmware/target/clip-cache`, the pack builds in about a minute; with only the saved speech, in about 5 minutes; with none, in about 25 minutes. `cargo clean` deletes the cache.

1. Set the new version in three places: `version` in `firmware/Cargo.toml`, `version` in `logic/Cargo.toml`, and the `pocketcw-logic` dependency in `firmware/Cargo.toml`.
2. Build once, so the `Cargo.lock` files take the new version. Commit everything.
3. Build the release:

   ```bash
   VOICE_PYTHON=PYTHON VOICE_MODELS=MODELS sh tools/release.sh vX.Y.Z
   ```

   `PYTHON` is the Python with the voice tools, and `MODELS` the folder with `kokoro-v1.0.onnx` and `voices-v1.0.bin`.

4. Tag the commit and push the tag:

   ```bash
   git tag vX.Y.Z
   git push origin vX.Y.Z
   ```

5. Make a GitHub release for the tag, and upload every file in `dist/`. The release notes name the release that last changed the content pack, such as "Content pack: changed in v0.4.0". Users who skip releases need this: the install script and the install page tell users to install the pack when it changed since their last install. With the GitHub CLI:

   ```bash
   gh release create vX.Y.Z dist/*
   ```

6. Push `main` last. The docs site serves `install.sh` from `main`, and the script downloads files from the newest release. So a script that needs a new release file must not go live before that release.

## What release.sh does

1. Stops with an error when `VOICE_PYTHON` or `VOICE_MODELS` is not set, when `firmware/Cargo.toml` does not hold the version you gave, or when the working tree has changes that are not committed. Then the files always match the tag.
2. Runs `tools/mkimage.sh` and copies the image to `dist/A000014.bin`.
3. Copies `LICENSE` and `firmware/src/B612-OFL.txt` to `dist/`.
4. Builds the content pack into `dist/F000400.bin` with `tools/pack/pack.py`. It prints how many common words fit.
5. Builds `dist/pocketcw-flash` for Apple and Intel Macs, macOS 10.15 or later.
6. Writes `dist/THIRD-PARTY.txt` with `tools/notices.py` and `tools/pack/pack-notice.txt`.
7. Lists `dist/`, prints the content pack's SHA-256 for the release notes, and prints the tag and upload steps.

A pack built on another machine can differ: the clip cache key covers the Kokoro version, model and voice, and how clips are made, but not espeak-ng or FFmpeg. So the SHA-256 in the release notes records which pack shipped.

Git ignores `dist/` and `tools/pocketcw-flash`.
