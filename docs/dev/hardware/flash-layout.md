# Flash layout and bootloader

## Layout

| Region | Contents |
| - | - |
| MCU 0x08000000 to 0x08004FFF | The vendor bootloader. Never write here. |
| MCU 0x08005000 up | The app. The app must set VTOR to 0x08005000. |
| SPI flash 0x000000 | Vendor boot screen, file `F000000.bin`. |
| SPI flash 0x030000 | Vendor UI images, file `F0000C0.bin`. |
| SPI flash 0x100000 | PocketCW file `F000400.bin`, 4 KB. |
| SPI flash 0xE00000 | Vendor fonts, file `F003800.bin`. |

PocketCW runs from the MCU app area. It never reads the SPI flash.

`F000400.bin` is `PocketCW`, a format version byte (1), then 0xFF to 4 KB. `tools/release.sh` writes it. The vendor files leave 0x0C5080 to 0xDFFFFF empty, and the bootloader takes a file at this offset.

`firmware/memory.x` gives the app 236 KB from 0x08005000, and `cortex-m-rt` sets VTOR from its `set-vtor` feature.

## Update files

An update file name is a hex offset in KB, after a letter: `A` for MCU flash, `F` for SPI flash. `A000014.bin` is MCU flash plus 0x14 KB, which is 0x08005000.

## The bootloader

The bootloader is a modified Artery `virtual_msc_iap` demo. It shows a USB drive: "HAM MODULE" in the vendor manual, "POKEMON-2" on board 2.64.

To start it: hold F1 and F4, then hold the dial until the update screen shows. Plug in USB after that.

The bootloader routes file data by the name in the root directory entry. So the directory entry must reach the device before the data, as Windows writes it. macOS and mtools write the data first, and the data goes to the wrong place.

On macOS, PocketCW installs with `tools/pocketcw-flash.c`. The tool blocks the mount of the update drive, so macOS writes nothing to it. Then it writes the raw disk in the order the bootloader needs: directory entry, data, FAT, directory entry again.

After a finished write, the status file reads `Success` for about 1 second. Then the drive drops off USB and comes back as `Ready`.

A failed write leaves the bootloader in update mode at every power on, until a write finishes. The vendor upgrade notes say so, and a 2.64 unit did it. Likely cause on that unit: macOS mounted the drive when it came back `Ready` after `Success`, and wrote its own files to it. So the tool counts a write as done only when it reads `Success`, and it blocks the mount until the drive is back.

On first installs, every user who installed only `A000014.bin` saw the device stay in update mode. Installing an `F` file first, then `A000014.bin`, fixed it. The vendor V5.03 notes give the same fix for a failed upgrade: `F000000.bin`, then `A000014.bin`. The cause is not known; no dump of the bootloader exists. So a first install writes `F000400.bin` before `A000014.bin`, and the stock install always writes `F0000C0.bin` before `A000014.bin`.

## Write protection

The vendor app has a one-time activation routine at 0x08018014, keyed to the chip UID. It writes these option bytes: Data0 and Data1 = `F2 60`, WRP0 = 0xE0, EOPB0 = 0xFE, Data3 and Data4 = `D`, `y`.

WRP0 = 0xE0 write-protects 0x08000000 to 0x08004FFF exactly. So on an activated unit, a flash write cannot overwrite the bootloader.

At every boot, the vendor app checks Data3 and Data4. If they do not hold `D`, `y`, the app overwrites itself with random data. PocketCW does not need these bytes. They matter only when the vendor firmware must run after an option byte erase.
