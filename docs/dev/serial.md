# Serial interface

PocketCW has a USB serial port that a computer uses to press the buttons, key and play text, read what the device decodes and plays, read and change the settings, read the stats and the key timing, and read the screen. [Remote control and screenshots](building.md#remote-control-and-screenshots) covers the harness that uses it.

## The port

The device is a composite USB device: a keyboard and a CDC-ACM serial port, with Artery's composite example ID, 2E3C:5750. It is on USB while USB OUT is anything but OFF. SERIAL gives the port alone, with no typing. With USB OUT set to OFF, the device detaches and the port goes away.

The port shows up as `/dev/cu.usbmodem*` on macOS and `/dev/ttyACM*` on Linux. The device ignores the baud rate and the other line settings.

The port has two modes. It starts in text mode, which the sections below up to [WinKeyer](#winkeyer) describe. A 0x00 byte switches it to [WinKeyer](#winkeyer) mode.

## Commands

Each command is one line of up to 55 bytes, ended by `\n` or `\r`. Commands are upper case, except the two that VBand sends; text and values may be lower case. The device skips empty lines, answers a line it does not know with `ERROR UNKNOWN`, and answers a longer line, or text over 48 characters, with `ERROR TOO LONG`.

| Command | Device |
| - | - |
| `PRESS F1` to `PRESS F4` | Presses and releases that soft key. No reply. |
| `PRESS LEFT`, `PRESS RIGHT` | Turns the dial one step. No reply. |
| `PRESS CLICK` | Presses and releases the dial. No reply. |
| `SEND TEXT` | Keys up to 48 characters of text. See [Send](#send). |
| `PLAY TEXT` | Sounds up to 48 characters of text. See [Play](#play). |
| `SAY TEXT` | Speaks up to 48 characters from the content pack: each word the pack has a clip for, and every other word, such as `QTH`, spelled a character at a time. One abbreviation on its own also gets its meaning: `SAY QTH` says "Q T H. location", and `SAY QTH IS` says "Q T H is". Answers `OK SAY TEXT`, or `ERROR SAY: NO PACK`, or `ERROR SAY: NOTHING TO SAY`. Speech stops at about 3.7 s or 32 clips. A screen change, or a drill's STOP, also stops it. Lower case counts as upper case; a prosign is written as `<BK>`. |
| `STOP` | Stops the text of `SEND` and `PLAY`, and anything else the device is sounding. No reply. |
| `SHOT` | Sends the screen. See [Screenshot](#screenshot). |
| `INFO` | Answers `INFO VERSION=0.2.0 SCREEN=KEYER PANEL=IPS PACK=3131 AUDIO=1900/4096`: the firmware version, the screen, the panel (`IPS` or `ILI9341`), the content pack's word count, or `NONE`, `OLD` for a pack of an earlier format, or `BAD` for a corrupt one, and the sample interrupt's longest run since the last `INFO` against its budget, in CPU cycles. |
| `GET NAME`, `GET` | Sends a setting or memory, or all of them. See [Settings](#settings). |
| `SET NAME VALUE` | Changes a setting or memory. See [Settings](#settings). |
| `EDGE ON`, `EDGE OFF` | Starts or stops the key timing lines. See [Edges](#edges). |
| `VBand On`, `VBand Off` | Starts or stops the CW Hotline lines, in this mixed case. See [VBand Hotline](#vband-hotline). No reply. |
| `STATS LETTERS`, `STATS PROGRESS`, `STATS TIMING` | Sends the letter stats, the practice sessions, or your keying timing. See [Stats](#stats). |

Every command goes to the screen through one queue, in the order it came, so a screenshot after a press shows the result of the press. A press skips the debounce, so it cannot hold a button: `PRESS CLICK` does not power off. More than about 30 commands sent at once can fill the queue; the device then shows `n INPUTS LOST`.

The screen waits for room in the serial queue for each reply line, for up to 0.5 s; a line that waits longer is lost, with the rest of its reply, and counts in `n SERIAL LINES LOST`. Event lines can come between the lines of a reply.

## Send

`SEND` keys its text as a straight key with perfect timing, at the keyer speed. Lower case counts as upper case, and a prosign is written in angle brackets, such as `<AR>`. A character with no Morse code is skipped.

The keying takes the same path as the paddle: the decoder, the screen, the sidetone and the LED. Every screen sees it as your keying, so it answers drills and works QSOs. KEY OUT keys the radio when it is ON and OUT TYPE is STRAIGHT. Touching the key stops the text.

One word gap after the text ends, the device sends `SENT`. By then the decoder has sent the last character and the word gap as `KEY` lines. A `SEND` before the `SENT` of the one before gets `BUSY` and is not keyed.

## Play

`PLAY` sounds its text as the receive drills do: at CHAR SPEED and FARNSWORTH, on TONE, with NOISE and FADING on every screen while it sounds. Lower case counts as upper case, and a prosign is written in angle brackets. It does not key the radio, and the decoder does not hear it. The `PLAY` event line comes when it starts, and `PLAYED` when everything the device is sounding has finished. A `PLAY` while a drill sounds plays beside it on a free voice.

## Settings

A name is a settings row with `_` for each space: `MODE`, `PADDLES`, `SPEED`, `KEY_OUT`, `OUT_TYPE`, `OUT_LINES`, `USB_OUT`, `TONE`, `VOLUME`, `WAVE`, `NOISE`, `FADING`, `CHAR_SPEED`, `FARNSWORTH`, `CHEW_SHOWS`, `DECODER_BOARD`, `PILEUP`, `FIST`, `SPEED_SPREAD`, `STYLE`, `PALETTE` and `BRIGHTNESS`. A value is what the row shows, without `WPM`, `HZ` or `%`: `IAMBIC B`, `ON`, `25`, `MATCH`.

The memories have their own names:

| Name | Value |
| - | - |
| `CALL` | Your callsign: up to 10 of A to Z, 0 to 9 and `/`. |
| `M1` to `M4` | A memory's text, up to 48 characters, with a prosign written as `<AR>`. Repeated spaces drop out. |
| `M1_REPEAT` to `M4_REPEAT` | `ON` or `OFF`. |
| `MEM_EVERY` | Seconds between repeats, 2 to 30. |
| `MEM_STAY_OPEN` | `ON` or `OFF`. |

| Command | Reply |
| - | - |
| `GET SPEED` | `VALUE SPEED 25` |
| `GET` | A `VALUE` line for every name, then `END`. |
| `SET SPEED 25` | `OK SPEED 25`. The device redraws the screen and saves the change as it does from the dial. |
| `SET SPEED 70` | `ERROR SPEED: 5 TO 60`. Nothing changes. The reason lists the values the setting takes. |

`SET` cannot turn USB off: `SET USB_OUT OFF`, and `SET DECODER_BOARD OFF` while USB OUT is RADIO, answer an error. Set them on the device.

## Edges

After `EDGE ON`, the device sends a line for each change on the key and paddle lines, until `EDGE OFF` or the computer closes the port:

```
EDGE KEY 1 123456 4021
```

The line is `KEY` for the keyed signal, from the paddles or a `SEND`, and `TIP` and `RING` for the paddle contacts. `1` is closed or down and `0` open or up. The third field is the time on the device's millisecond count.

The last field is the sequence number. The device counts every change on these lines, sent or not, and the count wraps after 2^32 - 1 to 0. A jump of more than one between two lines means lines were lost between them: in a full queue to the screen, or in a full serial queue, which counts in `n SERIAL LINES LOST`.

`EDGE ON` answers with the settings a replay of the lines needs, and the device sends them again whenever one changes while edge lines are on:

```
VALUE MODE IAMBIC B
VALUE PADDLES NORMAL
VALUE SPEED 25
```

## VBand Hotline

VBand's page can read keying from a serial port, as it does from a CW Hotline. It opens the port at 115200 baud and writes `VBand On`; when its Disconnect button is clicked, it writes `VBand Off`. In between, the device sends one line for each mark of the signal that keys the radio: your keying, `SEND` text and memories.

```
SM,3000,60
SM,60,180
```

The first field is the space before the mark and the second is the mark, both in ms and at most 3000. The first mark after `VBand On`, and any mark after a longer space, has a space of 3000, which VBand plays as the start of an over. A line comes when its mark ends. VBand skips every other line, so the event lines can go on beside these.

While the Hotline is on, the device sends no keyboard reports. It ends on `VBand Off`, or when the computer closes the port.

## Stats

`STATS LETTERS` sends `LEVEL n`, the number of Koch characters in play, then one line for each Koch character in Koch order, then the mixups, most common first, then `END`:

```
LETTER K 15 80 KNOWN
MIXUP A N 3
```

A `LETTER` line gives the character, its tries, the percent copied right, and `KNOWN`, `UNSURE`, `WEAK`, or `NEW` for a character not yet sent. A `MIXUP` line gives the character sent, the one copied, and how often.

`STATS PROGRESS` sends one line for each practice session, oldest first, then `END`:

```
SESSION 12 600 3 120 100 450
```

The fields are the session number, the seconds of practice, the contacts logged, the characters sent in receive drills, how many of those you copied right, and the characters you keyed.

`STATS TIMING` sends the timing of your keying since power on, or since CLR on the TIMING screen, then `END`:

```
MARK DIT 120 62.40 4.10
MARK DAH 118 190.25 9.80
GAP ELEMENT 210 1.02 0.15
GAP CHAR 40 3.10 0.42
GAP WORD 9 7.80 1.25
RATIO 3.05
WPM 19.60
BINS MARK 4 0 0 0 1 12 ...
BINS GAP 8 0 2 ...
```

- A `MARK` or `GAP` line gives the count, the mean and the standard deviation. Marks are in ms; gaps are in dots of the keyer speed at the time. The standard deviation divides by the count, so one sample gives 0. `-` stands for a value that needs at least one sample.
- `RATIO` is the dah length over the dit length; 3 is standard. `WPM` is the speed your timing works out to. Each line comes only once its value is known.
- A `BINS` line is a histogram: the range in dots, then the count in each of 24 equal bins from 0. The last bin also holds everything longer than the range.
- Numbers have two decimals.
- Marks in ms mix speeds when SPEED changes after a CLR. Text that `SEND` keys counts too, since it keys the same line.

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
| `EDGE LINE 1 MS SEQ` | A key or paddle line changed, after `EDGE ON`. See [Edges](#edges). |
| `VALUE NAME VALUE` | After `EDGE ON`, a keyer setting at the start and when it changes. See [Edges](#edges). |
| `SM,SPACE,MARK` | A mark ended, after `VBand On`. See [VBand Hotline](#vband-hotline). |

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
| `00 0A` set WK1 mode | Stops the pushbutton bytes. |
| `00 0B` set WK2 mode | Starts the pushbutton bytes. |
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
| Speed pot, `0x80` plus SPEED less the pot's lowest speed | On request, and when SPEED changes on the device. A speed the host set sends none. |
| Pushbutton, `0xC8` with a bit for the button down | In WK2 mode, when F1 to F4 go down and up: bit 0 for F1, 1 for F2, 2 for F3, 4 for F4. The screen does not see them then. |
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
| Settings and memories by name, for `GET` and `SET` | `logic/src/config.rs` |
| Replies to `INFO`, `GET`, `SET`, `PLAY` and `STATS` | `firmware/src/ui/serial.rs`, and `answer` in `firmware/src/main.rs` |
| The sender that keys `SEND` and WinKeyer text | `logic/src/sender.rs` |
| The WinKeyer parser | `logic/src/winkeyer.rs` |
| WinKeyer actions, and the bytes to the host | `winkeyer_act` and `wk_chunk` in `firmware/src/main.rs` |
| The USB device, keyboard and serial port | `firmware/src/usb.rs` |
| The USB task, which reads commands and writes the queued bytes | `usb_events` in `firmware/src/main.rs` |
| The serial key, merged with the keyer output | `tick` in `firmware/src/main.rs` |
| Event lines, `EDGE` lines, `SEND` and `STOP` | `event_line`, `emit` and the `ui` task in `firmware/src/main.rs` |
| Hotline marks and spaces | `logic/src/hotline.rs`, and `tick` in `firmware/src/main.rs` |
| Pushbutton bytes | `tick` in `firmware/src/main.rs` |
| The screenshot, row by row, from the UI task | `send_screen` in `firmware/src/main.rs`, `read_row` in `firmware/src/lcd.rs` |
