# Receive drills

In a receive drill, the device sends a group of characters, and you key back what you copied. The device then shows the group with each character marked right or wrong.

To start, open 03 RECEIVE in the [menu](menu.md). The Koch drill starts. F2 (KIND) moves to the next drill:

KOCH, WORDS, CALLS, then the [ragchew](ragchew.md) CHEW and QUIZ, then KOCH again.

| Drill | What the device sends |
| - | - |
| KOCH | Groups of 5 characters from the Koch characters of your level. |
| WORDS | One common word or abbreviation heard on the air, such as `QTH`, `5NN` or `<BK>`. |
| CALLS | One US callsign. |

The device sends at CHAR SPEED, with FARNSWORTH spacing when you set it. See the RECV tab in [Settings](settings.md). The RCV value in the header shows the speed, such as `20/20`, or `20/10` with Farnsworth spacing.

## How a drill runs

1. The device sends a group. The status shows `LISTEN`.
2. The status shows `YOUR TURN`. Start to key what you copied within 5 seconds. You can also start while the group still plays.
3. While you key, the status shows `PAUSE ENDS`: when you stop keying for three word gaps, your copy ends.
4. The device scores your copy and shows it in the list. The next group starts after about 1 second.

If you do not start to key within 5 seconds, the group counts as missed.

To hear the group again, key `?` alone as your copy.

## What the screen shows

- The top line: in KOCH, your level and your progress to the next level, such as `LEVEL 5  80% OF 30  NEED 90% OF 50`.
- The character set, each character in the color of how well you copy it. Green: you know it. Amber: not enough tries to tell. Red, inverted: weak. Gray: not sent yet. KOCH shows the characters of your level, and a line marks the newest one. WORDS and CALLS show all 44 characters.
- The list of recent groups. Each row shows the group that was sent, then your copy with each character marked: green for right, red and inverted for wrong, `_` for missed. The score shows at the right, such as `4/5`.
- Your copy as you key it, with the status at the right.
- The footer: your score for this session, such as `SESSION 85%  120 CHARS`.

## The Koch method

The KOCH drill starts at level 2, with K and M. You move up one level when you copy at least 90 % right over 50 characters. Each level adds the next character in this order:

`K M U R E S N A P T L W I . J Z = F O Y , V G 5 / Q 9 2 H 3 8 B ? 4 7 C 1 D 6 0 X <AR> <SK> <KN>`

The score counts extra characters you key as errors, the same as missed ones. After 50 characters below 90 %, the count starts again. When you move up, the status line shows the new character, such as `LEVEL UP: ADDED U`.

The drill picks weak characters more often, and the newest character twice as often.

The device keeps your level and your progress toward the next level while you use other screens. The level stays after you turn the device off.

## Controls

| Control | Action |
| - | - |
| F1 (SET) | Pauses the drill and opens the settings at the RECV tab. F4 (BACK) there returns, and the next group plays at the new settings. |
| F2 (KIND) | Starts the next drill. |
| F3 (STOP or GO) | Pauses the drill, or starts it again. |
| F4 (BACK) | Opens the [menu](menu.md). |
| Dial turn | Changes the focused header value. See [Header values](controls.md#header-values). | KOCH also has `LEVEL` under the header: it sets your level, 2 to 44. A new level starts its progress count from zero; turn back to the level you left and its count returns. |
| Dial click | Moves the focus to the next header value. |
| Dial hold | Turns the device off. |
| Key | Keys your copy. |

Your scores also go to [Letters](letters.md) and [Progress](progress.md).
