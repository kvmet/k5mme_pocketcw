# Audio

## Signal path

PA8 makes the audio. It goes into input 1 of the audio codec, and the codec's analog bypass sends it to the speaker and the headphone jack. The speaker amplifier needs PB12 high.

- The vendor sends a square wave on PA8 at the tone pitch, from TMR1 channel 1 PWM. Confirmed on 2.64.
- PocketCW uses PA8 as a 1 bit DAC: a 281 kHz pulse train whose duty changes 17,578 times a second. The codec input and the headphones smooth the pulses into sine tones and noise. Confirmed on 2.64.

PocketCW timing (`firmware/src/audio.rs`): TMR1 counts 256 steps per pulse at 72 MHz, which is 281.25 kHz. The duty changes every 16 pulses. Between sounds the duty stays at half, the silent level, so a new sound does not pop, and the sample interrupt stops.

## Codecs

The board has one of two codecs on I2C2. PocketCW tries the ES8388 address first, then the WM8978 address, and uses the one that answers.

| Codec | I2C address | Boards |
| - | - | - |
| WM8978 | 0x1A | Older boards. |
| ES8388 | 0x10 | Board 2.64, which has no device at 0x1A. |

### WM8978

V5.03.13 uses the ALIENTEK WM8978 driver: register write at 0x0801BEE8, register cache at RAM 0x200000F0. Its setup at 0x08005F04 leaves these values:

| Register | Value | Effect |
| - | - | - |
| R0 | 0 | Reset. |
| R1 | 0x00B | Bias on, VMID 5k. |
| R2 | 0x000 | Inputs and OUT1 off. |
| R3 | 0x06C | Output mixers and OUT2 on. |
| R43 | 0x010 | ROUT2 inverted, for the bridged speaker. |
| R49 | 0x004 | Speaker boost. |
| R44 | 0x000 | Nothing into the input PGAs. |
| R47, R48 | 0x170 | Line input into the boost stage at +6 dB. |
| R50, R51 | 0x01E | Bypass into the output mixers at +6 dB. |

Then it drives PB12 high and sets the speaker volume, R54 and R55, from the vendor volume setting (0 to 63, and 0 writes the mute bit). R52 and R53, the OUT1 volume, are written at 0x080164D4, but R2 leaves OUT1 off.

PocketCW writes the same values (`firmware/src/wm8978.rs`). Its volume level 44 gives 61 in R54 and R55, which is +4 dB.

### ES8388

No vendor firmware sets up the ES8388, so the vendor app is silent on board 2.64.

PocketCW (`firmware/src/es8388.rs`) follows the bypass start-up in section 10.4 of the Everest ES8388 User Guide:

| Step | Register = value | Effect |
| - | - | - |
| 1 | 0x02 = 0xF3 | State machines in reset, references down. |
| 2 | 0x00 = 0x07 | Reference on, VMID 5k. |
| 3 | 0x01 = 0x40 | Analog and bias generator on. |
| 4 | 0x03 = 0x3F | ADCs down, analog inputs on for the bypass. |
| 5 | 0x04 = 0xFC | DACs down; LOUT1, ROUT1, LOUT2 and ROUT2 on. |
| 6 | 0x26 = 0x00 | Mixer source: LIN1 and RIN1. |
| 7 | 0x02 = 0xF0 | References up. |
| 8 | Volume | Registers 0x27 and 0x2A (input into the output mixers), and 0x2E to 0x31 (output volumes). |
| 9 | Wait 100 ms | VMID charges. |
| 10 | 0x00 = 0x05 | VMID 50k for play. |

- Analog bypass from input 1 to the output mixers, with PB12 high, plays through the speaker. At 0 dB output volume it is very loud. Input 2 carries no tone.
- OUT1 (LOUT1, ROUT1) drives the speaker, and OUT2 (LOUT2, ROUT2) the headphones.
- A plug in the headphone jack cuts the speaker whatever the codec setup, so the jack switch is in the speaker path. The switch reaches no MCU input, so the firmware cannot tell which output is in use.

## Volume

`logic/src/volume.rs` maps the VOLUME setting. Level 0 mutes. Levels 1 to 44 go from -60 dB to +4.5 dB in 1.5 dB steps. On the ES8388, the output stage cuts first, down to -45 dB. Below that, the mixer input cuts in 3 dB steps. A cut before the output stage raises the hiss relative to the tone.

## Speaker amplifier

PB12 enables the speaker amplifier.

- The vendor sets it high during WM8978 init, and toggles it with the mute shortcut.
- PocketCW holds it low until the codec setup ends, which on the ES8388 includes the VMID charge. Then it drives PB12 high while the volume is above 0. At volume 0 the amplifier is off, so the pulse train on the codec input cannot reach the speaker.

## Synthesis

`logic/src/synth.rs` mixes five voices into one signal: your sidetone and four players, so stations can overlap.

- Key edges rise and fall over 5 ms on a raised cosine, so they do not click.
- WAVE SQUARE sums the odd harmonics up to 31. At 17.6 kHz this reaches the Nyquist frequency for pitches down to 284 Hz. A lower pitch loses its top harmonics.
- Band noise has a 500 Hz bandwidth, like a receiver CW filter. Its 10 levels are 2.5 dB apart, up to an RMS of 0.35.
- Fading goes down to a tenth at the top level. Each player fades at its own rate, so stations fade apart.
- When stations overlap, the gain drops so the sum stays at one voice's peak. The gain recovers over 0.5 s, like a receiver AGC. A limiter above 0.8 bends noise peaks down so the mix does not clip.
