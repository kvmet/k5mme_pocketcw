# Memories

The device stores four messages, M1 to M4. You key each message in once on the MEM tab of the settings. Then you send it with one press from the KEYER screen. A memory can repeat, for calling CQ until a station answers.

The first time PocketCW starts, the memories hold messages for a POTA activation with your callsign:

| Memory | Message |
| - | - |
| M1 | `CQ POTA DE <your call> <your call> K`, set to repeat |
| M2 | `UR 5NN 5NN <BK>` |
| M3 | `TU 72 E E` |
| M4 | `QRZ DE <your call> K` |

## Send a memory

1. On the KEYER screen, press F2 (MEM). The memory row replaces the decode lines and shows each memory. A lit label marks a memory that repeats. The labels of F1 to F4 change to M1 to M4.
2. Press F1 to F4 to send M1 to M4. An empty memory sends nothing, and the status line shows `M1 IS EMPTY (SETTINGS, MEM)`.

The memory sends at your keyer speed and tone. The status line shows `SENDING M1  KEY TO STOP`. A repeating memory waits for the REPEAT EVERY time after it ends, then sends again. The status line then shows the time to the next send, such as `M1 AGAIN IN 6S  KEY TO STOP`.

A memory keys the func jack only when KEY OUT is ON and OUT TYPE is STRAIGHT. With OUT TYPE PADDLE, the memory plays only as a tone, and the status line says `M1 TONE ONLY: OUT IS PADDLE`.

### Stop a memory

Touch your key, press any F button, or leave the KEYER screen. While the memory row is closed, a dial click also stops the memory. An F button stops it as the button goes down, and does nothing else: press again to send a memory. A dial turn does not stop it.

### Controls with the memory row open

| Control | Action |
| - | - |
| F1 to F4 (M1 to M4) | Sends that memory. With AFTER SEND set to CLOSE, the memory row closes. While a memory sends, the press only stops it. |
| Dial turn | Changes the marked value on the KEYER screen. |
| Dial click | Closes the memory row. While a memory sends, the click stops it instead. |
| Dial hold | Turns the device off. |
| Key | Stops a memory that is sending. With AFTER SEND set to CLOSE, the memory row closes. |

## Edit the memories

Open the settings, and press F1 (TAB) until the MEM tab shows. The tab has six rows: M1 to M4, REPEAT EVERY and AFTER SEND.

| Row | Values |
| - | - |
| M1 to M4 | The message, up to 48 characters. A prosign such as `<AR>` takes 4. `RPT` marks a memory that repeats. |
| REPEAT EVERY | The silence between two sends of a repeating memory: 2 to 30 seconds. The default is 6 seconds. |
| AFTER SEND | CLOSE closes the memory row after a send. STAY OPEN keeps the memory row open, for one QSO after another. |

A hint line under the rows tells you how to use the selected row. On a memory row, it shows how many characters are left.

### Controls on a memory row

| Control | Action |
| - | - |
| F1 (TAB) | Opens the next tab. |
| F2 (DEL) | Deletes the last character. A prosign goes as one character. |
| F3 (CLR) | Clears the memory. |
| F4 (BACK) | Leaves the settings. |
| Dial turn | Selects another row. |
| Dial click | Turns repeat on or off for this memory. |
| Dial hold | Turns the device off. |
| Key | Adds what you key to the memory. A prosign goes in as `<AR>`. The memory gets one space between words, and no space at the start. When the memory is full, the status line shows `MEMORY FULL`. |

### Controls on REPEAT EVERY and AFTER SEND

| Control | Action |
| - | - |
| F1 (TAB) | Opens the next tab. |
| F2 (-), F3 (+) | REPEAT EVERY: one second less or more. AFTER SEND: changes the value. |
| F4 (BACK) | Leaves the settings. |
| Dial turn | Selects another row. While you edit REPEAT EVERY, the dial changes the value. |
| Dial click | REPEAT EVERY: starts or ends editing. AFTER SEND: changes the value. |
| Dial hold | Turns the device off. |
| Key | Nothing more. |

The device saves the memories when you leave the settings screen.
