# Memories

The device stores four messages, M1 to M4. You key each message in once in the MEMORIES section of the settings. Then you send it with one press from the KEYER screen. Hold the button instead to send the memory again and again, for calling CQ until a station answers.

The first time PocketCW starts, the memories hold messages for a POTA activation with your callsign:

| Memory | Message |
| - | - |
| M1 | `CQ POTA DE <your call> <your call> K` |
| M2 | `UR 5NN 5NN <BK>` |
| M3 | `TU 72 E E` |
| M4 | `QRZ DE <your call> K` |

## Send a memory

1. On the KEYER screen, press F2 (MEM). The memory row replaces the decode lines and shows each memory. The labels of F1 to F4 change to M1 to M4, and the status line says `HOLD REPEATS  CLICK CLOSES`.
2. Press F1 to F4 to send M1 to M4 once. Hold the button to send the memory again and again. An empty memory sends nothing, and the status line shows `M1 IS EMPTY (SETTINGS)`.

The memory sends at your keyer speed and tone. The status line shows `SENDING M1  KEY TO STOP`. A held memory waits for the REPEAT EVERY time after it ends, then sends again. The status line then shows the time to the next send, such as `M1 AGAIN IN 6S  KEY TO STOP`.

With F1 HOLD set to MEM ROW, holding F1 on any screen opens the KEYER screen with the memory row open. See [Held buttons](controls.md#held-buttons).

A memory keys the func jack only when KEY OUT is ON and OUT TYPE is STRAIGHT. With OUT TYPE PADDLE, the memory plays only as a tone, and the status line says `M1 TONE ONLY: OUT IS PADDLE`.

### Stop a memory

Touch your key, press any F button, or leave the KEYER screen. While the memory row is closed, a dial click also stops the memory. An F button stops it as the button goes down, and does nothing else: press again to send a memory. A dial turn does not stop it.

### Controls with the memory row open

| Control | Action |
| - | - |
| F1 to F4 (M1 to M4) | Sends that memory once. With AFTER SEND set to CLOSE, the memory row closes. While a memory sends, the press only stops it. |
| F1 to F4 hold (REPEAT) | Sends that memory again and again, until you stop it. |
| Dial turn | Changes the marked value on the KEYER screen. |
| Dial click | Closes the memory row. While a memory sends, the click stops it instead. |
| Dial hold | Turns the device off. |
| Key | Stops a memory that is sending. With AFTER SEND set to CLOSE, the memory row closes. |

## Edit the memories

Open the settings, and press F3 (NEXT) until the MEMORIES section shows. The section has six rows: M1 to M4, REPEAT EVERY and AFTER SEND. See [MEMORIES](settings.md#memories) for their values.

To edit a memory, select its row and click the dial. Then key the message. Click again to end. While you edit, the line under the list shows how many characters are left.

| Control while you edit a memory | Action |
| - | - |
| F2 (DEL) | Deletes the last character. A prosign goes as one character. |
| F3 hold (CLEAR) | Clears the memory. |
| F4 (BACK) | Leaves the settings. |
| Dial turn | Nothing. |
| Dial click | Ends editing. |
| Dial hold | Turns the device off. |
| Key | Adds what you key to the memory. A prosign goes in as `<AR>`. The memory gets one space between words, and no space at the start. When the memory is full, the status line shows `MEMORY FULL`. |

REPEAT EVERY and AFTER SEND work as the other value rows do: click the dial, turn it to change the value, and click again.

The device saves the memories when you leave the settings screen.
