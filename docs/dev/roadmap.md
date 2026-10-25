# Roadmap

The planned work after USB: the SPI flash, copy training, two-way QSOs, and an in-line logger. Order of work:

1. Probes: SPI flash, bootloader erase, vendor log, voice quality
2. SPI flash driver and log location
3. Content pack
4. Morse then voice, and the head-copy ladder
5. Two-way ragchew bot: deferred
6. In-line logger: deferred until after MIDI, Vail and F key long presses
7. Stats export: built
8. More QSO kinds: P2P and S2S callers, short contest exchanges
9. Later: send drill, keyer extras, games
10. Revisit at the end: F key long presses, when the serial port is up

## Ground facts

- Storage, and what each holds:

  | Store | Size | Free | Holds |
  | - | - | - | - |
  | MCU flash | 241,664 bytes | 30.7 KiB at `opt-level = "z"` (image 210,260 bytes on 2026-10-25) | Code, and small tables the core features need without a pack. Code cannot run from the SPI flash: it sits on SPI1, not a memory-mapped interface. |
  | SPI flash (F files) | 16 MB | About 13 MB outside the vendor regions | Content pack, voice packs, log. |
  | EEPROM (I2C) | 2 KB | 64 bytes (0x7C0 to 0x7FF) | Settings, letter stats, progress, memories. Too small for the log; the plan only appends the LOG setting to the settings record. |
  | RAM | 64 KB | About 56 KB | Buffers, such as the 16 KB voice clip buffer. |

- `opt-level = "z"` is adopted: the app ran fine on hardware. The worst-case synth sample costs 1900 of 4096 cycles at `"z"` and 1400 at `"s"`, measured on board 2.64.
- USB MIDI is planned next, and will take MCU flash.
- SPI flash: 16 MB GigaDevice GD25Q128 (JEDEC C8 40 18), on SPI1 with the LCD (`LCD_CS` PB6, `FLASH_CS` PB9). No write protection is set. SFDP 1.0 with 9 dwords lists 4 KB (0x20), 32 KB (0x52) and 64 KB (0xD8) erase, but no erase times and no page size.
- SPI flash in use, by scan: 0x000000 to 0x025FFF (boot screen), 0x030000 to 0x0C5FFF (UI images), 0x100000 to about 0x8FC000 (the content pack `F000400.bin`), 0xE00000 to 0xFECFFF (fonts). Free: 0x0C6000 to 0x0FFFFF and 0x8FC000 to 0xDFFFFF. The stock firmware has a contact log whose location is not known, so a pack install may erase it.
- Install limit: `pocketcw-flash` refuses a file that needs more than one FAT sector (`tools/pocketcw-flash.c:250`): 1022 clusters. The update drive has 8 KB clusters, so 8,372,224 bytes. An 8.37 MB install is tested.
- An install erases and writes only the 4 KB sectors its file covers. Data in the next sector is kept.
- No RTC (PC14 and PC15 carry the dial) and no audio input (no I2S lines to the MCU).
- Task model: `sample` at priority 4, `tick` at 3, `ui` at 1. The `audio` resource is shared by `tick` and `ui`, so its lock masks the 1 ms tick. The event queue to `ui` is 64 deep.
- The serial port is in main: text commands, WinKeyer mode and the VBand CW Hotline (`docs/dev/serial.md`). The port is up while USB OUT is anything but OFF; USB OUT SERIAL gives the port alone, with no typing.
- Logic goes in `logic/` with host tests; `firmware/` stays thin. See [Code layout](code-layout.md).

## 1. Probes

All four go in probe images. None needs the flash driver in the main image. Each has a pass rule that can fail.

### SPI flash

- JEDEC ID (0x9F), status registers 1 to 3 (0x05, 0x35, 0x15), SFDP (0x5A). Take erase sizes and timings from SFDP, not from a part name, since clone parts are common.
- Pass: SFDP reads, 4 KB sector erase is listed, and no protect bit (BP, TB, SEC, CMP, SRP) covers the free area.
- Read 0x100000 and compare it with `F000400.bin`, at 18 MHz and at 4.5 MHz. Pass: both match.
- Erase and program one 4 KB sector in the free area and read it back, timing the erase. Pass: the data matches, and the worst erase time is recorded.
- Program 16 partial writes into one 256 byte page, then read it back. The log appends small records into pages, and some parts limit partial programs per page. Pass: all 16 match.

Passed on 2026-10-25. Erase takes 15 ms on a blank sector and 45 ms on a programmed one; a page programs in at most 917 µs. See [Storage](hardware/storage.md#spi-flash).

### Bootloader erase

Done on 2026-10-25 with `firmware/src/bin/flashprobe.rs` and `tools/flashtest.py`. See [SPI flash probe](building.md#spi-flash-probe).

- The update drive has 8 KB clusters and 51,200 sectors of 2 KB. `pocketcw-flash` prints both.
- A 2,093,056 byte test file at 0x100000 installed in full: 511 tagged sectors, 0x100000 to 0x2FEFFF.
- Markers at the file's end, plus 4 KB, plus 64 KB, plus 1 MB, and at 0xC00000 and 0xDFF000 all survived.
- Result: an install touches only the sectors its file covers. The log range is safe.

### Vendor log

- On a unit that has run stock firmware, scan 0x0C5080 to 0xDFFFFF for bytes that are not 0xFF. That shows where the vendor contact log lives, so the PocketCW log does not overlap it and a return to stock does not wipe it.
- Deferred until a Pro unit is available. The 2.64 unit has no stock contact log, and its free area reads blank.

### Voice quality

Done on 2026-10-25 with `firmware/src/bin/voiceprobe.rs`. See [Voice probe](building.md#voice-probe).

- Three candidate voices: Kokoro-82M `af_heart`, Piper LJSpeech, and espeak-ng, one probe image each. Each plays 20 words at 8789 Hz and 20 at 5859 Hz.
- Kokoro was clear. Piper had a better tone but cut off word endings ("egg" sounded like "ek"). espeak-ng sounded least human.
- On the speaker, 8789 Hz and 5859 Hz sounded the same. Headphones are not checked yet.
- Result: Kokoro `af_heart` at 5859 Hz.
- Then, with the content pack on the speaker, 5859 Hz lost the ends of letters: QTH sounded like QT8. A listening pack (`tools/pack/listenpack.py`) compared 5859, 8789 and 17,578 Hz. 8789 Hz with a treble boost was clear, and 17,578 Hz best. The pack moved to 8789 Hz for every clip, since one rate for letters and another for words would be jarring. The same tests found foreign vowels from the phoneme form, and rushed lone letters; see [Content pack](building.md#content-pack).

## 2. SPI flash driver and log location

### Bus sharing

`Lcd` owns `pac::SPI1` today. Split it into a `Bus` that owns SPI1 and both chip selects, with `Lcd` and `Flash` as users. All flash access runs in the `ui` task, which already owns every LCD draw, so no lock is needed. `read_row` changes the SPI divider and the MISO mode (`lcd.rs`), so the flash driver sets both on every transaction.

### Erase without blocking

- Only 4 KB sector erase. No block or chip erase.
- The bus is free while the chip erases. The driver starts an erase and returns. Every later command but a status read first waits while the chip is busy, so other `ui` work keeps running, and a content pack read during an erase waits for it. Built.
- The log erases its next sector ahead, when the head sector is half full and the key is idle. A record write is then only page programs of a few ms.

### Layout

Only the log is persisted user data, so only its location is fixed now: 0xC00000 to 0xDFFFFF (2 MB), the top of the free area. 0xBF0000 to 0xBFFFFF is a guard that nothing writes. The step 1 probes showed the range blank and untouched by installs.

The packs grow up from 0x100000 and end below the guard. Installs rewrite them and they ship with matching firmware, so each release can move or resize them. Each pack file is 8,372,224 bytes or less, at a 64 KB aligned offset (`F000400.bin` at 0x100000, `F000C00.bin` at 0x300000, and so on).

The logic crate holds the layout and an `allowed(address, len)` check (`logic/src/log.rs`). The driver refuses any erase or program outside the log range. Host tests cover the check. Built.

Flash cost: about 3 KB.

## 3. Content pack

`F000400.bin`, built by a Python tool from text in the repo. Format 2 of the existing header (`PocketCW`, format byte):

- Header: magic, format version, a table of sections (kind, offset, length, CRC32).
- Sections are added as the features that use them arrive: word lists by frequency first, for the head-copy ladder.

The device parses the header in `logic`, with host tests on packs the tool builds. Features that need a missing section show `CONTENT PACK MISSING: INSTALL IT`; the rest of the firmware works.

The core bots must work without a pack. Their tables stay in MCU flash unless the flash budget forces a move; then a small fallback table stays.

Flash cost: about 1 KB.

## 4. Morse then voice, and the head-copy ladder

### What it does

- NINJA drill: the device sends a word in Morse, waits, then speaks it. No keying needed. Key the word before the voice, and it is scored like the other drills. Built as a fourth receive drill, with single words from the content pack; phrases are not built.
- Head-copy ladder: words get longer as you copy them, then two words, then short sentences. You key back at the end. Built as a fifth receive drill, HEAD, with QSO exchange lines as the sentences.
- Planned for HEAD: save the rung across power off, as an appended field of the progress record.
- Maybe for HEAD: the voice says the item after your copy, as in NINJA. The pack would first need clips for every word of the QSO lines (names, cities, rigs, weather), which is a pack change.

### Voice clips

- Voice: Kokoro-82M `af_heart`, built on a computer with `tools/pack/voiceclips.py`. The probe picked it among three, each with a license that allows redistribution of its output:

  | Voice | License |
  | - | - |
  | Kokoro-82M | Apache 2.0 weights, trained on permissive or public domain audio. The CC BY data in it covers Japanese and French voices; `af_heart` is American English. |
  | Piper LJSpeech | Public domain dataset. Other Piper voices carry their own licenses and are not candidates. |
  | espeak-ng | GPL-3.0. Robotic, but formant speech can stay clear at low sample rates. |

- `tools/release.sh` appends the content pack's credits, `tools/pack/pack-notice.txt`, to `THIRD-PARTY.txt`.
- Format: 4-bit IMA ADPCM at 8789 Hz (17578 / 2), about 4.4 KB a second. Playback draws two output samples per input on straight lines (`logic/src/adpcm.rs`), so no fractional resampler is needed. The pack header gives the rate, so a pack at another rate needs no firmware change.
- Size: a pack file is 8,372,224 bytes or less, about 1900 s of speech. `tools/pack/pack.py` leaves out the least common words until the pack fits. An 8.37 MB pack installed and played its first and last clips.
- Built: `tools/pack/audition.py` plays every clip as the device plays it, so a listener can mark the bad ones without the device and fix them with a respelling or phonemes. Marks and fixes are kept per voice, in `tools/pack/voices`. See [Reviewing the clips](building.md#reviewing-the-clips).

### Playback

- The `ui` task reads a whole clip into a `ui`-owned 16 KB buffer, outside every lock. 16 KB holds about 3.7 s at 8789 Hz. The clip builder fails any clip longer than the buffer, so the limit is checked at build time.
- It hands the buffer to the `sample` task through a single-producer, single-consumer index: no copy under the `audio` lock.
- The `sample` task decodes ADPCM and plays it as one more voice in `synth`. The cost is tens of cycles a sample against 4096 available.
- Built: `SAY` on the serial port speaks text. Words in the pack play their clip; one abbreviation on its own also gets its meaning. Other words are spelled from character clips (`tools/pack/words/chars.txt`), up to 32 clips in the 16 KB buffer. Clips play in turn with 120 ms between them.

Flash cost: about 4 KB.

## 5. Two-way ragchew bot

Deferred on 2026-10-25. The POTA and SOTA modes already give two-way practice, and its 6 to 10 KB of flash goes to Vail and MIDI support instead.

### What it does

You are one of the two stations. The bot calls CQ or answers yours. The QSO runs as on the air: reports, names, QTH, rig, antenna, weather, then 73. The bot answers what you send: it sends your report back, uses your name, asks `AGN?` about a field it could not read, and answers `QRS` and `?`.

### Design

- A new `logic/src/chat.rs`, next to `ragchew.rs` (which keeps the listen-only kinds), built on its `Station` data. Answers longer than 48 characters split into lines as `ragchew.rs` already does.
- Your overs are longer than the current buffer: the over buffer is 48 characters and the parsers take 16 words. Chat needs its own over buffer of about 300 characters.
- Over end: ragchew senders pause mid-over, so three word gaps (`decoder.rs` `COPY_END_UNITS`) would cut them off. Chat ends an over on `K`, `KN`, `<BK>`, `<AR>` or `<SK>` after `DE <call>`, and on a long silence as fallback.
- Parser: your name and QTH are unknown values, so tolerance applies to the keywords (`NAME`, `OP`, `QTH`, `UR`, `RST`, `RIG`, `ANT`, `WX`) and to the bot's own fields. The token after a keyword is taken as given. When a doubled token disagrees (`BOB BOP`), the bot asks `AGN?`.
- Scoring: copy is scored with the existing `Ragchew::question` after the QSO, so you send what you would on the air. Scores go to Letters and Progress.

### Tests

Host tests run scripted QSOs: a clean one, one with an error in each field, one with `AGN`, `QRS` and `?`, and one with mid-over pauses. On hardware, the serial `SEND` command keys a whole QSO.

Flash cost: 6 to 10 KB, the largest item.

## 6. In-line logger

Deferred on 2026-10-25, after MIDI, Vail and the F key long presses: long presses can give note mode and the LOG screen a place outside the menu. Built and kept: the log ring and format (`logic/src/log.rs`), and the flash write guard (`firmware/src/spi_flash.rs`). Built and taken out: run capture, the LOG setting, `DUMP LOG` and `pocketcw.py log`, in commit d920c55; commit 3bb0d26 takes them out, and a revert of 3bb0d26 puts them back.

Open questions for when it returns:

- Note mode and the LOG screen need a place: the menu is full at 11 rows.
- A run ends after 4 s with no new text. Check this against real sending.
- A memory is logged whole at the end of its playback, and only from STRAIGHT output with KEY OUT on.
- On a Pro, check whether the decoder board hears your own sidetone. If it does, your keying logs twice, as YOU and as THEM.
- A memory's run ends when the "playback ended" message arrives. A full event queue can drop that message, and the run then stays open until the next memory or power off.

The device sits between your key and the radio with KEY OUT on, and records a raw transcript. The device does no QSO detection. `tools/pocketcw.py` finds QSOs in the exported transcript.

### Why detection runs on the computer

- Python has no flash budget, no 48 character buffers, and full regex and fuzzy matching.
- A better rule re-reads every transcript ever recorded, with no firmware release.
- Real exported transcripts are the test files, and a test run takes seconds.
- On-device QSO records can come later as a new record kind. The log walker skips kinds it does not know, so that is an addition, not a migration.

What the device loses: a live contact count in the field ("do I have my 10?"). Accepted for now. A later on-device count is an addition.

### Turning it on

- LOG on the KEYER tab: ON or OFF, default OFF. Bit 4 of the settings record's hardware flags byte, which older firmware ignores, so the record length stays the same.
- While LOG is ON with the decoder board on, the decoder board listener also runs (Pro only). Today it runs only on the RADIO screen or with USB OUT RADIO.

### What goes in the transcript

- YOU: what the decoder reads from your keying.
- MEMORY: the text of each memory that keys the radio. Memory playback does not pass through the decoder, so the text is logged directly.
- NOTE: text keyed in note mode. The key writes to the log and KEY OUT stays off. A hunter on a Mini never sends the activator's call, so a note is the only way it reaches the log. Note mode shows on the screen while it is on, and ends after one note, so it cannot silently stop transmitting. Note mode starts from a menu row, since the KEYER F keys are taken. F key long presses are on the revisit list.
- THEM (Pro only): the decoder board text. The Pro is untested.

Text is buffered in RAM and written once per over or note, not once per character. Power off, including the battery-empty path, writes the buffers first.

### Time

No clock and no time entry. A session is one power-on. Times are milliseconds since boot (u32, 49 days). `pocketcw.py` takes an optional start time per session to fill QSO_DATE and TIME_ON in ADIF. POTA upload needs them.

### Log format

Persisted, so decided now.

Ring:

- One ring in the log range, as a run of 4 KB sectors. A record never crosses a sector.
- Every sector starts with a header record, written when the sector is first used. A sector with no valid header is erased again before use, since a torn erase can read as 0xFF and still be weak. A sector erased ahead and not yet used has no header, so after a power-off it is erased again; that costs one erase.
- The sector header stays version 1 forever, and its fields only append. Older firmware erases a sector whose header it cannot read, so a new header version would wipe the log on a downgrade.

Records use the `logic/src/record.rs` frame: magic (2 bytes), version, body length (1 byte), body, CRC16. The magic is `L` and a kind letter, so its first byte is never 0xFF.

| Kind | Magic | Body, version 1 |
| - | - | - |
| Sector header | `LH` | sequence u32, session u32 (the session open when the sector started) |
| Session start | `LS` | session u32, start tick u32, firmware version (major, minor, patch: 3 bytes), callsign length u8, callsign (up to 10) |
| Transcript run | `LT` | source u8 (YOU 0, MEMORY 1, NOTE 2, THEM 3), run id u16 (counted per session), part u8 (bit 7 set: more parts follow), start tick u32, end tick u32, text length u8, text (up to 242) |

Numbers are little-endian. Text is ASCII as the screen shows it: a prosign as its letters in angle brackets (`<AR>`), an unknown character as `*`, a word gap as one space.

Runs and parts:

- A run is one over or note of one source. It ends after 4 s with no new text from that source.
- A run longer than 242 characters goes in parts. A part is written with bit 7 set only once the next character arrives, so the bit never promises a part that does not come. A reader still accepts a last part with bit 7 set, since power loss can cut a run.
- Parts join by plain concatenation. Each part's start and end ticks give that part's own span. A run ends at part 127, and the next character starts a new run.
- The session start record is written before the first run of a power-on, so a power-on with nothing logged writes nothing. Its start tick is the time it was written.

Fixed fields come first and text last, behind its own length, so later versions can append fields after the text. The session-start record carries your callsign for ADIF STATION_CALLSIGN, since it can change between sessions. Start and end ticks order YOU and THEM runs that overlap.

Walking a sector:

- First byte 0xFF: end of data.
- A known kind whose body is too short for its fields or text length: skipped.
- Length past the sector, or a bad CRC: torn. The sector is closed and writing goes on in the next one.
- Valid frame of an unknown kind or version: skipped. `record::unframe` cannot be used here, since it rejects every other magic and version as if corrupt.

Startup:

- The head is the sector with the highest sequence. The next session number is one more than the highest session in the head sector.
- With no valid header anywhere (the first boot), the first write goes to the first sector, with sequence 1 and session 1.
- A torn record in the head closes it, so the next write goes to the next sector.
- The session counter lives in the log, not in the EEPROM boot count, which does not advance when its write fails.

Wrap:

- When the ring wraps, the oldest sector goes. A 2 MB ring holds hundreds of activations.
- The oldest surviving sector can start mid-session and mid-run. Its header gives the session. A reader drops the parts of a run whose part 0 is gone.

### On the device

- `DUMP LOG` on the serial port sends the raw records. Set USB OUT to SERIAL to export.
- LOG screen: a transcript viewer. Scroll back through past sessions by session number, with each run marked by its source.

### On the computer

- `pocketcw.py log` reads the dump and writes a text transcript per session.
- `pocketcw.py adif` proposes QSOs from the transcript and writes ADIF. Activating: your `<call> 5NN`, then `TU <state> 72`. Hunting: your call sent to an activator, then a NOTE with its call. QSOs it is not sure of are marked for you to check.

### Tests

- Device, on host: wrap, torn record, torn erase, a future-kind record between two known ones (both known records read), a wrap where the oldest sector begins with part 2 of a run, session start replay, next session number at startup.
- Device, on hardware: power off during a write, then check the log reads back. Battery-empty power off flushes the buffers.
- Computer: QSO detection runs only after you record real activations. Those transcripts, with the QSOs you logged by hand, are the test fixtures.

Flash cost: 4 to 5 KB (ring, `DUMP LOG`, note mode, LOG screen).

## 7. Stats export

Built: `STATS LETTERS`, `STATS PROGRESS` and `STATS TIMING` on the serial port, and `pocketcw.py stats` saves each as CSV. See [Stats](serial.md#stats). Timing statistics are not saved across power off, and the EEPROM has 64 bytes left, so a saved version would go in the log as a new kind.

Flash cost: about 1 KB.

## 8. More QSO kinds

### P2P and S2S callers

Park to park (P2P) and summit to summit (S2S) contacts are common. Today only Hunt takes a P2P call from you, and no bot sends a reference.

- Activate: some hunters are at a park or on a summit. Such a hunter sends `P2P` or `S2S` and its reference in the exchange, such as `RR TU 559 CA CA P2P US-1234 72` or `S2S W7O/CN-001`. Confirm the current POTA reference format before building: POTA moved from `K-1234` to country codes such as `US-1234`.
- Hunt: from a park or summit you send `P2P` or `S2S` and your reference; the activator answers with its own.
- Copy check: the contact counts as copied when your closing holds the other station's reference.
- Tables: a few dozen park and summit references per call area, chosen to match the hunter's state.

Flash cost: 1 to 2 KB.

### Short contest exchanges

Contests are short, fixed exchanges at speed: no 72 or 73, often only `TU` and your call to close. Add them as programs in Activate, beside POTA and SOTA: you call `CQ TEST`, the bot sends a pileup, and each caller sends one exchange.

| Contest | Exchange | Example |
| - | - | - |
| CWT | Name and member number, or state for a non-member | `BOB 1234`, `BOB TX` |
| SST | Name and state | `BOB TX` |
| SKCC sprint | RST, state, name and member number | `579 TX BOB 12345T` |
| Field Day | Class and ARRL section | `2A STX` |

- Copy check: you key the exchange you copied, as Hunt logs a callsign. Each field scores apart.
- Names and states come from the ragchew and hunter tables. New tables: member numbers (made at random), Field Day classes and ARRL sections.

Flash cost: 2 to 3 KB.

## Flash budget

| Item | Estimate |
| - | - |
| SPI flash driver | 3 KB |
| Content pack parser | 1 KB |
| Voice and head copy | 4 KB |
| Two-way ragchew (deferred) | 6 to 10 KB |
| Logger, LOG screen, `DUMP LOG` (deferred) | 4 to 5 KB; capture and `DUMP LOG` measured 5.6 KB |
| Stats export | 1 KB |
| P2P and S2S callers | 1 to 2 KB |
| Short contest exchanges | 2 to 3 KB |
| Total, without the deferred items | 12 to 14 KB |

The plan needs 12 to 14 KB of 30.7 KiB free at `"z"`, before Vail and MIDI. More ways to get flash back:

- Float formatting: about 10 KB of `core::fmt` float code (grisu, dragon, `float_to_decimal_common_exact`), from `{:.1}` in `ui/tape.rs:156` and `ui/home.rs:234` and at least one shortest-form float print. Fixed-point integer formatting removes it.
- Move fixed text tables (ragchew names, cities, rigs; bot phrases) into the content pack, with a small fallback in MCU flash so the bots still work without a pack.
- Look at the largest symbols (`cargo bloat` or `nm --size-sort`) for fonts, formatting code and panics.

If flash stays short, set a budget per feature: which features ship first.

## Revisit at the end

1. F key long presses: a second action per F key, so controls like note mode need not live in a menu.
2. When the serial port is up: today only while USB OUT is not OFF. Option: up whenever USB is plugged in, with USB OUT choosing only what types.
