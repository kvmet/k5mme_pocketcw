#!/bin/sh
# Builds a firmware binary (default: the app; `probe` for the hardware probe) and writes
# firmware/target/A000014.bin. Refuses an image that would overwrite the bootloader.
set -eu
bin=${1:-pocketcw-firmware}
cd "$(dirname "$0")/../firmware"
cargo build --release --bin "$bin"
llvm=$(dirname "$(ls ~/.rustup/toolchains/*/lib/rustlib/*/bin/llvm-objcopy | head -1)")
elf=target/thumbv7em-none-eabihf/release/$bin
lowest=$("$llvm/llvm-objdump" -p "$elf" | awk '/LOAD/ {print $5}' | grep -i '^0x08' | sort | head -1)
if [ "$lowest" != 0x08005000 ]; then
    echo "image starts at $lowest, not 0x08005000" >&2
    exit 1
fi
"$llvm/llvm-objcopy" -O binary "$elf" target/A000014.bin
size=$(wc -c < target/A000014.bin)
if [ "$size" -gt $((236 * 1024)) ]; then
    echo "image is $size bytes, larger than the 236 KB app area" >&2
    exit 1
fi
echo "target/A000014.bin: $bin, $size bytes"
tools=$(cd ../tools && pwd)
echo "flash: sh $tools/flash.sh $(pwd)/target/A000014.bin"
