# Storage

## EEPROM

A 24C16 EEPROM sits on I2C2 at 0x50: 2 KB, as eight 256 byte blocks at addresses 0x50 to 0x57, with 16 byte pages.

The vendor firmware uses 0x000 to 0x367. PocketCW keeps its records above that (`firmware/src/store.rs`):

| Address | Record | Magic |
| - | - | - |
| 0x400 | Letter statistics and Koch level | `Rl` |
| 0x480 | Practice history: 32 slots of 12 bytes | none |
| 0x600 | Memories | `Rm` |
| 0x700 | Settings, copy 0 | `Rw` |
| 0x740 | Settings, copy 1 | `Rw` |
| 0x780 | Battery full point | `Rb` |
| 0x7A0 | Boot count | `Rc` |
| 0x7B0 | Panel choice | `Rp` |
| 0x7C0 up | Free | |

Each record owns the bytes up to the next address, and startup reads all of them. So a longer record from later firmware still reads whole. Asserts in `store.rs` stop the build when a record grows into the next one.

### Record frame

`logic/src/record.rs` defines the frame:

| Bytes | Field |
| - | - |
| 2 | Magic |
| 1 | Version |
| 1 | Body length |
| n | Body |
| 2 | CRC-16/CCITT-FALSE of everything before it, high byte first |

A reader takes the fields it knows from the front of the body. So later firmware can add fields at the end, and this firmware still reads the record. The version changes only when the body no longer starts the same way.

- History slots are not frames. Each 12 byte slot ends in an 8 bit checksum.
- Settings have two copies, saved in turn. Each copy ends its body with a save count, so a save cut off by power loss leaves the other copy to load.
- The boot count record fits in one EEPROM page, so the write at every startup cannot tear it.
- Records from before frames still read. Their magics are in capitals, and they end in an 8 bit checksum. A board that goes back to v0.1.0 after this firmware wrote its records sees blank records and uses defaults.

### Read failures

When a read fails at startup, PocketCW starts with defaults for that record and refuses every write until the next power on. A write could otherwise replace a good record with defaults. The EEPROM row of the SYSTEM screen then shows FAIL.

## SPI flash

A GigaDevice GD25Q128 (JEDEC ID `C8 40 18`), 16 MB, on SPI1 with the screen: `FLASH_CS` PB9, `LCD_CS` PB6, MISO PB4. Confirmed on 2.64 with `firmware/src/bin/flashprobe.rs`. See [Flash layout and bootloader](flash-layout.md) for what lives where.

- Status registers 1 to 3 read `00 00 40`. No block protection: BP 0, CMP 0, WPS 0.
- SFDP 1.0, with a 9 dword basic table. It lists 4 KB (0x20), 32 KB (0x52) and 64 KB (0xD8) erase, and no erase times or page size. Pages are 256 bytes.
- Reads match at 18 MHz and at 4.5 MHz.
- Erasing a 4 KB sector takes 15 ms when it is blank and 45 ms after it was programmed.
- Programming a 256 byte page takes at most 917 µs, with the transfer at 18 MHz.
- One page programmed in 16 pieces of 16 bytes reads back whole. So small records can go into a page one at a time.

`firmware/src/spi_flash.rs` is the driver: read 0x03, page program 0x02, 4 KB erase 0x20, JEDEC ID 0x9F, SFDP 0x5A, status 0x05, 0x35 and 0x15. The LCD changes the SPI1 divider and PB4, so every flash command sets both. The app reads the content pack (`firmware/src/pack.rs`) and writes nothing.

## No clock

The board has no real-time clock. The date and time are lost at power off.
