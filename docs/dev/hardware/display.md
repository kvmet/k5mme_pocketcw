# Display

The LCD is 320 x 240 in landscape, RGB565 (COLMOD 0x55). It is on SPI1, which it shares with the SPI flash. See [Pins](pins.md).

## Two panel types

| Panel | Boards | MADCTL | Vendor init |
| - | - | - | - |
| ILI9341 compatible | Older boards | 0xE8 | 0x080145E4 in V5.03, 0x080134F4 in V5.03.13 |
| ST7789 style IPS | Board 2.63 and later, 2.64 too | 0xA0 | 0x080177E6 in V5.03.13 |

V5.03.13 sends the IPS init only when Data1 == 0x63, with a `beq` at 0x08014F80. So under V5.03.13, board 2.64 gets the ILI9341 init.

With the wrong init, the image turns 90 degrees and wraps into interlaced strips. The ILI9341 init sends C0h 0x23 (power control 1). On the ST7789, C0h is LCMCTRL, and 0x23 sets its XMV bit, which swaps the row and column address order. That gives the rotation and the strips.

`firmware/src/lcd.rs` holds both init sequences, from V5.03.13.

## Bus setup

- The vendor sets LCD CS, LCD DC, flash CS and LCD reset to large drive, and SPI SCK and MOSI to maximum drive. PocketCW does the same.
- The vendor raises CS after every command and parameter byte. PocketCW does the same, but sends pixel data in one burst.
- SPI1 runs at 18 MHz for both panels: APB2 / 4 in the vendor (HAL init at 0x08015ED8), APB2 / 2 in PocketCW.

## How PocketCW picks the panel

Data1 does not reliably name the panel. The vendor activation routine writes `F2 60` on every board, so an activated IPS board can read 2.60 and get the ILI9341 init, under the vendor firmware and under PocketCW.

PocketCW picks the panel in this order (`logic/src/panel.rs`):

1. A choice saved with F2 at power on.
2. The panel's ID.
3. Data1: IPS from 0x63 up, except 0xFF (an erased byte), which gets the ILI9341, as in the vendor firmware.

### ID read

After the reset pulse, PocketCW bit-bangs RDDID (0x04) and reads 32 bits on both PB4 and PB5. A panel answers on its SDO pin if one is wired, and on its bidirectional SDA pin otherwise. Each read runs twice, once with the pins pulled up and once pulled down. A pin whose 24 ID bits differ between the two reads is not driven. The dummy bit and the 7 bits after the ID can float, so they are left out of the check.

The ST7789V answer 85 85 52, right after the one dummy clock that both datasheets document, picks IPS. An ILI9341 answers 00 80 00, or a module maker's ID with ID2 at 0x80 or more, so it cannot match. No ILI9341 ID is matched, because its D3h read is not documented for the serial interface.

The SYSTEM screen shows the raw 32 bits read pulled up for each pin, or FLOAT. On the 2.64 Mini, B4 reads `42C2A97F`: a dummy 0, the ID 85 85 52, and 7 floating ones. B5 is FLOAT.

### F2 override

Hold F2 while the board powers on, and PocketCW uses the other panel type. It saves the choice in its own EEPROM record at 0x7B0. Hold F2 at power on again to go back. A choice that matches the automatic one is saved as no choice.

The BOARD row of the SYSTEM screen shows where the choice came from:

| Label | Source |
| - | - |
| ID | The panel's ID. |
| AUTO | Data1. |
| SET | The saved choice. |
| NOSAVE | F2 was held, but the EEPROM write failed. The flip still applies until power off. |

## Backlight

PB8 drives the backlight from TMR4 channel 3, PWM at 20 kHz. PocketCW holds it off until the first screen clear, because the panel shows random pixels before that.
