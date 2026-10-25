#!/usr/bin/env python3
"""Writes a test F000400.bin for the SPI flash probe: CLUSTERS 2 KB clusters, every 4 KB sector
tagged with PCWTEST and its SPI flash address, the rest 0xFF. After installing it, the probe
shows which of its markers past the file's end the bootloader erased.

    python tools/flashtest.py --out firmware/target/flashtest
    python tools/flashtest.py --clusters 1023 --out firmware/target/flashtest-1023

1022 clusters is the most `pocketcw-flash` writes; it refuses the 1023-cluster file.
"""

import argparse
import struct
from pathlib import Path

FILE_ADDRESS = 0x100000
CLUSTER = 2048
SECTOR = 4096
TAG = b"PCWTEST\0"


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--clusters", type=int, default=1022)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    data = bytearray(b"\xff" * (args.clusters * CLUSTER))
    for offset in range(0, len(data), SECTOR):
        tag = TAG + struct.pack("<I", FILE_ADDRESS + offset)
        data[offset : offset + len(tag)] = tag
    args.out.mkdir(parents=True, exist_ok=True)
    path = args.out / "F000400.bin"
    path.write_bytes(data)
    end = FILE_ADDRESS + len(data)
    print(f"{path}: {len(data)} bytes, SPI flash {FILE_ADDRESS:06X}-{end - 1:06X}")


if __name__ == "__main__":
    main()
