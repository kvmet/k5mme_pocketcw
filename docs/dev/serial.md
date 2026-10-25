# Serial interface

PocketCW has a USB serial port that a computer uses to press the buttons, key text, read what the device decodes and plays, and read the screen. [Remote control and screenshots](building.md#remote-control-and-screenshots) covers the harness that uses it.

## The port

The device is a composite USB device: a keyboard and a CDC-ACM serial port, with Artery's composite example ID, 2E3C:5750. It is on USB while USB OUT is anything but OFF. SERIAL gives the port alone, with no typing. With USB OUT set to OFF, the device detaches and the port goes away.

The port shows up as `/dev/cu.usbmodem*` on macOS and `/dev/ttyACM*` on Linux. The device ignores the baud rate and the other line settings.

The port has two modes. It starts in text mode, which the sections below up to [WinKeyer](#winkeyer) describe. A 0x00 byte switches it to [WinKeyer](#winkeyer) mode.

## Commands

Each command is one line of up to 53 bytes, ended by `\n` or `\r`. Commands are upper case. The device ignores a line it does not know and a longer line, with no reply.

| Command | Device |
| - | - |
| `PRESS F1` to `PRESS F4` | Presses and releases that soft key. No reply. |
| `PRESS LEFT`, `PRESS RIGHT` | Turns the dial one step. No reply. |
| `PRESS CLICK` | Presses and releases the dial. No reply. |
| `SEND TEXT` | Keys up to 48 characters of text. See [Send](#send). |
| `SHOT` | Sends the screen. See [Screenshot](#screenshot). |

Presses, `SEND` and `SHOT` go to the screen through one queue, in the order they came, so a screenshot after a press shows the result of the press. A press skips the debounce, so it cannot hold a button: `PRESS CLICK` does not power off. More than about 30 presses sent at once can fill the queue; the device then shows `n INPUTS LOST`.

## Send

`SEND` keys its text as a straight key with perfect timing, at the keyer speed. Lower case counts as upper case, and a prosign is written in angle brackets, such as `<AR>`. A character with no Morse code is skipped.

The keying takes the same path as the paddle: the decoder, the screen, the sidetone and the LED. Every screen sees it as your keying, so it answers drills and works QSOs. KEY OUT keys the radio when it is ON and OUT TYPE is STRAIGHT. Touching the key stops the text.

One word gap after the text ends, the device sends `SENT`. By then the decoder has sent the last character and the word gap as `KEY` lines. A `SEND` before the `SENT` of the one before gets `BUSY` and is not keyed.

## Events

While the computer has the port open (holds DTR), the device sends a line for each of these:

| Line | When |
| - | - |
| `KEY C` | The decoder read character `C` from your keying. A word gap is `KEY` and one space; a pattern with no character is `KEY *`; a prosign is `KEY <AR>`. |
| `HEARD C` | The decoder board's tone line gave character `C`, written as for `KEY`. |
| `PLAY TEXT` | The device started to sound `TEXT`: a drill group, a station's line, or a memory. |
| `PLAYED` | Everything the device was sounding has finished. |
| `SENT` | The text of a `SEND` has finished. |
| `BUSY` | A `SEND` came before the last one finished. |

`PLAY` gives a drill group before you hear it, so a script can answer it. Event lines never break into a screenshot: the screen sends them in turn with its other work. An event line that finds the serial queue full is lost, and the screen shows `n SERIAL LINES LOST`.

## Screenshot

The device answers `SHOT` with a header line, then the screen:

```
SHOT 320 240 RGB666\n
```

After the header come 240 rows, top to bottom, each 320 pixels left to right: 230400 bytes. Each pixel is 3 bytes, red, green, blue, with the channel in the top 6 bits of its byte. To make an 8-bit channel from byte `b`, copy the top bits down: `b & 0xFC | b >> 6`.

The bytes are the panel's memory, read back over SPI with RAMRD. See [Memory read](hardware/display.md#memory-read). The screen stops for under a second while a screenshot sends.

| Answer or status | Cause |
| - | - |
| `SHOT UNCHECKED PANEL` and a line end, in place of the header | The board has the ILI9341 panel, whose memory read is not checked. No rows follow. |
| `SCREENSHOT NOT SENT` on the screen | USB took nothing for 0.5 s, so the device stopped sending. |

A screenshot that stops early can leave its bytes queued. The device drops them only when USB detaches, so closing the port keeps them. A reader looks for `SHOT ` to find the next header.

## WinKeyer

The device answers the K1EL WinKeyer host protocol as a WK2 with firmware 23, so logging programs can key through it. K1EL's WinKeyer 3 datasheet (rev 1.3) defines the protocol; this section lists what the device does with it.

No text command starts with 0x00, so a 0x00 byte starts the WinKeyer parser. Until host open, the parser takes only admin commands (`00 nn`). Host open (`00 02`) answers 23 and starts WinKeyer mode. Host close (`00 03`), reset (`00 01`), the computer dropping DTR, and USB detaching all end it: the queued text is dropped, the port goes back to text mode, and event lines start again. Text mode sends no event lines while WinKeyer mode is on.

Text bytes go to the same sender as `SEND`, which keys them in the same path. A queue holds 128 characters and buffered commands. Upper case letters, digits and the signs in `morse` key as themselves. The WinKeyer prosign signs key as merged pairs, such as `<` for AR and `=` for BT. `|` is half a dot of silence. Lower case and other bytes above `]` are ignored, as on a WinKeyer.

| Command | Device |
| - | - |
| `00 04 nn` echo test | Sends back `nn`. |
| `00 05`, `00 06`, `00 07`, `00 15`, `00 18` | Sends back 0. |
| `00 09` firmware major revision | Sends back 23. |
| `00 17` firmware minor revision | Sends back 0. |
| `02 nn` speed | Sets SPEED to `nn` WPM, clamped to 5 to 60, and saves it; 0 keeps it. |
| `05 min range x` speed pot setup | Sets the lowest speed of the pot reply. |
| `06 nn` pause | Holds the queue while `nn` is not 0. |
| `07` get speed pot | Sends back `0x80` plus SPEED less the pot's lowest speed, at most 31. |
| `08` backspace | Drops the newest queued item. |
| `0A` clear buffer | Stops and drops the queue. |
| `0B nn` key immediate | Holds the key down while `nn` is not 0. |
| `0E nn` WinKeyer mode | Bit 2 turns serial echo on, bit 6 paddle echo. The other bits are ignored. |
| `0F` load defaults (15 values) | Takes the mode, the speed and the pot's lowest speed. |
| `15` request status | Sends back the status byte. |
| `1B a b` merge letters | Keys `a` and `b` as one character. |
| `1C nn` buffered speed | Keys the queued items after it at `nn` WPM. |
| `1E` cancel buffered speed | Goes back to SPEED. |

The device reads the parameter bytes of every other command and ignores them, so the bytes after them keep their meaning. `00 0D` load EEPROM skips its 256 bytes; `00 0C` dump EEPROM sends nothing back.

### Bytes to the host

| Byte | When |
| - | - |
| Status, `0xC0` with flags | On request, and on each change. Bit 2 BUSY: the queue is keying or holds text. Bit 1 BREAKIN: the paddles keyed within a word gap. Bit 0 XOFF: the queue is over 2/3 full. |
| Serial echo | Each text byte, once its last mark ends, while serial echo is on. A prosign sign such as `<` echoes itself; a `1B` merge echoes nothing. |
| Paddle echo | Each character the decoder reads from the paddles, and a space for a word gap, while paddle echo is on. A prosign with a WinKeyer sign echoes the sign, such as `+` for AR; one with no sign echoes nothing. |

A byte that finds the queue to the host full is lost, and the screen shows `n WINKEYER BYTES LOST`. So is a text byte that finds the key queue full.

The text keys with perfect timing, at the WinKeyer speed. The paddles keep the device's MODE and PADDLES, and the sidetone keeps TONE and VOLUME. Touching the key clears the queue, as on a WinKeyer.

## Errors

`n SERIAL ERRORS` on the screen counts the serial reads and writes that USB refused. A refused write drops the rest of its 64-byte piece. A full USB buffer is not an error: the device tries again on the next poll.

## Code

| Part | File |
| - | - |
| Command lines and their parse | `logic/src/serial.rs` |
| The sender that keys `SEND` and WinKeyer text | `logic/src/sender.rs` |
| The WinKeyer parser | `logic/src/winkeyer.rs` |
| WinKeyer actions, and the bytes to the host | `winkeyer_act` and `wk_chunk` in `firmware/src/main.rs` |
| The USB device, keyboard and serial port | `firmware/src/usb.rs` |
| The USB task, which reads commands and writes the queued bytes | `usb_events` in `firmware/src/main.rs` |
| The serial key, merged with the keyer output | `tick` in `firmware/src/main.rs` |
| Event lines and `SEND` | `event_line`, `emit` and the `ui` task in `firmware/src/main.rs` |
| The screenshot, row by row, from the UI task | `send_screen` in `firmware/src/main.rs`, `read_row` in `firmware/src/lcd.rs` |
