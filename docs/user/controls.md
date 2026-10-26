# Controls and power

## The controls

| Control | How you use it |
| - | - |
| F1 to F4 | The four buttons beside the screen, F1 at the top. Press a button and let go to use it. The column at the right edge, under the header, shows what each button does on the current screen. A blank label means the button does nothing there. |
| F hold | Keep an F button down for half a second. The button does its second action as soon as the half second ends, and nothing more when you let go. The small label in the button's box names the second action. A button with no second action does its usual action when you let go. |
| Dial turn | Turn the dial left or right. Hold it turned to step again and again. |
| Dial click | Press the dial and let go within 1 second. |
| Dial hold | Press the dial and keep it down for 1 second. This turns the device off on every screen after the boot screen. |
| Key | The paddle, straight key or sideswiper in the key jack. See [Keys and jacks](keys.md). |

Each screen page in this manual has a table of what these controls do on that screen.

## Held buttons

Two held buttons work on every screen:

| Hold | Label | Action |
| - | - | - |
| F1 | Set by F1 HOLD | Does what F1 HOLD in the DEVICE section of the [settings](settings.md#device) sets. The default is KEY OUT. |
| F4 | `HOME` | Opens the KEYER screen. A drill or QSO you were in pauses: RESUME in the [menu](menu.md) goes on with it. |

On KEYER, F4 has no hold. With the memory row open, every hold sends its memory again and again instead. See [Memories](memories.md).

F1 HOLD has four values:

| F1 HOLD | Label | Action |
| - | - | - |
| KEY OUT | `KEYOUT` | Turns KEY OUT on or off. The status line shows `KEY OUT ON` or `KEY OUT OFF`. On a screen where your key does not key the radio, it shows `KEY OUT ON: NOT ON THIS SCREEN`. See [The func jack](keys.md#the-func-jack). |
| RESUME | `RESUME` | Goes on with the drill or QSO you paused. With nothing paused, it starts the last one you opened again. With no last one, the status line shows `NOTHING TO RESUME`. |
| MEM ROW | `MEM` | Opens the KEYER screen with the [memory](memories.md) row open. |
| TIMING | `TIMING` | Opens the [TIMING](timing.md) screen. |

Some screens have more held buttons:

| Screen | Hold | Label | Action |
| - | - | - | - |
| KEYER, memory row open | F1 to F4 | `REPEAT` | Sends that memory again and again. See [Memories](memories.md). |
| LETTERS | F2 | `CLEAR` | Erases your letter statistics. See [Letters](letters.md). |
| TIMING | F2 | `CLEAR` | Clears the statistics and the tape. See [Timing](timing.md). |
| Settings, editing a memory or your callsign | F3 | `CLEAR` | Clears the text. |

On every screen, your key sounds the sidetone and types or keys VBand over USB when USB OUT is TEXT or VBAND. On the KEYER, TIMING and RADIO screens, it also keys the func jack when KEY OUT is ON. The Key row of each table lists what else the key does on that screen.

## Turn on

Click the dial. The boot screen shows Morse code falling as rain in the colors of your palette. The rain builds your callsign in large letters. Then a line with the version and your callsign shows, and a list of startup checks, each marked as passed or failed.

To skip the boot screen, press any F button or turn the dial. The KEYER screen opens after the boot screen.

## Turn off

Hold the dial. After 1 second the status line says `RELEASE TO POWER OFF`. Let go of the dial, and the device saves everything and turns off.

To stay on, press an F button or turn the dial before you let go. The status line clears, and the button does nothing else.

If a save fails, the status line names what was not saved, and the device waits 3 seconds before it turns off.

When the battery is empty, the device saves everything and turns off by itself. See [System](system.md).

## The screen

- The header across the top names the screen. It also shows details such as the keyer mode, the values the dial sets, and a battery icon at the right end. `TX` shows while your key keys the radio.
- The column at the right edge, under the header, shows the labels of F1 to F4.
- The footer at the bottom shows what the screen waits for or what the selected row does. Short messages show there for 3 seconds, brighter, and then the footer comes back. See [Status messages](status-messages.md).

## Header values

On every screen except KEYER, MENU and SETTINGS, the dial sets values that show at the right end of the header. Each screen shows the values it uses.

| Value | What it is | Setting |
| - | - | - |
| `VOL 23` | The volume. | VOLUME in the SOUND section |
| `SND 20` | Your keyer speed, in WPM. | SPEED in the KEYER section |
| `RCV 20/15` | The speed of what the device sends you: the character speed, then the Farnsworth speed. Equal numbers mean no Farnsworth spacing. | CHAR SPEED and FARNSWORTH in the RECEIVE section |

- A dial click moves the focus to the next value. The focused number shows inverted. On RECEIVE KOCH, the focus also reaches `LEVEL` under the header.
- A dial turn changes the focused value.
- A dial turn does nothing while you key, and for one word gap after you stop. A brushed dial cannot change a value in the middle of your over.
- Opening a screen puts the focus on RCV where it shows, on SND on TIMING, and on VOL on the others. After F1 (SET) and then F4 (BACK), the focus stays where it was.
- With CHAR SPEED at MATCH, RCV shows your keyer speed. A turn sets CHAR SPEED, starting from that speed. To set MATCH again, use the RECEIVE section of the settings.
- Turn the second RCV number down for Farnsworth spacing. Turn it up to the first number to stop the spacing.
- On QSO and HUNT, a new RCV speed moves every station by the same amount: each keeps its offset from RCV and its fist. Any QRS ends.

## What the device saves

The device keeps everything below when it is off. It has no clock, so it does not keep the date or time.

| What | When it saves |
| - | - |
| Settings | 2 seconds after your last change. |
| Memories | When you leave the settings screen. |
| Letter statistics and Koch level | When you leave a receive drill or a ragchew. |
| Progress of this session | At most once a minute. |
| Battery calibration | When it changes. |
| Settings, memories, letter statistics and progress | When you turn the device off. |

If the device loses power in another way, such as when you take the battery out, it can lose changes it did not save yet.
