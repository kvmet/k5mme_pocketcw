# Timing

The TIMING screen gives a close look at your keying. The header shows TIMING, the keyer mode, and the SND and VOL values.

The screen measures every mark and gap against the dot length the decoder uses: the dot length of your set speed in the iambic modes, or of the speed you actually send in STRAIGHT mode. It grades each mark and gap as good, off or bad. See [Timing grades](keyer.md#timing-grades).

## What the screen shows

### Zoomed tape

The tape draws time from left to right, 8 pixels per dot length.

- The SET row shows brackets around the ideal length of each mark.
- The YOU row shows each mark you sent, in the color and shape of its grade. A good mark is solid, an off mark has a stripe through it, and a bad mark is an outline.
- Over each mark, a number shows its error in percent: `+10` is 10 % long, `-10` is 10 % short.
- Under the marks, each decoded character shows, and each gap between characters or words shows its length in units.
- A gap inside a character that is off or bad gets an underline in its grade.

### Histograms

| Histogram | Range | Dotted lines at |
| - | - | - |
| MARKS | 0 to 4 units | 1 (dot) and 3 (dash) |
| GAPS | 0 to 8 units | 1 (inside a character), 3 (between characters) and 7 (between words) |

Each bar counts the marks or gaps of that length, in the color of the grade a mark or gap of that length gets.

### Averages

| Line | What it shows |
| - | - |
| DIT | Average dot length and its standard deviation (SD) in milliseconds, then RATIO: average dash over average dot. The ideal ratio is 3. |
| DAH | Average dash length and its standard deviation in milliseconds, then the speed your timing works out to. |
| GAP, CHR, WRD | Average gap inside characters, between characters and between words, in units. |

The statistics collect your keying on every screen since the device was turned on, or since you last pressed F2 (CLR). The KEYER screen shows the same statistics.

## Controls

| Control | Action |
| - | - |
| F1 (SET) | Opens the settings at the KEYER tab. |
| F2 (CLR) | Clears the statistics and the tape. |
| F3 | Nothing. |
| F4 (BACK) | Opens the KEYER screen. |
| Dial turn | Changes the focused header value. See [Header values](controls.md#header-values). |
| Dial click | Moves the focus to the next header value. |
| Dial hold | Turns the device off. |
| Key | Draws on the tape and adds to the statistics. |
