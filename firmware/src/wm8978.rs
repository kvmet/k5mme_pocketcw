//! WM8978 audio codec on boards before 2.64, set up as vendor V5.03.13 does it. The sidetone
//! enters on the line input and reaches the speaker output (OUT2) through the bypass.

use pocketcw_logic::volume::wm8978_speaker;

use crate::i2c::I2c;

pub const ADDRESS: u8 = 0x1A;

/// The register values the vendor setup leaves: (register, 9-bit value), reset first.
const SETUP: &[(u8, u16)] = &[
    (0, 0x000),  // reset
    (1, 0x00B),  // bias on, VMID 5k
    (2, 0x000),  // inputs and OUT1 off
    (3, 0x06C),  // output mixers and OUT2 on
    (43, 0x010), // ROUT2 inverted, for the bridged speaker
    (49, 0x004), // speaker boost
    (44, 0x000), // nothing into the input PGAs
    (47, 0x170), // line input into the boost stage at +6 dB
    (48, 0x170),
    (50, 0x01E), // bypass into the output mixer at +6 dB
    (51, 0x01E),
];
const SPEAKER_VOLUME: [u8; 2] = [54, 55];
const MUTE: u16 = 0x040;
/// Set on the right channel write, so both channels change together.
const UPDATE: u16 = 0x100;

/// Each write is 7 register bits then 9 value bits.
fn write(bus: &mut I2c, register: u8, value: u16) -> bool {
    bus.write(ADDRESS, &[register << 1 | ((value >> 8) as u8 & 1), value as u8])
}

/// Returns true when every register write was acknowledged.
pub fn init(bus: &mut I2c, volume: u8) -> bool {
    let setup_ok = SETUP.iter().fold(true, |ok, &(register, value)| write(bus, register, value) && ok);
    set_volume(bus, volume) && setup_ok
}

/// `level` as in `pocketcw_logic::volume`: 0 mutes the speaker.
pub fn set_volume(bus: &mut I2c, level: u8) -> bool {
    let value = wm8978_speaker(level).map_or(MUTE, u16::from);
    let left = write(bus, SPEAKER_VOLUME[0], value);
    write(bus, SPEAKER_VOLUME[1], value | UPDATE) && left
}
