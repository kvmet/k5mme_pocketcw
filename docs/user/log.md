# Log

With LOG on, the device records what goes out on the air while it sits between your key and your radio. It keeps a raw transcript, not a list of contacts: a computer finds the contacts later.

## Turn it on

Set LOG to ON on the KEYER tab of the [settings](settings.md). It is OFF at first.

## What the log records

| Source | What | When |
| - | - | - |
| YOU | What the device decodes from your keying, including text that `SEND` or a WinKeyer program keys. | LOG and KEY OUT are ON. |
| MEMORY | The text of each [memory](memories.md) you send. | LOG and KEY OUT are ON, and OUT TYPE is STRAIGHT. |
| THEM | What the decoder board of the Pro model hears. | LOG and DECODER BOARD are ON. |

Text from one source goes in one run until that source is quiet for 4 seconds, about one over. A memory is logged whole when it ends, even when a touch of your key stops it early.

Each power-on is one session. A session gets its number, your callsign and the firmware version when its first run is written. A power-on with nothing logged writes nothing. Times are seconds since power-on: the device has no clock.

The log holds about 2 MB, hundreds of activations. When it is full, the oldest records go first. Turning the device off, or an empty battery, writes the runs that are still open.

## Save the log on a computer

1. Set USB OUT to SERIAL, and plug the device into the computer.
2. Run:

```bash
python3 -I tools/pocketcw.py log log.txt
```

`log.txt` then holds one transcript per session:

```
SESSION 3  K5MME  v0.3.0
     12.4  MEMORY  CQ POTA DE K5MME K5MME K
     20.1  YOU     W1AW 5NN
```

Text from the last 4 seconds, and a memory still playing, are not in the file yet. The log stays on the device after you save it.
