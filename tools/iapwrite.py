#!/usr/bin/env python3
"""Write one update file to the device's update drive, sector by sector.

usage: sudo python3 iapwrite.py /dev/diskN FILE

The bootloader routes file data by the name in the root directory entry, so
the entry must reach the device before the data, as Windows writes it:
directory entry, data, FAT, directory entry again. macOS and mtools write the
data first. Keep the drive unmounted (nomount) so macOS writes nothing else.
"""
import os
import re
import struct
import sys
import time

SECTOR = 2048
# Success shows for about 1 s, so poll well inside that.
POLL_S = 0.4
NAME_RE = re.compile(r'^([AF][0-9A-F]{6})\.BIN$')
# MCU flash below 0x14 KB holds the vendor bootloader, the only recovery path without a probe.
APP_FIRST_KB = 0x14
FLASH_KB = 256


def geometry(boot):
    bps, spc, res, nfats, root_entries = struct.unpack_from('<HBHBH', boot, 11)
    fat_sectors = struct.unpack_from('<H', boot, 22)[0]
    if bps != SECTOR or boot[54:62] != b'FAT16   ':
        sys.exit(f'error: unexpected boot sector ({bps} B sectors, {boot[54:62]!r})')
    root = (res + nfats * fat_sectors) * SECTOR
    data = root + root_entries * 32
    fats = [(res + i * fat_sectors) * SECTOR for i in range(nfats)]
    return spc * SECTOR, fats, root, data


def status_name(root_sector):
    return root_sector[0:11].decode('ascii', 'replace')


def main():
    if len(sys.argv) != 3:
        sys.exit(__doc__)
    disk, path = sys.argv[1], sys.argv[2]
    raw = re.sub(r'^/dev/disk', '/dev/rdisk', disk)
    match = NAME_RE.match(os.path.basename(path).upper())
    if not match:
        sys.exit(f'error: {path} is not an update file name (A000014.bin, F000000.bin, ...)')
    payload = open(path, 'rb').read()
    if not payload:
        sys.exit(f'error: {path} is empty')
    if match.group(1).startswith('A'):
        offset_kb = int(match.group(1)[1:], 16)
        if offset_kb < APP_FIRST_KB:
            sys.exit(f'error: {path} would overwrite the bootloader; MCU files start at A{APP_FIRST_KB:06X}')
        if offset_kb * 1024 + len(payload) > FLASH_KB * 1024:
            sys.exit(f'error: {path} ({len(payload)} bytes) runs past the end of MCU flash')

    fd = os.open(raw, os.O_RDWR)
    cluster_bytes, fats, root, data = geometry(os.pread(fd, SECTOR, 0))
    root_sector = bytearray(os.pread(fd, SECTOR, root))
    status = status_name(root_sector)
    if status.upper() != 'READY   TXT':
        sys.exit(f'error: drive status is {status!r}, not Ready; hold F4 to reset the drive')
    # The bootloader scans for the file from entry 2, after the status file and volume label.
    if any(root_sector[64:]):
        sys.exit('error: root directory has entries after the volume label; replug the device')

    entry = bytearray(32)
    entry[0:11] = (match.group(1).ljust(8) + 'BIN').encode('ascii')
    entry[11] = 0x20
    struct.pack_into('<HI', entry, 26, 2, len(payload))
    root_sector[64:96] = entry

    padded = payload + b'\xFF' * (-len(payload) % SECTOR)
    clusters = -(-len(padded) // cluster_bytes)
    fat = bytearray(SECTOR)
    fat[0:4] = b'\xF8\xFF\xFF\xFF'
    for n in range(2, 2 + clusters):
        struct.pack_into('<H', fat, n * 2, n + 1 if n < 1 + clusters else 0xFFFF)
    if 2 * (2 + clusters) > SECTOR:
        sys.exit('error: file needs more than one FAT sector')

    print(f'status Ready, writing {match.group(0)} ({len(payload)} bytes) to {raw}')
    os.pwrite(fd, bytes(root_sector), root)
    for off in range(0, len(padded), SECTOR):
        os.pwrite(fd, padded[off:off + SECTOR], data + off)
    for fat_offset in fats:
        os.pwrite(fd, bytes(fat), fat_offset)
    os.pwrite(fd, bytes(root_sector), root)
    os.fsync(fd)

    # When done, the bootloader renames the status file to Success, then drops off USB for
    # about 2 s and comes back Ready. The drop only follows a finished write.
    for _ in range(round(10 / POLL_S)):
        time.sleep(POLL_S)
        try:
            status = status_name(os.pread(fd, SECTOR, root))
        except OSError as err:
            print(f'device dropped off USB ({err.strerror}), waiting for it to return')
            os.close(fd)
            print(f'status now {read_status_after_reconnect(raw, root)!r}')
            print('done: the device finished the update')
            return
        if status.upper().startswith('SUCCESS'):
            print(f'status now {status!r}')
            print('done: the device finished the update')
            return
        if status.upper() != 'READY   TXT':
            sys.exit(f'error: status now {status!r}, not Success')
    sys.exit('error: status still Ready after 10 s; the device did not take the file')


def read_status_after_reconnect(raw, root):
    for _ in range(round(15 / POLL_S)):
        time.sleep(POLL_S)
        try:
            fd = os.open(raw, os.O_RDONLY)
        except FileNotFoundError:
            continue
        try:
            return status_name(os.pread(fd, SECTOR, root))
        finally:
            os.close(fd)
    sys.exit(f'error: {raw} did not return within 15 s; it may have a new disk number (see nomount output)')


if __name__ == '__main__':
    main()
