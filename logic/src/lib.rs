//! Hardware-free parts of the firmware, tested on the host with `cargo test`.
#![no_std]

pub mod battery;
pub mod board;
pub mod buttons;
pub mod decode_lines;
pub mod decoder;
pub mod dial;
pub mod drill;
pub mod hunt;
pub mod input;
pub mod keyboard;
pub mod keyer;
pub mod letters;
pub mod listen;
mod math;
pub mod memories;
pub mod morse;
pub mod panel;
pub mod player;
pub mod progress;
pub mod qso;
pub mod ragchew;
pub mod record;
pub mod rng;
pub mod sender;
pub mod serial;
pub mod settings;
pub mod synth;
pub mod text;
pub mod ticks;
pub mod timing;
pub mod volume;
pub mod winkeyer;
