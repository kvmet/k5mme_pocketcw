# Serial interface

PocketCW has a USB serial port that a computer uses to press the buttons, key text, read what the device decodes and plays, and read the screen. [Remote control and screenshots](building.md#remote-control-and-screenshots) covers the harness that uses it.

## The port

The device is a composite USB device: a keyboard and a CDC-ACM serial port, with Artery's composite example ID, 2E3C:5750. It is on USB only while USB OUT is TEXT, VBAND or RADIO. With USB OUT set to OFF, the device detaches and the port goes away.

The port shows up as `/dev/cu.usbmodem*` on macOS and `/dev/ttyACM*` on Linux. The device ignores the baud rate and the other line settings.

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

## Errors

`n SERIAL ERRORS` on the screen counts the serial reads and writes that USB refused. A refused write drops the rest of its 64-byte piece. A full USB buffer is not an error: the device tries again on the next poll.

## Code

| Part | File |
| - | - |
| Command lines and their parse | `logic/src/serial.rs` |
| The USB device, keyboard and serial port | `firmware/src/usb.rs` |
| The USB task, which reads commands and writes the queued bytes | `usb_events` in `firmware/src/main.rs` |
| The serial key, merged with the keyer output | `tick` in `firmware/src/main.rs` |
| Event lines and `SEND` | `event_line`, `emit` and the `ui` task in `firmware/src/main.rs` |
| The screenshot, row by row, from the UI task | `send_screen` in `firmware/src/main.rs`, `read_row` in `firmware/src/lcd.rs` |
