# Controls and power

## The controls

| Control | How you use it |
| - | - |
| F1 to F4 | The four buttons beside the screen, F1 at the top. Press a button to use it. The right edge of the screen shows what each button does on the current screen. A blank label means the button does nothing there. |
| Dial turn | Turn the dial left or right. Hold it turned to step again and again. |
| Dial click | Press the dial and let go within 1.5 seconds. |
| Dial hold | Press the dial and keep it down for 1.5 seconds. This turns the device off on every screen after the boot screen. |
| Key | The paddle, straight key or sideswiper in the key jack. See [Keys and jacks](keys.md). |

Each screen page in this manual has a table of what these controls do on that screen.

On every screen, your key sounds the sidetone, and keys the func jack when KEY OUT is ON. The Key row of each table lists what else the key does on that screen.

## Turn on

Click the dial. The boot screen shows Morse code falling as rain in the colors of your palette. The rain builds your callsign in large letters. Then a line with the version and your callsign shows, and a list of startup checks, each marked as passed or failed.

To skip the boot screen, press any F button or turn the dial. The KEYER screen opens after the boot screen.

## Turn off

Hold the dial. After 1.5 seconds the status line says `RELEASE TO POWER OFF`. Let go of the dial, and the device saves everything and turns off.

If a save fails, the status line names what was not saved, and the device waits 3 seconds before it turns off.

When the battery is empty, the device saves everything and turns off by itself. See [System](system.md).

## The screen

- The header at the top names the screen. It also shows details such as the keyer mode or the speed, and a battery icon at the right end.
- The column at the right edge shows the labels of F1 to F4.
- The status line at the bottom shows short messages. A message stays until the screen draws that area again. See [Status messages](status-messages.md).

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
