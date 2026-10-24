# Flash layout and bootloader

## Layout

| Region | Contents |
| - | - |
| MCU 0x08000000 to 0x08004FFF | The vendor bootloader. Never write here. |
| MCU 0x08005000 up | The app. The app must set VTOR to 0x08005000. |
| SPI flash 0x000000 | Vendor boot screen, file `F000000.bin`. |
| SPI flash 0x030000 | Vendor UI images, file `F0000C0.bin`. |
| SPI flash 0xE00000 | Vendor fonts, file `F003800.bin`. |

PocketCW uses only the MCU app area. It never reads the SPI flash.

`firmware/memory.x` gives the app 236 KB from 0x08005000, and `cortex-m-rt` sets VTOR from its `set-vtor` feature.

## Update files

An update file name is a hex offset in KB, after a letter: `A` for MCU flash, `F` for SPI flash. `A000014.bin` is MCU flash plus 0x14 KB, which is 0x08005000.

## The bootloader

The bootloader is a modified Artery `virtual_msc_iap` demo. It shows a USB drive: "HAM MODULE" in the vendor manual, "POKEMON-2" on board 2.64.

To start it: hold F1 and F4, then hold the dial until the update screen shows. Plug in USB after that.

The bootloader routes file data by the name in the root directory entry. So the directory entry must reach the device before the data, as Windows writes it. macOS and mtools write the data first, and the data goes to the wrong place.

On macOS, PocketCW installs with two tools that `tools/flash.sh` runs:

- `tools/nomount.c` blocks the mount of the update drive, so macOS writes nothing to it.
- `tools/iapwrite.py` writes the raw disk in the order the bootloader needs: directory entry, data, FAT, directory entry again.

After a finished write, the status file reads `Success` for about 1 second. Then the drive drops off USB and comes back as `Ready`.

## Write protection

The vendor app has a one-time activation routine at 0x08018014, keyed to the chip UID. It writes these option bytes: Data0 and Data1 = `F2 60`, WRP0 = 0xE0, EOPB0 = 0xFE, Data3 and Data4 = `D`, `y`.

WRP0 = 0xE0 write-protects 0x08000000 to 0x08004FFF exactly. So on an activated unit, a flash write cannot overwrite the bootloader.

At every boot, the vendor app checks Data3 and Data4. If they do not hold `D`, `y`, the app overwrites itself with random data. PocketCW does not need these bytes. They matter only when the vendor firmware must run after an option byte erase.
