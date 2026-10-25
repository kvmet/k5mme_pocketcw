# Receive drills

In a receive drill, the device sends a group of characters, and you key back what you copied. The device then shows the group with each character marked right or wrong.

To start, open RECEIVE in the [menu](menu.md), then the drill. The header shows the name of the drill. For a ragchew to copy, see [Ragchew](ragchew.md) in the ON AIR list.

| Drill | What the device sends |
| - | - |
| KOCH | Groups of 5 characters from the Koch characters of your level. |
| WORDS | One common word or abbreviation heard on the air, such as `QTH`, `5NN` or `<BK>`. |
| CALLS | One US callsign. |
| NINJA | One word from the content pack, then the voice says it. See [NINJA](#ninja). |
| HEAD | Words that get longer as you copy them, then two words, then a QSO line. See [HEAD](#head). |

The device sends at CHAR SPEED, with FARNSWORTH spacing when you set it. See the RECV tab in [Settings](settings.md). The RCV value in the header shows the speed, such as `20/20`, or `20/10` with Farnsworth spacing.

## How a drill runs

1. The device sends a group. The status shows `LISTEN`.
2. The status shows `YOUR TURN`. Start to key what you copied within 5 seconds. You can also start while the group still plays, except in HEAD.
3. While you key, the status shows `PAUSE ENDS`: when you stop keying for three word gaps, your copy ends.
4. The device scores your copy and shows it in the list. The next group starts after about 1 second.

If you do not start to key within 5 seconds, the group counts as missed.

To hear the group again, key `?` alone as your copy.

## What the screen shows

- The top line: in KOCH, your level and your progress to the next level, such as `LEVEL 5  80% OF 30  NEED 90% OF 50`.
- The character set, each character in the color of how well you copy it. Green: you know it. Amber: not enough tries to tell. Red, inverted: weak. Gray: not sent yet. KOCH shows the characters of your level, and a line marks the newest one. The other drills show all 44 characters.
- In HEAD, the top line shows your rung and what it sends, such as `RUNG 3 OF 8  1 WORD OF 5 CHARS`.
- The list of recent groups. Each row shows the group that was sent, then your copy with each character marked: green for right, red and inverted for wrong, `_` for missed. The score shows at the right, such as `4/5`.
- In HEAD, each item takes two rows: what was sent, then your marked copy under it. The list shows the last 3 items.
- Your copy as you key it, with the status at the right.
- The footer: your score for this session, such as `SESSION 85%  120 CHARS`.

## NINJA

NINJA works like [Morse Code Ninja](https://morsecode.ninja): the device sends a word, and then the voice says it. You can just listen, or copy it in your head and key it back first.

1. The device sends a word. The status shows `LISTEN`.
2. The status shows `YOUR TURN`. To be scored, start to key the word within 1.5 seconds.
3. If you key, the device scores your copy as in the other drills, then the voice says the word.
4. If you do not key, the voice says the word, and the list shows it with no score.
5. The next word starts about 1 second after the voice ends.

The words are the ham words and abbreviations of the content pack, then the most common English words: 500 by default. NINJA WORDS on the RECV tab of the [settings](settings.md) sets how many. An abbreviation on its own comes with its meaning: after `QTH`, the voice says "Q T H. location".

NINJA needs the content pack. Without it, the drill shows `CONTENT PACK MISSING: INSTALL IT` and sends nothing. [Install](install.md) again, with the [content pack](install.md#the-content-pack).

## HEAD

HEAD is a head-copy ladder. You copy each item in your head while it plays, and key it back after it ends. Keying while it plays does not count.

| Rung | What the device sends |
| - | - |
| 1 | One word of 2 or 3 characters |
| 2 | One word of 4 |
| 3 | One word of 5 |
| 4 | One word of 6 |
| 5 | One word of 7 to 10 |
| 6 | Two words of 2 to 4 each |
| 7 | Two words of 3 to 6 each |
| 8 | A line of a QSO, such as `NAME BOB BOB` or `RIG IC7300 100W` |

Three copies in a row all right move you up one rung. Three copies in a row under half right move you down one. An extra character makes a copy not all right. A `?` repeat does not count.

The words come from the content pack: ham words and common English words. HEAD needs the content pack, as NINJA does. The device keeps your rung while you use other screens, but not after you turn it off.

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
| F2 (AGN) | While a group plays or waits for your copy: sends the group again, and clears your copy. |
| F3 (STOP or GO) | Pauses the drill, or starts it again. |
| F4 (BACK) | Pauses the drill and opens the RECEIVE list of the [menu](menu.md). Click the drill or RESUME there to go on. |
| Dial turn | Changes the focused header value. See [Header values](controls.md#header-values). | KOCH also has `LEVEL` under the header: it sets your level, 2 to 44. A new level starts its progress count from zero; turn back to the level you left and its count returns. |
| Dial click | Moves the focus to the next header value. |
| Dial hold | Turns the device off. |
| Key | Keys your copy. |

Your scores also go to [Letters](letters.md) and [Progress](progress.md).
