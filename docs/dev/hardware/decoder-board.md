# Decoder board

The Pro model adds a decoder board that listens to a radio's audio. The vendor V5.03 app image starts at 0x08005000. No Pro unit has been checked.

The host needs three things to use the board: power on PB13, the USART1 link, and the tone line on PB14.

## Power (PB13)

- Push-pull output (HAL_GPIO_Init at 0x0801560A and 0x08015618). Confirmed.
- Driven low at boot, with PB2, PB7, PB12 and PB15 (0x08015598). Confirmed.
- After the settings load, set from the "decoder board power" setting (0x08013244). The menu toggle near 0x08009F9E drives it high for ON and low for OFF. OFF also clears the board present flags. So high = board powered. Confirmed.

## Link (USART1)

- Setup at 0x08015858. 115200 baud, 8N1, TX and RX. PA9 TX is alternate function push-pull (0x080128C2), PA10 RX is an input (0x080128D2). Confirmed.
- RX goes by DMA1 channel 5 into a 64 byte buffer. The idle line interrupt (0x08018C44) ends a frame, so frames need no terminator. Confirmed.
- The frame parser is at 0x08005D00, called from 0x0801727C. Host frames end with `\r\n`. Confirmed.

## Board to host

Every frame starts with `RE+` (string at 0x08022867), then a two letter code. Confirmed.

| Frame | Host action |
| - | - |
| `RE+IF1x` | Marks the board present and replies `AT+OK\r\n`. `x` = `1` sets a second flag whose use is unknown. |
| `RE+MD0`, `RE+MD1` | Stores the digit. Use unknown. |
| `RE+FL0` to `RE+FL2` | Stores the filter setting. The vendor shows it as an icon. |
| `RE+NR0` to `RE+NR6` | Stores the noise reduction level. The vendor shows it as an icon. |
| `RE+MC`, `RE+AC`, `RE+MU`, `RE+VL` | Accepted, no action. |
| Any other `RE+` frame | Replies `AT+ERR\r\n`. |

PocketCW (`logic/src/board.rs`) follows the same rules. A known code with a digit out of range changes nothing. A frame without `RE+` gets no reply, as in the vendor parser. Confirmed in code.

## Host to board

Sent through 0x0801D480, which formats and transmits. Confirmed.

| Command | When |
| - | - |
| `AT+FL<0..2>` (0x0800BD7C) | F2 on the decoder screen cycles the filter. |
| `AT+NR<0..6>` (0x0800BD8C) | F3 on the decoder screen cycles noise reduction. |
| `AT+VL<000..063>` (0x080183BC, 0x0801B120) | The host volume changed and the board is present. The value has three digits. |

## Detection

The host never asks. The board sends `RE+IF1x`, and the host answers `AT+OK`. Confirmed in code. That the board sends it at power up is Likely.

## Decoding

The board sends no characters. It raises PB14 while it hears a tone, and the host times and decodes the marks itself. Setup at 0x08006410 sets PB14 as an input with pull-down. The 1 ms SysTick polls it (0x080179CC calls 0x0800A094; high goes to 0x0801D6D4, low to 0x0801D71C). Confirmed.

The board manual describes no audio path from the board into the host, and the host gets no audio from it, only PB14.

### Vendor decoding rules

At 0x0801D6D4 and 0x0801D71C. Times are in 1 ms samples. Start values: unit 51, word 154, 20 WPM.

- Up to 6 low samples inside a tone do not end it. A mark lasts the high run plus 6.
- A mark under 0.5 unit is not an element. It pulls the unit down by a quarter of the difference, and a unit under 24 resets to 51.
- A mark from 0.5 to under 1.5 units is a dot. A longer mark is a dash, and sets word = (word + mark) / 2, held to 77 to 365, WPM = 3850 / word, unit = word / 3.
- A gap over 2 units ends the character. A gap over 1.5 words gives one word space. A 9th element clears the pattern.

## Ports

From the decoder board user manual and the Quick Start Guide. Confirmed in the manuals.

| Port | Use |
| - | - |
| AUX1 | Radio audio in, or the external MIC module. The board filters this and detects tone in it. |
| AUX2 | Stereo headphone or powered speaker out. `AT+VL` sets its volume from the host volume. |
| AUX3 | Not used. |
| USB | The Pro's charging port. No vendor code refers to it. |

## PocketCW

All of this is confirmed in code and not on a board.

- The DECODER BOARD setting, on the RECV tab, defaults to OFF, so a Mini sees no change. ON powers the board, starts USART1, and adds RADIO to the menu.
- OFF stops USART1, sets PA9 as a pulled-down input so the board is not powered through it, and drives PB13 low. PA10 has a pull-up, so an idle line reads high with no board.
- Frames are parsed in the USART1 interrupt and get the vendor replies at once.
- The RADIO screen shows the board state, the filter and noise reduction (F2 and F3 send `AT+FL` and `AT+NR`), the heard speed, and the decoded text.
- A volume change sends `AT+VL` once the board has announced itself. The value is the WM8978 speaker register for the same volume, 0 to 63.
- PB14 is polled every 1 ms while the RADIO screen is open, and on every screen while USB OUT is RADIO.
- The baud rate comes from the crystal. The divider error is 0.16 %.
- Frames with framing, noise or overrun errors count as `ERR n` on the RADIO screen.

### PocketCW decoding

`logic/src/listen.rs` decodes in its own way. The vendor rules cannot follow below about 10 WPM, count the bridged silence into every mark, and let a noise blip pull the speed down.

- A dropout of 6 ms or less inside a mark is bridged, and the mark keeps its true length. A mark under 10 ms is noise.
- Speed: two adjacent marks 2 to 4.5 times apart are a dot and a dash. The first pair sets the speed. A sample more than 1.5 times off the estimate replaces it, and other samples average in at 1/8. The range is 5 to 60 WPM.
- The marks of a character are classified when the character ends, at the midpoint of the dot and dash estimates.
- A character ends after 2 dots of silence. A word ends after 5 dots, or after 5/3 of the learned letter gap when that is longer, so Farnsworth spacing decodes. The letter gap is learned from adjacent gaps, the same way as the speed.
- WPM = 1200 / dot length in ms.
- Tests key known text and require it back: 5 to 60 WPM, 20 % jitter, dash ratios 2.5 and 3.5, Farnsworth 20/8 and 18/10, speed changes up to 3 times, pauses, dropouts, blips and noise bursts. The vendor test inputs are kept, with the sent text as the expected output.

The same decoder reads your keying in STRAIGHT mode.
