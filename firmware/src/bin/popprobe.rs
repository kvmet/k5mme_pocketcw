//! Pop probe: finds which ES8388 write makes a volume change pop in silence. Each mode toggles
//! one kind of write, by hand with F4 or once a second with F3, so the pops can be compared by
//! ear. A pop with SAME VALUES or READ ONLY comes from the bus traffic, not a gain change.
//! `sh tools/mkimage.sh popprobe`.
#![no_std]
#![no_main]

use core::fmt::Write;
use core::ptr::read_volatile;

use at32f4xx_pac::at32f403a as pac;
use cortex_m_rt::entry;
use embedded_graphics::{
    mono_font::{MonoTextStyleBuilder, ascii::FONT_10X20},
    pixelcolor::Rgb565,
    prelude::*,
    text::{Baseline, Text as Label},
};

use pocketcw_firmware::clock::{self, delay_ms};
use pocketcw_firmware::codec::Codec;
use pocketcw_firmware::es8388::{self, ADDRESS, HEADPHONE_VOLUME, MIXER_INPUT, MIXER_ON_0DB, SPEAKER_VOLUME};
use pocketcw_firmware::i2c::I2c;
use pocketcw_firmware::lcd::Lcd;
use pocketcw_firmware::pins::{BACKLIGHT, DIAL_LEFT, DIAL_RIGHT, F_KEYS, POWER_BUTTON, SIDETONE, SPEAKER_AMP};
use pocketcw_firmware::{eeprom, gpio, power, store};
use pocketcw_logic::panel;
use pocketcw_logic::text::Text;
use pocketcw_logic::volume::{MAX_LEVEL, MAX_MIXER_CUT, codec_gain};

/// Timer counts per pulse, as in `audio.rs`, and pulses per compare load.
const STEPS: u16 = 256;
const REPEAT: u8 = 16;
const MID: u16 = STEPS / 2;
const VOLUME_START: u8 = 23;
/// The output volume registers' top, +4.5 dB.
const OUTPUT_MAX: u8 = 0x21;
const TOGGLE_LOOPS: u32 = 100; // 1 s of 10 ms loops
const HOLD_OFF_LOOPS: u32 = 150; // 1.5 s of 10 ms loops
const COLUMNS: usize = 32;
const ROW_HEIGHT: i32 = 20;

#[derive(Clone, Copy)]
enum Mode {
    /// Every volume register, between the level and the next, as the app writes them.
    All,
    /// Every volume register again, with the values it already holds.
    Same,
    Speaker,
    Headphones,
    Mixer,
    /// Both outputs, 4 steps apart: does a bigger gain change pop louder?
    Big,
    /// Reads the volume registers and writes nothing.
    Read,
}

const MODES: [(Mode, &str); 7] = [
    (Mode::All, "ALL, AS THE APP DOES"),
    (Mode::Same, "SAME VALUES AGAIN"),
    (Mode::Speaker, "OUT1 SPEAKER 1.5 DB"),
    (Mode::Headphones, "OUT2 HEADPHONES 1.5 DB"),
    (Mode::Mixer, "MIXER 3 DB"),
    (Mode::Big, "OUT1 AND OUT2 6 DB"),
    (Mode::Read, "READ ONLY"),
];

#[entry]
fn main() -> ! {
    // The vector table holds only default handlers, so the probe polls with interrupts off.
    cortex_m::interrupt::disable();
    let dp = pac::Peripherals::take().unwrap();
    dp.CRM.apb2en().modify(|_, w| {
        w.iomux().set_bit().gpioa().set_bit().gpiob().set_bit().gpioc().set_bit();
        w.spi1().set_bit().tmr1().set_bit()
    });
    power::hold();
    clock::init(&dp.CRM);
    // JTAG off and SWD on frees PA15, PB3 and PB4. SPI1 moves to PB3/PB4/PB5.
    dp.IOMUX.remap().modify(|_, w| unsafe { w.swjtag_mux().bits(0b010).spi1_mux0().set_bit() });
    let (gpioa, gpiob, gpioc) = (gpio::port_a(), gpio::port_b(), gpio::port_c());
    gpio::set_high(gpiob, BACKLIGHT);
    gpio::configure(gpiob, BACKLIGHT, gpio::OUTPUT);
    gpio::set_low(gpiob, SPEAKER_AMP);
    gpio::configure(gpiob, SPEAKER_AMP, gpio::OUTPUT);
    for pin in [POWER_BUTTON, F_KEYS[0], F_KEYS[1], F_KEYS[2], F_KEYS[3]] {
        gpio::set_low(gpioa, pin);
        gpio::configure(gpioa, pin, gpio::INPUT_PULL);
    }
    for pin in [DIAL_LEFT, DIAL_RIGHT] {
        gpio::set_low(gpioc, pin);
        gpio::configure(gpioc, pin, gpio::INPUT_PULL);
    }

    let mut bus = I2c::new();
    let mut record = [0u8; panel::RECORD_LEN];
    let saved = if eeprom::read(&mut bus, store::PANEL_ADDRESS, &mut record) { panel::decode(&record) } else { None };
    let mut lcd = Lcd::new(dp.SPI1);
    lcd.reset();
    let detected = panel::detect(&lcd.read_id());
    lcd.init(panel::choose(saved, detected, unsafe { read_volatile(0x1FFF_F806 as *const u8) }).0);
    lcd.clear(Rgb565::BLACK).unwrap();

    show(&mut lcd, 0, "POP PROBE");
    let es8388 = Codec::detect(&mut bus) == Some(Codec::Es8388);
    if es8388 {
        es8388::init(&mut bus, VOLUME_START);
        gpio::set_high(gpiob, SPEAKER_AMP);
        show(&mut lcd, 2, "F1/F2 MODE");
        show(&mut lcd, 3, "F3 AUTO TOGGLE  F4 TOGGLE");
        show(&mut lcd, 4, "DIAL VOLUME  HOLD CLICK: OFF");
    } else {
        show(&mut lcd, 2, "NEEDS THE ES8388");
    }
    start_pwm(&dp.TMR1);
    gpio::configure(gpioa, SIDETONE, gpio::PERIPHERAL_OUTPUT);

    let mut line = Text::<64>::new();
    let mut mode = 0;
    let mut volume = VOLUME_START;
    let mut toggling = false;
    let mut raised = false;
    let mut loops = 0;
    let mut previous = [gpio::inputs(gpioa), gpio::inputs(gpioc)];
    // The click is still down from switching on, so a hold counts only after one release.
    let mut armed = false;
    let mut held = 0;
    let mut redraw = es8388;
    loop {
        if redraw {
            draw(&mut lcd, &mut line, MODES[mode].1, volume, toggling);
            redraw = false;
        }
        delay_ms(10);
        let now = [gpio::inputs(gpioa), gpio::inputs(gpioc)];
        let pressed = |port: usize, pin: u32| now[port] & !previous[port] & (1 << pin) != 0;
        let f = |i: usize| pressed(0, F_KEYS[i]);
        // The dial click also raises PC14, so a click must not count as dial left.
        let left = pressed(1, DIAL_LEFT) && now[0] & (1 << POWER_BUTTON) == 0;
        if es8388 && (f(0) || f(1) || left || pressed(1, DIAL_RIGHT)) {
            if f(0) || f(1) {
                mode = if f(0) { (mode + MODES.len() - 1) % MODES.len() } else { (mode + 1) % MODES.len() };
            } else {
                volume = if left { volume.saturating_sub(1).max(1) } else { (volume + 1).min(MAX_LEVEL) };
            }
            es8388::set_volume(&mut bus, volume);
            (raised, loops, redraw) = (false, 0, true);
        }
        if es8388 && f(2) {
            toggling = !toggling;
            (loops, redraw) = (0, true);
        }
        loops += 1;
        if es8388 && (f(3) || toggling && loops >= TOGGLE_LOOPS) {
            raised = !raised;
            toggle(&mut bus, MODES[mode].0, volume, raised);
            loops = 0;
        }
        if now[0] & (1 << POWER_BUTTON) != 0 {
            held += 1;
            if armed && held >= HOLD_OFF_LOOPS {
                show(&mut lcd, 10, "RELEASE TO POWER OFF");
                power::standby();
            }
        } else {
            armed = true;
            held = 0;
        }
        previous = now;
    }
}

/// `value` moved `by` steps when `raised`: up where that stays at or under `max`, else down.
fn moved(value: u8, by: u8, max: u8, raised: bool) -> u8 {
    match raised {
        false => value,
        true if value + by <= max => value + by,
        true => value - by,
    }
}

/// Writes `mode`'s registers at `volume`, or the values a step away when `raised`.
fn toggle(bus: &mut I2c, mode: Mode, volume: u8, raised: bool) {
    let gain = codec_gain(volume).unwrap();
    let mut write = |registers: [u8; 2], value: u8| {
        for register in registers {
            bus.write(ADDRESS, &[register, value]);
        }
    };
    match mode {
        Mode::All => {
            es8388::set_volume(bus, moved(volume, 1, MAX_LEVEL, raised));
        }
        Mode::Same => {
            es8388::set_volume(bus, volume);
        }
        Mode::Speaker => write(SPEAKER_VOLUME, moved(gain.speaker, 1, OUTPUT_MAX, raised)),
        Mode::Headphones => write(HEADPHONE_VOLUME, moved(gain.headphones, 1, OUTPUT_MAX, raised)),
        Mode::Mixer => write(MIXER_INPUT, MIXER_ON_0DB + (moved(gain.mixer_cut, 1, MAX_MIXER_CUT, raised) << 3)),
        Mode::Big => {
            write(SPEAKER_VOLUME, moved(gain.speaker, 4, OUTPUT_MAX, raised));
            write(HEADPHONE_VOLUME, moved(gain.headphones, 4, OUTPUT_MAX, raised));
        }
        Mode::Read => {
            for register in MIXER_INPUT.into_iter().chain(SPEAKER_VOLUME).chain(HEADPHONE_VOLUME) {
                bus.read_register(ADDRESS, register);
            }
        }
    }
}

/// PA8's pulse train at the silent level, with no DMA or sample interrupt.
fn start_pwm(timer: &pac::TMR1) {
    unsafe {
        timer.div().write(|w| w.bits(0));
        timer.pr().write(|w| w.bits(STEPS - 1));
        timer.rpr().write(|w| w.bits(REPEAT - 1));
        timer.c1dt().write(|w| w.bits(MID));
        // C1OCTRL = 110 (PWM mode A), C1OBEN = 1 (compare loads at the update event).
        timer.cm1_output().write(|w| w.bits(0b110 << 4 | 1 << 3));
        timer.brk().write(|w| w.oen().set_bit());
    }
    timer.swevt().write(|w| w.ovfswtr().set_bit());
    timer.cctrl().write(|w| w.c1en().set_bit());
    timer.ctrl1().write(|w| w.tmren().set_bit());
}

fn draw(lcd: &mut Lcd, line: &mut Text<64>, mode: &str, volume: u8, toggling: bool) {
    show(lcd, 6, mode);
    line.clear();
    let _ = write!(line, "VOL {volume}  AUTO {}", if toggling { "ON" } else { "OFF" });
    show(lcd, 7, line.as_str());
}

fn show(lcd: &mut Lcd, row: i32, text: &str) {
    let style = MonoTextStyleBuilder::new()
        .font(&FONT_10X20)
        .text_color(Rgb565::WHITE)
        .background_color(Rgb565::BLACK)
        .build();
    // Pad to the full width so a shorter line erases the old one.
    let mut padded = Text::<64>::new();
    let _ = write!(padded, "{text:<COLUMNS$}");
    Label::with_baseline(padded.as_str(), Point::new(0, row * ROW_HEIGHT), style, Baseline::Top)
        .draw(lcd)
        .unwrap();
}
