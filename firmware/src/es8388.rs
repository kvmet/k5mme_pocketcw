//! ES8388 audio codec from board 2.64. The sidetone (PA8) enters on input 1 and reaches both
//! outputs through the analog bypass: OUT1 drives the speaker, OUT2 the headphones.

use pocketcw_logic::volume::codec_gain;

use crate::clock;
use crate::i2c::I2c;

pub const ADDRESS: u8 = 0x10;

/// The bypass start-up of the Everest ES8388 User Guide, section 10.4, with VMID on its 5k
/// divider so the reference charges fast.
const BYPASS: &[(u8, u8)] = &[
    (0x02, 0xF3), // DEM and state machines in reset, references down
    (0x00, 0x07), // reference on, VMID 5k
    (0x01, 0x40), // analog and bias generator on
    (0x03, 0x3F), // ADCs, mic bias and ADC bias down; analog inputs on for the bypass
    (0x04, 0xFC), // DACs down; LOUT1, ROUT1, LOUT2, ROUT2 on
    (0x26, 0x00), // mixer source: LIN1 and RIN1
    (0x02, 0xF0), // references up; DEM and state machines stay in reset
];
/// VMID charge time on the 5k divider before it moves to 50k for play.
const VMID_CHARGE_MS: u32 = 100;
const VMID_50K: (u8, u8) = (0x00, 0x05);
pub const MIXER_INPUT: [u8; 2] = [0x27, 0x2A]; // left and right input into the output mixers
/// Input switched into the mixer at 0 dB; each step of bits 5..3 above it cuts 3 dB.
pub const MIXER_ON_0DB: u8 = 0x50;
pub const SPEAKER_VOLUME: [u8; 2] = [0x2E, 0x2F]; // LOUT1, ROUT1
pub const HEADPHONE_VOLUME: [u8; 2] = [0x30, 0x31]; // LOUT2, ROUT2

/// Returns true when every register write was acknowledged. Takes VMID_CHARGE_MS; the speaker
/// amp should switch on only after it, so the charge doesn't thump.
pub fn init(bus: &mut I2c, volume: u8) -> bool {
    let bypass_ok = BYPASS.iter().fold(true, |ok, &(register, value)| bus.write(ADDRESS, &[register, value]) && ok);
    let volume_ok = set_volume(bus, volume);
    clock::delay_ms(VMID_CHARGE_MS);
    bus.write(ADDRESS, &[VMID_50K.0, VMID_50K.1]) && volume_ok && bypass_ok
}

/// `level` as in `pocketcw_logic::volume`: 0 disconnects the tone from the mixers. The synth
/// makes the rest of the level with `pocketcw_logic::volume::CodecGain::synth_gain`.
pub fn set_volume(bus: &mut I2c, level: u8) -> bool {
    let (mixer, speaker, headphones) = match codec_gain(level) {
        Some(gain) => (MIXER_ON_0DB + (gain.mixer_cut << 3), gain.speaker, gain.headphones),
        None => (0, 0, 0),
    };
    let writes = MIXER_INPUT.map(|r| (r, mixer)).into_iter();
    let writes = writes.chain(SPEAKER_VOLUME.map(|r| (r, speaker))).chain(HEADPHONE_VOLUME.map(|r| (r, headphones)));
    writes.fold(true, |ok, (register, value)| bus.write(ADDRESS, &[register, value]) && ok)
}
