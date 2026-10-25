# Status messages

The status line at the bottom of the screen shows short messages. In this list, `n` is a number and `X` is a character.

## Power and battery

| Message | Meaning |
| - | - |
| `RELEASE TO POWER OFF` | You held the dial for 1 second. Let go of the dial to turn the device off, or press an F button or turn the dial to stay on. |
| `BATTERY LOW: PLUG IN USB` | The battery is below 3.4 V. Connect USB power to charge it. |
| `BATTERY EMPTY: POWERING OFF` | The battery is empty. After 2 seconds the device saves everything and turns off. |

See [System](system.md).

## Saves

Each of these messages means a write to the memory chip failed. The device keeps the values while it is on, but they can be lost when it turns off.

| Message | What was not saved |
| - | - |
| `SETTINGS NOT SAVED` | Your settings. |
| `MEMORIES NOT SAVED` | Your [memories](memories.md). |
| `LETTER STATS NOT SAVED` | Your [letter statistics](letters.md) and Koch level. |
| `PROGRESS NOT SAVED` | This session in [Progress](progress.md). |
| `BATTERY CALIBRATION NOT SAVED` | The full point of the battery gauge. |

When a save fails at power off, the device waits 3 seconds before it turns off, so you can read the message. If these messages come again and again, look at the EEPROM row on the [System](system.md) screen.

## Key and hardware

| Message | Meaning |
| - | - |
| `DIT STUCK ON: CHECK PLUG` | A contact of the key jack stayed closed too long, and stopped keying until it opens. The message names DIT, DAH, TIP or RNG. See [Keys and jacks](keys.md). |
| `CODEC DID NOT ANSWER` | The audio chip did not take a new volume. |
| `n INPUTS LOST` | Button presses, key changes or decoded characters came faster than the screen could handle them, and the device lost `n` of them. |
| `n USB KEYS LOST` | The device lost `n` characters or key changes on their way to the computer. See [USB keyboard](usb.md). |
| `n SERIAL ERRORS` | USB refused `n` reads or writes on the serial port. |
| `n SERIAL LINES LOST` | The serial port had no room for `n` lines to the computer. The computer was not reading them. |
| `SCREENSHOT NOT SENT` | The computer stopped taking a screenshot for 0.5 s. |
| `KEY OUT ON`, `KEY OUT OFF` | You held F1 and turned KEY OUT on or off. See [Controls](controls.md#held-buttons). |
| `KEY OUT ON: NOT IN PRACTICE` | You held F1 and turned KEY OUT on, on a screen where your key does not key the radio. It keys the radio on the KEYER, TIMING and RADIO screens. |

## WinKeyer

These messages show while a logging program uses the device as a WinKeyer. See [USB keyboard](usb.md#winkeyer).

| Message | Meaning |
| - | - |
| `WINKEYER OPEN` | A logging program took the serial port as a WinKeyer. |
| `WINKEYER CLOSED` | The logging program let the serial port go, or the computer closed it. Text it had not sent yet is dropped. |
| `WINKEYER n WPM` | The logging program set SPEED to `n` WPM. |
| `n WINKEYER BYTES LOST` | The device dropped `n` bytes to or from the logging program, because its queue was full. |

## VBand Hotline

These messages show when VBand's page connects to the device as a CW Hotline. See [USB keyboard](usb.md#vband-cw-hotline).

| Message | Meaning |
| - | - |
| `HOTLINE ON` | VBand took the serial port. The device sends your keying to it, and types nothing. |
| `HOTLINE OFF` | VBand let the serial port go, or the browser closed it. |

## Memories

These messages show on the KEYER screen. See [Memories](memories.md).

| Message | Meaning |
| - | - |
| `CLICK CLOSES` | The memory row is open. A dial click closes it. |
| `LIT M REPEATS  CLICK CLOSES` | The memory row is open, and a lit label marks a memory that repeats. |
| `SENDING Mn  KEY TO STOP` | Memory n is sending. Touch your key to stop it. |
| `Mn AGAIN IN nS  KEY TO STOP` | Memory n repeats after the time shown. |
| `Mn TONE ONLY: OUT IS PADDLE` | Memory n plays as a tone and does not key the func jack, because OUT TYPE is PADDLE. |
| `Mn IS EMPTY (SETTINGS, MEM)` | Memory n holds no message. Key one in on the MEM tab of the settings. |
| `MEMORY FULL` | On the MEM tab: the memory has no room for the last character you keyed. A memory holds up to 48 characters, and a prosign such as `<AR>` takes 4. |

## Practice

| Message | Meaning |
| - | - |
| `LEVEL UP: ADDED X` | You moved up a Koch level, and the drill now sends character X too. See [Receive drills](receive.md). |
| `CLR AGAIN TO ERASE STATS` | On the LETTERS screen: press F2 (CLR) again to erase your letter statistics. See [Letters](letters.md). |

## The red error screen

If PocketCW finds an error it cannot continue from, the screen turns dark red and shows what went wrong. The func jack and the LED turn off, so a radio does not stay keyed. Click the dial to turn the device off. See [Troubleshooting](troubleshooting.md).
