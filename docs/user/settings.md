# Settings

All settings are on one screen with seven tabs: KEYER, SOUND, RECV, QSO, THEME, CALL and MEM. The device saves the settings 2 seconds after your last change.

## Open the settings

| From | Press | Opens at tab |
| - | - | - |
| [Menu](menu.md), row 09 SETTINGS | Dial click | KEYER |
| Menu, KEYER, TIMING | F1 (SET) | KEYER |
| Receive drills, ragchew, LETTERS, RADIO | F1 (SET) | RECV |
| POTA, SOTA, HUNT | F1 (SET) | QSO |
| SYSTEM | F1 (SET) | THEME |

F4 (BACK) leaves the settings and returns to the screen you came from. A drill or a QSO you paused continues. From row 09 of the menu, F4 (BACK) returns to the menu.

## Controls

These controls work on the KEYER, SOUND, RECV, QSO and THEME tabs. The CALL and [MEM](memories.md#edit-the-memories) tabs have their own controls.

| Control | Action |
| - | - |
| F1 (TAB) | Opens the next tab. After MEM, KEYER opens again. |
| F2 (-) | Changes the value of the selected row down one step. |
| F3 (+) | Changes the value of the selected row up one step. |
| F4 (BACK) | Leaves the settings. |
| Dial turn | Selects another row. While you edit, changes the value. |
| Dial click | Starts or ends editing the selected row. While you edit, the value shows as `< value >`. |
| Dial hold | Turns the device off. |
| Key | Nothing more. |

Numbers stop at their lowest and highest values. A choice of words goes around: after the last value, the first value comes again.

## KEYER tab

| Row | Values | Default | What it sets |
| - | - | - | - |
| MODE | IAMBIC A, IAMBIC B, STRAIGHT | IAMBIC B | Your key type. See [Keys and jacks](keys.md). |
| PADDLES | NORMAL, REVERSED | NORMAL | NORMAL: dots on the tip contact. REVERSED: dots on the ring contact. STRAIGHT mode ignores this row. |
| SPEED | 5 to 60 WPM | 20 WPM | Your sending speed. |
| KEY OUT | ON, OFF | ON | Whether the func jack keys. |
| OUT TYPE | STRAIGHT, PADDLE | STRAIGHT | STRAIGHT: one line keys, for a radio's straight key input. PADDLE: two lines follow your contacts, for the radio's own keyer. |
| OUT LINES | TIP KEYS, RING KEYS (STRAIGHT); DIT ON TIP, DIT ON RING (PADDLE) | TIP KEYS or DIT ON TIP | Which lines of the func jack key. |

## SOUND tab

| Row | Values | Default | What it sets |
| - | - | - | - |
| TONE | 300 to 1200 Hz, in steps of 10 Hz | 700 Hz | The pitch of your sidetone. Other stations send at pitches near it. |
| VOLUME | 0 to 44 | 23 | 0 is off. Each step is 1.5 dB. |
| WAVE | SINE, SQUARE | SINE | SINE is a pure tone. SQUARE is a buzzier tone with soft edges. |
| NOISE | OFF, 1 to 10 | OFF | Band noise under the stations you hear. |
| FADING | OFF, 1 to 10 | OFF | How deep the stations you hear fade in and out. |

The tab says `NOISE AND FADING: DRILLS, QSO, HUNT`: noise and fading play in the receive drills, the ragchew, and on the POTA, SOTA and HUNT screens. While the SOUND tab is open, the band noise plays, so you hear the level you set.

## RECV tab

| Row | Values | Default | What it sets |
| - | - | - | - |
| CHAR SPEED | MATCH, 5 to 40 WPM | MATCH | The speed of each character the device sends you. MATCH uses your keyer SPEED. |
| FARNSWORTH | OFF, 5 to 40 WPM | OFF | Longer gaps between characters and words, so the text comes at this slower overall speed. It never goes above CHAR SPEED. |
| CHEW SHOWS | WHOLE OVER, EACH LINE | WHOLE OVER | When the [ragchew](ragchew.md) CHEW shows its text. |
| DECODER BOARD | ON, OFF | OFF | ON: the device powers the decoder board of the Pro model, and the menu shows row 11 RADIO. See [Radio](radio.md). |

The tab says `SPEED OF WHAT YOU HEAR: BOTS, DRILLS`: CHAR SPEED and FARNSWORTH set the speed of every station the device sends you.

## QSO tab

| Row | Values | Default | What it sets |
| - | - | - | - |
| CHAR SPEED | MATCH, 5 to 40 WPM | MATCH | The same setting as CHAR SPEED on the RECV tab. |
| FARNSWORTH | OFF, 5 to 40 WPM | OFF | The same setting as FARNSWORTH on the RECV tab. |
| PILEUP | OFF, TAKE TURNS, OVERLAP | TAKE TURNS | How the hunters call when you [activate](activate.md). |
| FIST | HUMAN, CLEAN | HUMAN | HUMAN: each station keys with small timing errors. CLEAN: exact timing. |
| SPEED SPREAD | ON, OFF | ON | ON: each hunter or activator sends up to 5 WPM faster or slower than CHAR SPEED. |

The tab says `SAME SPEEDS AS THE RECV TAB`.

## THEME tab

| Row | Values | Default | What it sets |
| - | - | - | - |
| STYLE | HAZARD, HUD, BLOCK | HAZARD | The shapes of the screen: the frame, the header, the button labels and the selection. |
| PALETTE | 15 palettes | ORANGE | The colors of the screen and of the boot screen. |
| BRIGHTNESS | 10 % to 100 %, in steps of 10 % | 100 % | The brightness of the screen. |

The palettes are ORANGE, CYAN, AMBER, DAILY, and the pride flags RAINBOW, TRANS, BI, LESBIAN, PAN, NONBIN, ACE, ARO, FLUID, AGENDER and QUEER. DAILY shows a different flag each time you turn the device on, and the tab says `A NEW FLAG EACH BOOT`. Under the rows, a swatch shows the colors of each palette, with a frame around the selected one.

The colors that grade your timing are the same in every palette: green, amber and red.

## CALL tab

The CALL tab shows your callsign in large letters. The QSO and hunt screens use it, and the boot screen draws it. The default is N0CALL, a placeholder callsign.

Key your callsign on the key. Each character you key goes at the end. A callsign holds up to 10 characters: letters, digits and `/`. The device ignores other characters.

| Control | Action |
| - | - |
| F1 (TAB) | Opens the next tab. |
| F2 (DEL) | Deletes the last character. |
| F3 (CLR) | Deletes the whole callsign. |
| F4 (BACK) | Leaves the settings. |
| Dial turn | Nothing. |
| Dial click | Nothing. |
| Dial hold | Turns the device off. |
| Key | Adds characters to your callsign. |

## MEM tab

The MEM tab holds your four memories. See [Memories](memories.md#edit-the-memories).
