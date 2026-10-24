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

- To flash from macOS: Python 3, and `clang` from the Xcode command line tools.

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

1. Build `nomount` once. `flash.sh` looks for it next to itself:

   ```bash
   clang -O2 -arch arm64 -arch x86_64 -mmacosx-version-min=10.15 -o tools/nomount tools/nomount.c -framework DiskArbitration -framework CoreFoundation
   ```

2. Run the command that `mkimage.sh` printed:

   ```bash
   sh tools/flash.sh firmware/target/A000014.bin
   ```

3. Follow the steps the script prints. It asks for your Mac login password, because `iapwrite.py` writes to the raw disk with `sudo`.

`iapwrite.py` refuses a file that would write below 0x08005000, or past the end of the 256 KB flash. The [install guide](../user/install.md) and [troubleshooting](../user/troubleshooting.md) pages cover the steps on the device.

## Test

The tests run on your computer:

```bash
cd logic
cargo test
```

`logic/` has no target setting, so it builds for your computer. The firmware crate has no tests.

## Hardware probe

`firmware/src/bin/probe.rs` is a separate image for checking a board. It shows the chip and board IDs, scans the I2C bus, shows the live inputs, and tests the audio codec, the PA8 audio output and standby.

```bash
sh tools/mkimage.sh probe
```

The probe image also goes to `firmware/target/A000014.bin`. Flash it the same way, and flash the app again after.
