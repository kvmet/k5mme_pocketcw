# Credits

PocketCW builds on the work of others: data, assets, tools, and ideas for features. Each is listed here, with its license where PocketCW ships it.

## Data and assets

| Work | By | License | Used for |
| - | - | - | - |
| [B612 Mono](https://github.com/polarsys/b612) | Airbus, Intactile Design, PolarSys | [SIL Open Font License 1.1](https://github.com/kvmet/k5mme_pocketcw/blob/main/firmware/src/B612-OFL.txt) | The text font on every screen. |
| [wordfreq](https://github.com/rspeer/wordfreq) 3.1.1 | Robyn Speer | Data: CC BY-SA 4.0 | The common English words in the content pack (`tools/words/common.txt`). |
| [Kokoro-82M](https://huggingface.co/hexgrad/Kokoro-82M), voice `af_heart` | hexgrad | Apache 2.0 | The spoken words in the content pack. |

The Rust crates linked into the firmware are listed with their licenses in `THIRD-PARTY.txt`, in each release.

## Tools that build the content pack

These run on a computer when the pack is built. None of them ship in a release.

| Tool | By | License |
| - | - | - |
| [kokoro-onnx](https://github.com/thewh1teagle/kokoro-onnx) | thewh1teagle | MIT |
| [espeak-ng](https://github.com/espeak-ng/espeak-ng) | The espeak-ng project | GPL-3.0 |
| [FFmpeg](https://ffmpeg.org) | The FFmpeg project | LGPL-2.1 or later |
| [misaki](https://github.com/hexgrad/misaki) | hexgrad | Apache 2.0 |

`tools/voiceclips.py` holds a table from misaki's espeak fallback. The table converts espeak-ng phonemes to the form Kokoro was trained on.

## Ideas and compatibility

| Work | By | What PocketCW takes from it |
| - | - | - |
| [VBand](https://hamradio.solutions/vband/) | Ham Radio Solutions | The CW Hotline protocol and the USB keyboard keys that [USB](user/usb.md) speaks to VBand. |
| [WinKeyer](https://www.hamcrafters2.com/) | K1EL | The WinKeyer host protocol on the [serial port](dev/serial.md). |
| [Morse Code Ninja](https://morsecode.ninja) | Kurt Zoglmann, AD0WE | Morse, then the word spoken: the idea behind the planned NINJA drill. |
| The Koch method | Ludwig Koch | The order the [letters](user/letters.md) drill adds characters in. |
| Farnsworth timing | Donald R. Farnsworth, W6TTB | Characters at full speed with longer gaps, in the receive drills. |
| [Parks on the Air](https://parksontheair.com) and [Summits on the Air](https://www.sota.org.uk) | Their volunteer teams | The activation and hunting practice. |
| [CW-Pokemon-Infomation](https://github.com/admvip/CW-Pokemon-Infomation) | admvip | The vendor firmware files and notes used to go back to stock. |
| Artery `virtual_msc_iap` demo | Artery Technology | The vendor bootloader is based on it; [Flash layout](dev/hardware/flash-layout.md) describes how it takes files. |
