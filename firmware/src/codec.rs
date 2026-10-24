//! The audio codec: a WM8978 before board 2.64 and an ES8388 on it. They answer at different
//! I2C addresses, so the one that answers is the one fitted.

use crate::i2c::I2c;
use crate::{es8388, wm8978};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Codec {
    Es8388,
    Wm8978,
}

impl Codec {
    pub fn detect(bus: &mut I2c) -> Option<Codec> {
        if bus.probe(es8388::ADDRESS) {
            Some(Codec::Es8388)
        } else if bus.probe(wm8978::ADDRESS) {
            Some(Codec::Wm8978)
        } else {
            None
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Codec::Es8388 => "ES8388",
            Codec::Wm8978 => "WM8978",
        }
    }

    /// Returns true when every register write was acknowledged.
    pub fn init(self, bus: &mut I2c, volume: u8) -> bool {
        match self {
            Codec::Es8388 => es8388::init(bus, volume),
            Codec::Wm8978 => wm8978::init(bus, volume),
        }
    }

    pub fn set_volume(self, bus: &mut I2c, level: u8) -> bool {
        match self {
            Codec::Es8388 => es8388::set_volume(bus, level),
            Codec::Wm8978 => wm8978::set_volume(bus, level),
        }
    }
}
