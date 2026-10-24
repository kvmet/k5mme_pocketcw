# Board variants

The board version is in option bytes Data0 and Data1. The vendor System Information screen shows it as `(Data0 - 0xF0).(Data1 in hex)`: `F2 60` is 2.60. The 2.64 unit reads Data1 = 0x64.

PocketCW was tested on board 2.64 only.

## What the vendor firmware reads

Vendor V5.03.13 runs on every board from 2.60. It reads the board version in three places only:

| What | Where | Rule |
| - | - | - |
| LCD init | 0x08014F7A reads Data1 | IPS init (0x080177E6) when Data1 == 0x63, ILI9341 init (0x080134F4) otherwise. |
| Key output | 0x08009F86, 0x0800A5E6 and 0x08013EE8 compare Data0 and Data1 with `F2 60` | PA3, the func jack ring, is driven only on 2.60. PA2 and the PB2 LED are driven on every board. |
| System Information | 0x0800C300 copies Data0 and Data1 to RAM 0x2000095C | Display only. |

V5.03.13 knows only the WM8978 codec. So a board with an ES8388, such as 2.64, is silent under it. A 2.64 unit shipped with unpublished V5.03.15 had working sound, so V5.03.15 sets up the ES8388. No dump of V5.03.15 exists yet.

Data1 does not reliably name the panel. The activation routine writes `F2 60` on every board, so an activated IPS board can read 2.60.

## What PocketCW does

| What | PocketCW |
| - | - |
| Panel | Asks the panel for its ID first, then uses Data1. F2 at power on overrides both. See [Display](display.md). |
| Codec | Uses the codec that answers on I2C: 0x10 for the ES8388, 0x1A for the WM8978. See [Audio](audio.md). |
| Func jack | Drives PA2 and PA3 on every board. See [Key and func jacks](jacks.md). |
| System screen | Shows the board version, the panel type, and where the panel choice came from. |

## Known differences

| Part | Older boards | Board 2.63 | Board 2.64 |
| - | - | - | - |
| Panel | ILI9341 compatible | ST7789 style IPS | ST7789 style IPS |
| Codec | WM8978 at 0x1A | Not known | ES8388 at 0x10 |
