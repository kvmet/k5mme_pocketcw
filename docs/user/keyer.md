# Keyer

The KEYER screen is the main screen. It opens after the boot screen. The header shows KEYER and the keyer mode.

## What the screen shows

| Area | What it shows |
| - | - |
| Large digits | Your sending speed in WPM, 5 to 60. |
| TONE | The sidetone pitch in Hz. |
| VOL | The volume, 0 to 44. 0 is off. |
| OUT | KEY OUT: ON when the func jack keys, OFF when it does not. |
| DIT | The length of one dot at your speed, in milliseconds. |
| Bar | Your speed between 5 and 60 WPM. |
| Timing tape | Your keying drawn to scale. See below. |
| Statistics line | Your average gaps and speed. See below. |
| Decode | Five lines of what your keying decodes to. |

A small mark under a value shows which value the dial sets: speed, tone or volume.

### Timing tape

The tape draws time from left to right, 4 pixels per dot length.

- The KEY row shows the keyer output. In STRAIGHT mode, each mark has the color and shape of its timing grade.
- The two rows under it show the two contacts of your key: DIT and DAH, or TIP and RNG in STRAIGHT mode.
- Under the rows, each decoded character shows under its marks.
- Each gap between characters or words shows its length in units, in the color of its grade. A thin tick shows where the ideal gap would end.

A new page starts after a pause of 14 units or more, or when the next element would not fit.

### Timing grades

Every mark and gap gets a grade:

| Grade | Color | Meaning |
| - | - | - |
| Good | Green | Within 20 % of the ideal length. |
| Off | Amber | Between good and bad. |
| Bad | Red | More than halfway from the ideal length to the length where the decoder reads it as another kind, such as a dot read as a dash. |

Each grade also has its own shape, so you can read the grades without color.

### Statistics line

`CHR` and `WRD` are your average character gap and word gap in units (ideal: 3 and 7). `RATIO` is your average dash length over your average dot length (ideal: 3). The last value is the speed your timing works out to. Dashes in place of a value mean no data yet. F2 (CLR) on the [Timing](timing.md) screen clears these statistics.

### Decode

Each character has an underline in its grade: a thin line for good, a dashed line for off, a thick bar for bad. The grade is the worst timing in the character: its marks, its gaps, and the gap before it. `*` is a pattern that is not a character. A prosign shows as its two letters under a bar.

## Controls

| Control | Action |
| - | - |
| F1 (SET) | Opens the settings at the KEYER tab. |
| F2 (MEM) | Opens the memory row. See [Memories](memories.md). |
| F3 (OUT) | Turns KEY OUT on or off. |
| F4 (MENU) | Opens the [menu](menu.md). |
| Dial turn | Changes the marked value: speed, tone or volume. |
| Dial click | Moves the mark to the next value: speed, then tone, then volume. |
| Dial hold | Turns the device off. |
| Key | Draws on the timing tape and decodes. Stops a memory that is sending. |
