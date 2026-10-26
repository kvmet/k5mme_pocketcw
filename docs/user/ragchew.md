# Ragchew

In a ragchew, two stations hold a full QSO for you to copy. One station calls CQ, the other answers, and they trade signal reports, names, cities (QTH), rigs, antennas, weather (WX), years licensed, ages and jobs. Then they sign off with 73, and the station that called CQ sends the last `E E`. After 11 overs, two new stations start a new QSO.

Each station has its own way to word its overs, such as `NAME` or `OP`, and `HW?` or `HW CPY?`. Most stations are in the US. Some are in Canada or other countries: these send temperatures in Celsius and antenna heights in meters, and a station outside the US and Canada sends a city with no state.

Open ON AIR in the [menu](menu.md), then CHEW or QUIZ. The header shows which one:

| Kind | What you do |
| - | - |
| CHEW | Listen. The text of each over shows after the over ends. |
| QUIZ | After an over, answer a question about it. The device scores your answer like a receive drill. |

The stations send at CHAR SPEED, with FARNSWORTH spacing when you set it. With FIST set to HUMAN, each station keys with its own small timing errors. The two stations send at slightly different pitches. See [Settings](settings.md).

## What the screen shows

The screen shows each over in a list: the sender's callsign at the left of the first line, then the text. The two stations show in two colors. Text that you have not been shown yet shows as a row of dashes.

### CHEW

CHEW SHOWS in the DRILLS section of the settings sets when text shows:

- WHOLE OVER: each over shows after it ends.
- EACH LINE: each line shows after it ends.

The footer repeats this choice.

### QUIZ

After an over that holds a fact, the screen asks one question, such as:

`SENDER'S CALL?`, `RST?`, `NAME?`, `QTH CITY?`, `RIG?`, `POWER IN WATTS?`, `ANTENNA?`, `WX?`, `TEMPERATURE?`, `YEARS LICENSED?`, `AGE?` or `JOB?`

1. The status shows `YOUR TURN`. Start to key the answer within 10 seconds.
2. While you key, the status shows `KEYING`. When you stop keying for three word gaps, your answer ends.
3. The device shows the answer as sent, your copy with each character marked, and the score. The text of the over shows too. After 2.5 seconds the next over plays.

If you do not start to key within 10 seconds, the answer counts as missed. To hear the over again, key `?` alone as your answer. The sign off overs have no question.

The footer shows your score for this session.

## Controls

| Control | Action |
| - | - |
| F1 (SET) | Pauses and opens the settings at the DRILLS section. F4 (BACK) there returns, and the current over plays again at the new settings. |
| F2 (AGN) | While an over plays or waits for your answer: sends the over again from its start. Blank while paused. |
| F3 (STOP or GO) | Pauses, or starts again from the start of the current over. |
| F4 (BACK) | Pauses and opens the ON AIR list of the [menu](menu.md). Click CHEW or QUIZ, or RESUME, to go on from the start of the current over. |
| Dial turn | Changes the focused header value. See [Header values](controls.md#header-values). |
| Dial click | Moves the focus to the next header value. |
| Dial hold | Turns the device off. |
| Key | QUIZ: keys your answer. CHEW: nothing more. |

Quiz scores also go to [Letters](letters.md) and [Progress](progress.md).
