use at32f4xx_pac::at32f403a as pac;
pub use pac::gpioa::RegisterBlock as Port;

// Any task may set, clear and read pins: those registers are write-only or read-only, so no
// read-modify-write can race. `configure` is read-modify-write, so only init and the UI task
// call it.
pub fn port_a() -> &'static Port {
    unsafe { &*pac::GPIOA::ptr() }
}

pub fn port_b() -> &'static Port {
    unsafe { &*pac::GPIOB::ptr() }
}

pub fn port_c() -> &'static Port {
    unsafe { &*pac::GPIOC::ptr() }
}

/// Pin configurations: the 4-bit CFGLR/CFGHR field, IOFC in bits 3:2 and IOMC in bits 1:0.
pub const OUTPUT: u32 = 0b0010;
/// Large drive strength (IOMC 01): the vendor's setting for LCD CS, LCD DC, flash CS and LCD reset.
pub const OUTPUT_LARGE: u32 = 0b0001;
pub const OUTPUT_OPEN_DRAIN: u32 = 0b0110;
pub const PERIPHERAL_OUTPUT: u32 = 0b1010;
/// IOMC 11, the vendor's setting for the SPI clock and data. The PAC calls it maximum strength;
/// Artery's library documents only 01 and 10, so it is at least large.
pub const PERIPHERAL_OUTPUT_MAXIMUM: u32 = 0b1011;
/// Pull direction comes from the pin's ODT bit: set = pull-up, clear = pull-down.
pub const INPUT_PULL: u32 = 0b1000;
pub const ANALOG: u32 = 0b0000;

pub fn configure(port: &Port, pin: u32, config: u32) {
    let shift = (pin % 8) * 4;
    let update = |bits: u32| (bits & !(0xF << shift)) | (config << shift);
    if pin < 8 {
        port.cfglr().modify(|r, w| unsafe { w.bits(update(r.bits())) });
    } else {
        port.cfghr().modify(|r, w| unsafe { w.bits(update(r.bits())) });
    }
}

pub fn set_high(port: &Port, pin: u32) {
    port.scr().write(|w| unsafe { w.bits(1 << pin) });
}

pub fn set_low(port: &Port, pin: u32) {
    port.clr().write(|w| unsafe { w.bits(1 << pin) });
}

pub fn set(port: &Port, pin: u32, high: bool) {
    if high {
        set_high(port, pin)
    } else {
        set_low(port, pin)
    }
}

pub fn is_high(port: &Port, pin: u32) -> bool {
    inputs(port) & (1 << pin) != 0
}

pub fn inputs(port: &Port) -> u16 {
    port.idt().read().bits() as u16
}
