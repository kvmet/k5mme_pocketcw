# Flash layout and bootloader

## Layout

| Region | Contents |
| - | - |
| MCU 0x08000000 to 0x08004FFF | The vendor bootloader. Never write here. |
| MCU 0x08005000 up | The app. The app must set VTOR to 0x08005000. |
| SPI flash 0x000000 to 0x025FFF | Vendor boot screen, file `F000000.bin`. |
| SPI flash 0x030000 to 0x0C5FFF | Vendor UI images, file `F0000C0.bin`. |
| SPI flash 0x100000 up | PocketCW content pack, file `F000400.bin`, up to 8,372,224 bytes. |
| SPI flash 0xE00000 to 0xFECFFF | Vendor fonts, file `F003800.bin`. |

The vendor ranges are the used 4 KB sectors in a scan of a 2.64 unit. Every other sector read blank: 0x026000 to 0x02FFFF, 0x0C6000 to 0xDFFFFF, and 0xFED000 to 0xFFFFFF.

PocketCW runs from the MCU app area and reads the content pack from the SPI flash.

`tools/pack/pack.py` builds `F000400.bin`; `logic/src/pack.rs` gives the format. `tools/release.sh` builds it into each release.

`firmware/memory.x` gives the app 236 KB from 0x08005000, and `cortex-m-rt` sets VTOR from its `set-vtor` feature.

## Update files

An update file name is a hex offset in KB, after a letter: `A` for MCU flash, `F` for SPI flash. `A000014.bin` is MCU flash plus 0x14 KB, which is 0x08005000.

## The bootloader

The bootloader is a modified Artery `virtual_msc_iap` demo. It shows a USB drive: "HAM MODULE" in the vendor manual, "POKEMON-2" on board 2.64.

To start it: hold F1 and F4, then hold the dial until the update screen shows. Plug in USB after that.

The bootloader routes file data by the name in the root directory entry. So the directory entry must reach the device before the data, as Windows writes it. macOS and mtools write the data first, and the data goes to the wrong place.

On macOS, PocketCW installs with `tools/pocketcw-flash.c`. The tool blocks the mount of the update drive, so macOS writes nothing to it. Then it writes the raw disk in the order the bootloader needs: directory entry, data, FAT, directory entry again.

The update drive is FAT16 with 2 KB sectors, 8 KB clusters and 51,200 sectors (100 MB). Confirmed on 2.64; `pocketcw-flash` prints the cluster size. `pocketcw-flash` writes the whole FAT in one sector, so a file holds at most 1022 clusters: 8,372,224 bytes.

An SPI flash install erases and programs the 4 KB sectors its file covers. Confirmed on 2.64 with a 2,093,056 byte file at 0x100000, the largest tested: every sector of it read back. A 5.97 MB content pack also installed and played to its last clip, and an 8.37 MB one installed and played its first and last clips. Tags written beforehand in the sector just past its end, 4 KB, 64 KB and 1 MB past it, and at 0xC00000 and 0xDFF000 all survived.

Drive writes that are not part of an update file can also land in the SPI flash. On 2.64, after a session with several installs, 0x101000 to 0x103FFF held a FAT directory cluster (first entry `.`, attribute 0x12) in place of the test file data. The sectors were erased before the write. Likely: macOS wrote to the drive while it was mounted, and the bootloader routed the write to the SPI flash beside the last F file. So data that PocketCW keeps in the SPI flash checks its own frames, and the install path never lets macOS mount the drive.

After a finished write, the status file reads `Success` for about 1 second. Then the drive drops off USB and comes back as `Ready`.

A failed write leaves the bootloader in update mode at every power on, until a write finishes. The vendor upgrade notes say so, and a 2.64 unit did it. Likely cause on that unit: macOS mounted the drive when it came back `Ready` after `Success`, and wrote its own files to it. So the tool counts a write as done only when it reads `Success`, and it blocks the mount until the drive is back.

On first installs, every user who installed only `A000014.bin` saw the device stay in update mode. Installing an `F` file first, then `A000014.bin`, fixed it. The vendor V5.03 notes give the same fix for a failed upgrade: `F000000.bin`, then `A000014.bin`. The cause is not known; no dump of the bootloader exists. So a first install writes `F000400.bin`, the content pack, before `A000014.bin`. The install script asks whether to write it, since an update needs it only when the pack changed. The stock install always writes `F0000C0.bin` before `A000014.bin`. `pocketcw-flash` writes both files in one update mode session, as a Windows copy does: after `Success` on the F file, it waits for the drive to come back `Ready`, then writes the A file. A retry starts again at the F file.

## Write protection

The vendor app has a one-time activation routine at 0x08018014, keyed to the chip UID. It writes these option bytes: Data0 and Data1 = `F2 60`, WRP0 = 0xE0, EOPB0 = 0xFE, Data3 and Data4 = `D`, `y`.

WRP0 = 0xE0 write-protects 0x08000000 to 0x08004FFF exactly. So on an activated unit, a flash write cannot overwrite the bootloader.

At every boot, the vendor app checks Data3 and Data4. If they do not hold `D`, `y`, the app overwrites itself with random data. PocketCW does not need these bytes. They matter only when the vendor firmware must run after an option byte erase.
