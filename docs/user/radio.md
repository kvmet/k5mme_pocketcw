# Radio

The RADIO screen shows Morse code the device hears, as text: from the decoder board of the Pro model, which listens to the audio of a radio, or from a computer that sends it as [MIDI notes](usb.md#midi), such as Vail.

Open it from the ON AIR list of the menu, or hold F3 on the KEYER screen. The menu shows RADIO while DECODER BOARD is ON or USB OUT is not OFF.

No one has tried PocketCW on a Pro model yet. The board parts of this screen follow the vendor firmware, but they are not tested on a board.

## Turn the decoder board on

1. Open the settings, and go to the DEVICE section. See [Settings](settings.md).
2. Set DECODER BOARD to ON. The device powers the decoder board.
3. The ON AIR list of the menu now shows the RADIO row. Open it.

On a model without a decoder board, keep DECODER BOARD OFF. The setting is OFF by default.

Connect the radio's audio to the AUX1 port of the decoder board. Headphones or a powered speaker go in AUX2. The VOLUME setting also sets the volume of AUX2.

## What the screen shows

The top line shows the state of the board, or `MIDI IN` while DECODER BOARD is OFF:

| Item | Meaning |
| - | - |
| MIDI IN | DECODER BOARD is OFF, so the text comes only from MIDI notes. |
| BOARD OK or NO REPLY | BOARD OK: the board announced itself after it got power. NO REPLY: the board did not announce itself yet. |
| FIL n | The filter setting of the board, 0 to 2. |
| NR n | The noise reduction level of the board, 0 to 6. |
| nnWPM | The speed of the Morse code the device hears. Two dashes stand in for the number before the first speed reading. |
| ERR n | Data errors on the link to the board. This item shows only after an error. |

Under the top line, nine lines show the text the device decodes.

The device decodes only while the RADIO screen is open, or on every screen while USB OUT is RADIO. With the board on and MIDI notes coming in, both go to the same lines. See [USB keyboard](usb.md#radio). Each time decoding starts, the decoder starts at 20 WPM and then follows the speed it hears, from 5 to 60 WPM. The text stays when you leave the screen.

## Controls

| Control | Action |
| - | - |
| F1 (SET) | Opens the settings at the DEVICE section. F4 (BACK) there returns to this screen. |
| F2 (FILT) | Changes the filter setting of the board: 0, 1, 2, then 0 again. Only while DECODER BOARD is ON. |
| F3 (NR) | Changes the noise reduction level of the board: 0 to 6, then 0 again. Only while DECODER BOARD is ON. |
| F4 (BACK) | Opens the ON AIR list of the [menu](menu.md). |
| Dial turn | Changes the focused header value. See [Header values](controls.md#header-values). |
| Dial click | Moves the focus to the next header value. |
| Dial hold | Turns the device off. |
| Key | Nothing more. |
