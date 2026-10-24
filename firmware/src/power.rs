//! Board power. A dial click (PA0, the wakeup pin) powers the board up, and PC13 must then hold
//! the power on. "Off" is standby; the next click starts the bootloader and then the app again.

use at32f4xx_pac::at32f403a as pac;
use cortex_m::peripheral::SCB;

use pocketcw_logic::buttons::DEBOUNCE_MS;

use crate::clock;
use crate::gpio;
use crate::pins::{BACKLIGHT, KEY_OUT_RING, KEY_OUT_TIP, LED, POWER_BUTTON, POWER_HOLD, SPEAKER_AMP};

/// Keeps the board powered after the dial click is released. Call it within about 1 s of start,
/// with the GPIOC clock running.
pub fn hold() {
    gpio::set_high(gpio::port_c(), POWER_HOLD);
    gpio::configure(gpio::port_c(), POWER_HOLD, gpio::OUTPUT);
}

/// Waits for the dial click to read released for DEBOUNCE_MS, then turns the outputs off and
/// enters standby. Enabling the wakeup pin while the click is high wakes the chip at once.
pub fn standby() -> ! {
    let mut released_ms = 0;
    while released_ms < DEBOUNCE_MS {
        released_ms = if gpio::is_high(gpio::port_a(), POWER_BUTTON) { 0 } else { released_ms + 1 };
        clock::delay_ms(1);
    }
    cortex_m::interrupt::disable();
    // The backlight pin runs from its PWM timer; take it back so the low below turns it off.
    gpio::configure(gpio::port_b(), BACKLIGHT, gpio::OUTPUT);
    for (port, pin) in [
        (gpio::port_b(), BACKLIGHT),
        (gpio::port_b(), SPEAKER_AMP),
        (gpio::port_b(), LED),
        (gpio::port_a(), KEY_OUT_TIP),
        (gpio::port_a(), KEY_OUT_RING),
        (gpio::port_c(), POWER_HOLD),
    ] {
        gpio::set_low(port, pin);
    }
    // Interrupts stay off from here to reset, so nothing else uses these peripherals.
    let mut core = unsafe { cortex_m::Peripherals::steal() };
    let pwc = unsafe { &*pac::PWC::ptr() };
    core.SYST.disable_interrupt();
    core.SYST.disable_counter();
    // An enabled, pending interrupt ends WFI at once even with interrupts masked.
    for i in 0..core.NVIC.icer.len() {
        unsafe {
            core.NVIC.icer[i].write(u32::MAX);
            core.NVIC.icpr[i].write(u32::MAX);
        }
    }
    SCB::clear_pendst();
    SCB::clear_pendsv();
    pwc.ctrl().modify(|_, w| w.clsef().set_bit());
    pwc.ctrlsts().modify(|_, w| w.swpen().set_bit());
    // Enabling the wakeup pin can set the wakeup flag, so clear it afterwards.
    pwc.ctrl().modify(|_, w| w.clswef().set_bit());
    pwc.ctrl().modify(|_, w| w.lpsel().set_bit());
    core.SCB.set_sleepdeep();
    cortex_m::asm::dsb();
    cortex_m::asm::wfi();
    // Standby ends only in reset. Reaching here means standby was refused; restart visibly.
    SCB::sys_reset()
}
