//! PA8 as a 1-bit DAC into the codec's input 1. TMR1 sends a 281 kHz pulse train, and the sample
//! interrupt sets its duty from the synth 17,578 times a second; the codec input and the
//! headphones smooth the pulses into audio.

use at32f4xx_pac::at32f403a as pac;
use pocketcw_logic::synth::{self, Sound, Synth};

use crate::clock::TIMER_HZ;
use crate::gpio::{self, Port};
use crate::pins::SIDETONE as PIN;

/// Timer counts per pulse: the duty resolution.
const STEPS: u32 = 256;
/// Pulses per sample. The timer's update event, and with it the sample interrupt and the duty
/// change, comes once per this many.
const REPEAT: u32 = 16;
pub const SAMPLE_HZ: f32 = (TIMER_HZ / STEPS / REPEAT) as f32;
/// Half duty: the silent level. The pin stays here between sounds, so starting one doesn't pop.
const MID: u16 = (STEPS / 2) as u16;

pub struct Audio {
    timer: pac::TMR1,
    pub synth: Synth,
}

impl Audio {
    pub fn new(timer: pac::TMR1, gpioa: &Port, sound: Sound) -> Self {
        unsafe {
            timer.div().write(|w| w.bits(0));
            timer.pr().write(|w| w.bits(STEPS as u16 - 1));
            timer.rpr().write(|w| w.bits(REPEAT as u8 - 1));
            timer.c1dt().write(|w| w.bits(MID));
            // C1OCTRL = 110 (PWM mode A), C1OBEN = 1 (compare loads at the update event).
            timer.cm1_output().write(|w| w.bits(0b110 << 4 | 1 << 3));
            timer.brk().write(|w| w.oen().set_bit());
        }
        timer.swevt().write(|w| w.ovfswtr().set_bit());
        timer.cctrl().write(|w| w.c1en().set_bit());
        timer.ctrl1().write(|w| w.tmren().set_bit());
        gpio::configure(gpioa, PIN, gpio::PERIPHERAL_OUTPUT);
        Audio { timer, synth: Synth::new(SAMPLE_HZ, sound) }
    }

    /// Starts the sample interrupt when the synth has something to play. Call after keying it or
    /// changing its sound.
    pub fn wake(&mut self) {
        if !self.synth.is_silent() {
            self.timer.iden().modify(|_, w| w.ovfien().set_bit());
        }
    }

    /// The sample interrupt: the next duty, or once everything is silent, the silent level and no
    /// more interrupts until `wake`.
    pub fn sample(&mut self) {
        self.timer.ists().write(|w| w.ovfif().clear_bit());
        let duty = if self.synth.is_silent() {
            self.timer.iden().modify(|_, w| w.ovfien().clear_bit());
            MID
        } else {
            synth::duty(self.synth.sample(), STEPS as u16)
        };
        unsafe { self.timer.c1dt().write(|w| w.bits(duty)) };
    }
}
