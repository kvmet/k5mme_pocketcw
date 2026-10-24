//! LCD backlight on PB8, dimmed by TMR4 channel 3 PWM.

use at32f4xx_pac::at32f403a as pac;
use pocketcw_logic::settings::MAX_BRIGHTNESS;

use crate::clock::TIMER_HZ;
use crate::gpio;
use crate::pins::BACKLIGHT;

/// Above hearing, so the backlight supply can't whine into the audio.
const PWM_HZ: u32 = 20_000;
const PERIOD: u32 = TIMER_HZ / PWM_HZ;

pub struct Backlight {
    timer: pac::TMR4,
}

impl Backlight {
    /// Needs the TMR4 and GPIOB clocks running.
    pub fn new(timer: pac::TMR4, level: u8) -> Self {
        unsafe {
            timer.pr().write(|w| w.bits((PERIOD - 1) as u16));
            // C3OCTRL = 110 (PWM mode A), C3OBEN = 1 (buffered compare).
            timer.cm2_output().write(|w| w.bits(0b110 << 4 | 1 << 3));
        }
        let mut backlight = Backlight { timer };
        backlight.set(level);
        backlight.timer.swevt().write(|w| w.ovfswtr().set_bit());
        backlight.timer.cctrl().write(|w| w.c3en().set_bit());
        backlight.timer.ctrl1().write(|w| w.tmren().set_bit());
        gpio::configure(gpio::port_b(), BACKLIGHT, gpio::PERIPHERAL_OUTPUT);
        backlight
    }

    /// `level` in tenths, up to `MAX_BRIGHTNESS`.
    pub fn set(&mut self, level: u8) {
        let duty = PERIOD * level.min(MAX_BRIGHTNESS) as u32 / MAX_BRIGHTNESS as u32;
        unsafe { self.timer.c3dt().write(|w| w.bits(duty as u16)) };
    }
}
