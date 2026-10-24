//! Fatal errors: shown on screen with the keyed outputs and sidetone off, then the chip waits
//! for a dial click to power off.

use core::fmt::Write;

use at32f4xx_pac::at32f403a as pac;
use embedded_graphics::{
    pixelcolor::{Rgb565, RgbColor},
    prelude::*,
};

use crate::clock::delay_ms;
use crate::fonts::SMALL;
use crate::gpio::{self, port_a, port_b};
use crate::lcd::Lcd;
use crate::pins::{KEY_OUT_RING, KEY_OUT_TIP, LED, POWER_BUTTON};
use crate::power;
use crate::text::Text;

const COLS: usize = 32;
const ROWS: usize = 15;
const BG: Rgb565 = Rgb565::new(12, 0, 0);

/// A click must read the same this long to count.
const DEBOUNCE_MS: u32 = 50;

/// Shows `title` and the text `detail` writes, wrapped to the screen, then powers off at the next
/// dial click. Interrupts stay off, so the click is polled.
pub fn halt(title: &str, detail: impl FnOnce(&mut Text<400>)) -> ! {
    cortex_m::interrupt::disable();
    // Unkey first: a fault must not leave a radio transmitting.
    for (port, pin) in [(port_a(), KEY_OUT_TIP), (port_a(), KEY_OUT_RING), (port_b(), LED)] {
        gpio::set_low(port, pin);
    }
    // Interrupts stay off from here on, so nothing else touches these peripherals.
    let dp = unsafe { pac::Peripherals::steal() };
    dp.TMR1.cctrl().write(|w| w.c1en().clear_bit());

    let mut text = Text::new();
    detail(&mut text);
    let mut lcd = Lcd::new(dp.SPI1);
    let _ = lcd.clear(BG);
    let _ = SMALL.draw(&mut lcd, Point::zero(), title, Rgb565::WHITE, BG);
    let bytes = text.as_str().as_bytes();
    for (row, chunk) in bytes.chunks(COLS).take(ROWS - 3).enumerate() {
        let line = core::str::from_utf8(chunk).unwrap_or("?");
        let _ = SMALL.draw(&mut lcd, Point::new(0, (row as i32 + 2) * SMALL.height as i32), line, Rgb565::WHITE, BG);
    }
    let _ = SMALL.draw(&mut lcd, Point::new(0, (ROWS as i32 - 1) * SMALL.height as i32), "CLICK THE DIAL TO POWER OFF", Rgb565::WHITE, BG);
    // The click that powered the board on may still be down.
    wait_for_click(false);
    wait_for_click(true);
    wait_for_click(false);
    power::standby()
}

/// Returns once the dial click has read `down` for `DEBOUNCE_MS`.
fn wait_for_click(down: bool) {
    let mut steady_ms = 0;
    while steady_ms < DEBOUNCE_MS {
        steady_ms = if gpio::is_high(port_a(), POWER_BUTTON) == down { steady_ms + 1 } else { 0 };
        delay_ms(1);
    }
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    halt("panic", |text| {
        if let Some(location) = info.location() {
            let file = location.file().rsplit('/').next().unwrap_or("");
            let _ = write!(text, "{file}:{} ", location.line());
        }
        let _ = write!(text, "{}", info.message());
    })
}
