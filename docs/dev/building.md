# Build and flash

## What you need

- Rust from [rustup](https://rustup.rs), version 1.88 or later. The code uses let chains, from the 2024 edition.
- The target for the AT32F403A, a Cortex-M4F:

  ```bash
  rustup target add thumbv7em-none-eabihf
  ```

- The LLVM tools. `mkimage.sh` takes `llvm-objcopy` and `llvm-objdump` from the rustup toolchain folder.

  ```bash
  rustup component add llvm-tools
  ```

- To flash from macOS: `clang` from the Xcode command line tools.

## Build the image

```bash
sh tools/mkimage.sh
```

The script builds `firmware/` in release mode and writes `firmware/target/A000014.bin`. `firmware/.cargo/config.toml` sets the target, so a plain `cargo build` in `firmware/` also builds for the device.

The script stops with an error in two cases:

- The image does not start at 0x08005000. The bootloader is below that address. See [Flash layout](hardware/flash-layout.md).
- The image is larger than 236 KB, the app area.

At the end, the script prints the command that flashes the image.

The linker script `firmware/memory.x` puts the app at 0x08005000, with 236 KB of flash and 64 KB of RAM. `firmware/build.rs` passes the `nmagic` option to the linker. Without it, the linker maps the ELF headers at 0x08000000, inside the bootloader, and `mkimage.sh` refuses the image.

## Flash the image

The device installs updates through its bootloader, which shows a USB drive. macOS must not mount that drive. See [Flash layout](hardware/flash-layout.md) for the reason.

1. Build the install tool once. `mkimage.sh` prints a command that runs it from `tools/`:

   ```bash
   clang -O2 -arch arm64 -arch x86_64 -mmacosx-version-min=10.15 -o tools/pocketcw-flash tools/pocketcw-flash.c -framework DiskArbitration -framework CoreFoundation
   ```

2. Run the command that `mkimage.sh` printed:

   ```bash
   tools/pocketcw-flash firmware/target/A000014.bin
   ```

3. Follow the steps the tool prints. macOS asks for your Mac login password, because the tool writes to the raw disk through `authopen`.

`pocketcw-flash` refuses a file that would write below 0x08005000, or past the end of the 256 KB flash. The [install guide](../user/install.md) and [troubleshooting](../user/troubleshooting.md) pages cover the steps on the device.

## Test

The tests run on your computer:

```bash
cd logic
cargo test
```

`logic/` has no target setting, so it builds for your computer. The firmware crate has no tests.

## Hardware probe

`firmware/src/bin/probe.rs` is a separate image for checking a board. It shows the chip and board IDs, reads the LCD memory back, scans the I2C bus, shows the live inputs, and tests the audio codec, the PA8 audio output and standby.

```bash
sh tools/mkimage.sh probe
```

The probe image also goes to `firmware/target/A000014.bin`. Flash it the same way, and flash the app again after.

## USB probe

`firmware/src/bin/usbprobe.rs` is a separate image for checking the USB device. It shows up on the computer as a USB keyboard and serial port named "PocketCW USB probe".

```bash
sh tools/mkimage.sh usbprobe
```

| Input | Sends |
| - | - |
| Dit contact (tip) | Holds Left Ctrl, as VBand reads a dit. |
| Dah contact (ring) | Holds Right Ctrl, as VBand reads a dah. |
| F1 | Types `cq test de pocketcw` and Enter. |
| Hold the dial 1.5 s | Turns the device off. |

The screen shows the clock source, the USB state, the bus reset count, and the reports the computer took. CONFIGURED means the computer accepted the keyboard. macOS may open its Keyboard Setup Assistant the first time; close it.

The image goes to `firmware/target/A000014.bin`. Flash it the same way, and flash the app again after.

## Content pack

The content pack is `F000400.bin`: the words for the drills, each with a clip of the Kokoro voice `af_heart` at 8789 Hz. `logic/src/pack.rs` gives the format. The entries come from three lists:

- `tools/pack/words/chars.txt`: every character Morse sends, so the device can spell a word the pack lacks.
- `tools/pack/words/ham.txt`: CW words, abbreviations, Q codes and prosigns, each with the text the voice says and, for an abbreviation, its meaning. An abbreviation gets two clips: the spelled form, "Q T H", for running text, and the spelled form then the meaning, "Q T H. location", for a word on its own and for drills.
- `tools/pack/words/common.txt`: English words from the top 3177 of wordfreq. `tools/pack/wordlist.py` writes it, leaving out the words in `tools/pack/words/blocklist.txt`: offensive words, fragments, and words seldom sent in a CW contact. The blocklist is reviewed down to that rank, so raising the rank means reviewing the new words first. It needs the `wordfreq` package, and runs only when the list changes. The pack takes the most frequent of them that fit in one update file, 8,372,224 bytes.

`tools/pack/pack.py` builds the pack. It needs what the voice probe needs for Kokoro (see below). It keeps what Kokoro says for each text in `firmware/target/clip-cache/speech`, about 150 MB, and each finished clip beside it under a key of what makes it: `tools/pack/voiceclips.py`, the ffmpeg version and `CLIP_RECIPE` in `tools/pack/pack.py`. `cargo clean` deletes both. A rebuild speaks only new or changed text. A change to how clips are made, such as their level, makes every clip again from the saved speech in about 4 minutes. The first build speaks every clip, in about 25 minutes:

```bash
python tools/pack/pack.py --models MODELS --out firmware/target/pack
```

How each clip is made, in `tools/pack/voiceclips.py` and `tools/pack/pack.py`:

- Kokoro gets phonemes in the form of misaki, the tool its training data went through. espeak-ng makes the phonemes, and a table from misaki's espeak fallback converts them. Without the table, vowels sound foreign: "bee" comes out like "beay".
- A character ends in a period. Kokoro rushes a lone letter; as the end of a sentence it says it in full.
- Before encoding, a treble boost undoes the loss of the device's straight-line upsampling, 6 dB at the top of the band.
- Then the speech level is set to 7 dB under the sidetone's, from the power of its 20 ms frames without the pauses. A limiter keeps the peaks under 0.8 of full scale: ADPCM overshot dense peaks by up to 16% in a test. The synth plays 0.8 at the sidetone's peak (`CLIP_CEILING` in `logic/src/synth.rs`). A clip scaled by its peak alone sounds about 11 dB under the sidetone, since speech has short peaks far above its level. The build prints the level of the clips against the sidetone's.

`tools/pack/listenpack.py` builds a small pack that compares ways of making clips, to judge them on the device speaker. Each test word is made each way, as words such as `QTHA` and `QTHB`. The ways at lower rates go through what the device does to them, so the pack plays at 17,578 Hz with any firmware.

`--no-voice` builds the words without clips, in a second. `--fixture logic/fixtures/pack.bin` writes the small pack the logic tests read; build it again after a format change.

Install it, then speak a word over the serial port:

```bash
tools/pocketcw-flash firmware/target/pack/F000400.bin
```

```bash
python3 -I tools/pocketcw.py info say QTH say "hello K5MME 73"
```

`INFO` gives `PACK=` and the word count, or `NONE`, `OLD` or `BAD`. At startup the device checks the word list's CRC, and not the clips', which take too long to read.

## Voice probe

`firmware/src/bin/voiceprobe.rs` plays spoken words through PA8, so you can score by ear whether a voice is clear enough. Each image holds one voice: 20 words at 8789 Hz and 20 at 5859 Hz, the 17,578 Hz sample rate divided by 2 and 3. It also times the synth's heaviest load against the sample period, and shows it as `SYNTH worst/4096 MEAN n` in CPU cycles.

`tools/pack/voiceclips.py` builds the clips. It needs Python 3.12, since it encodes with `audioop`, which Python 3.13 removed. It also needs numpy, soundfile, ffmpeg and espeak-ng, and for the neural voices kokoro-onnx or piper-tts and their model files. Then build the image with the clip folder in `VOICE_CLIPS`:

```bash
python tools/pack/voiceclips.py kokoro --models MODELS --out firmware/target/voice-kokoro
```

```bash
VOICE_CLIPS=$PWD/firmware/target/voice-kokoro sh tools/mkimage.sh voiceprobe
```

Without `VOICE_CLIPS`, the image builds with no clips and says so.

| Control | Action |
| - | - |
| F1 | Plays the next word, with its text hidden. |
| F2 | Plays the word again. |
| F3 | Shows the word. |
| F1 or F4, with the word shown | Scores it right or wrong. |
| Dial click | Switches the rate. |
| Dial turn | Volume. |
| Hold the dial 1.5 s | Turns the device off. |

A rate passes at 19 of 20 right. Flash the app again after.

## SPI flash probe

`firmware/src/bin/flashprobe.rs` reads the 16 MB SPI flash beside the screen. At start it only reads: the JEDEC ID, the status registers and write protection, the SFDP table (size, page size, erase types and times), `F000400.bin` at 0x100000 at 18 MHz and at 4.5 MHz, and a scan of every 4 KB sector that lists the used ranges.

It erases and programs only in the log range, 0xC00000 to 0xDFFFFF, since the driver refuses writes anywhere else, and only sectors that read blank or that carry its own tags.

| Control | Action |
| - | - |
| F1 | Erase and program test in two blank sectors from 0xD00000: erase times, page program time, a page written in 16 pieces, read back, then erased again. |
| F2 | Writes a `PCWMARK` tag in each marker sector that reads blank: at each end of the log range. |
| F3 | Shows whether each marker is still there, then scans again. |
| F4, twice | Erases every sector in the log range that starts with a `PCWMARK` or `PCWTEST` tag. |
| Hold the dial 1.5 s | Turns the device off. |

The bootloader test shows what installing an F file erases past the file's end. `tools/flashtest.py` writes a 2 MB `F000400.bin` with every 4 KB sector tagged `PCWTEST`:

```bash
python tools/flashtest.py --out firmware/target/flashtest
```

```bash
sh tools/mkimage.sh flashprobe
```

1. Build and flash the probe. Read the scan, then press F1, then F2.
2. Flash `firmware/target/flashtest/F000400.bin`.
3. Turn the device on and press F3. The tags line counts the test sectors.
4. Press F4 twice, then flash `dist/F000400.bin` from `tools/release.sh` and the app.

## Remote control and screenshots

`tools/pocketcw.py` drives the device over USB serial. It presses the buttons, keys and plays text, shows what the device decodes and plays, reads and changes settings, saves the screen as a PNG, and saves the stats and the key timing as CSV. It needs Python 3 on macOS or Linux, with no other packages. [Serial interface](serial.md) describes the commands and the event lines.

1. Set USB OUT to SERIAL, so the device is on USB and types nothing.
2. Plug the device into the computer.
3. Run the steps, such as:

```bash
python3 -I tools/pocketcw.py F4 RIGHT CLICK shot timing.png
```

| Step | Does |
| - | - |
| `F1` to `F4`, `LEFT`, `RIGHT`, `CLICK` | Presses and releases that button. `LEFT` and `RIGHT` turn the dial one step. |
| `shot FILE.png` | Saves the screen. |
| `wait MS` | Pauses, for a screen that changes on its own. |
| `send TEXT` | Keys the text and waits until it is sent, then shows what the device decoded. Put text with spaces in quotes. |
| `expect REGEX` | Waits up to 60 s for an event line that matches from its start, such as `expect "PLAY "`. |
| `listen MS` | Shows the event lines that arrive in that time. |
| `shots DIR` | Saves every screen into the folder. See [Every screen](#every-screen). |
| `info` | Shows the firmware version, the screen and the panel. |
| `get NAME` | Shows a setting or memory, such as `get SPEED`. `get all` shows every one. |
| `set NAME VALUE` | Changes a setting or memory, written as the screen writes it, such as `set MODE "IAMBIC A"`. |
| `play TEXT` | Sounds the text as a receive drill does, and waits until it ends. |
| `stop` | Stops what `send` and `play` started. |
| `stats letters FILE.csv` | Saves each Koch character's tries, percent right, mastery, and whether it is in play. Shows the mixups. |
| `stats progress FILE.csv` | Saves each practice session. |
| `edges MS FILE.csv` | Saves each change on the key and paddle lines in that time, with its time in ms and its sequence number, and the MODE, PADDLES and SPEED settings as the device sends them. Shows how many changes were lost. |

Event lines show as they arrive, after `< `. The script finds the serial port by itself when there is only one; else give it with `--port PORT` before the steps.

A Python script can load the file and use its `Device` class: `press`, `shot`, `send`, `play`, `stop`, `info`, `get`, `set`, `stats`, `edges`, `expect`, `listen` and `drain` do what the steps do. A command the device answers with `ERROR` raises `RuntimeError`.

### Every screen

The `shots DIR` step saves a screenshot of every screen into a folder: KEYER and its MEM row, the menu, TIMING, the five receive drills, POTA, SOTA, HUNT, LETTERS, PROGRESS, SYSTEM, and each settings tab. Start it with the device on the KEYER screen; it ends there.

```bash
python3 -I tools/pocketcw.py shots shots
```

It keys back one group in each of the Koch, words and calls drills, so each shows a scored row. The ragchews run until one over ends, and each QSO runs for 4 s. All of them add to the practice history. The run takes about 3 minutes. RADIO is left out, because it shows only with the decoder board on.
