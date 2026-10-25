//! Hardware probe: shows chip and board IDs, reads the LCD memory back, scans I2C2, shows live
//! inputs, tests the audio codec, PWM audio and standby.
#![no_std]
#![no_main]

use core::fmt::Write;
use core::ptr::read_volatile;

use at32f4xx_pac::at32f403a as pac;
use cortex_m_rt::entry;
use embedded_graphics::{
    mono_font::{MonoTextStyleBuilder, ascii::FONT_6X10},
    pixelcolor::{Rgb565, raw::RawU16},
    prelude::*,
    primitives::Rectangle,
    text::{Baseline, Text},
};

use pocketcw_firmware::clock::{self, delay_ms};
use pocketcw_firmware::gpio;
use pocketcw_firmware::i2c::I2c;
use pocketcw_firmware::lcd::Lcd;
use pocketcw_firmware::{eeprom, store};
use pocketcw_firmware::pins::{
    BACKLIGHT, DIAL_LEFT, DIAL_RIGHT, F_KEYS, KEY_OUT_RING, KEY_OUT_TIP, DECODER_POWER, DECODER_TONE, LED, PB15, POWER_BUTTON, POWER_HOLD, RING,
    SIDETONE, SPEAKER_AMP, TIP,
};
use pocketcw_logic::{morse, panel, rng::Rng};

const PORT_A_BUTTONS: [u32; 5] = [POWER_BUTTON, F_KEYS[0], F_KEYS[1], F_KEYS[2], F_KEYS[3]];
const PORT_C_BUTTONS: [u32; 2] = [DIAL_LEFT, DIAL_RIGHT];

const TONE_HZ: u32 = 700;
const TIMER_TICK_HZ: u32 = 1_000_000;
const CODEC: u8 = 0x10;

const VOLUME_MAX: u8 = 0x21; // +4.5 dB in 1.5 dB steps from 0x00 = -45 dB
const VOLUME_START: u8 = 0x0C; // -27 dB; 0 dB through the speaker is painfully loud
/// ES8388 register 0x04 values: both DACs on plus the chosen output pairs.
const OUTPUTS: [(u8, &str); 3] = [(0x3C, "OUT1+OUT2"), (0x30, "OUT1     "), (0x0C, "OUT2     ")];

const COLUMNS: usize = 53;
const ROW_HEIGHT: i32 = 10;
const AUDIO_ROW: i32 = 7;
const DUMP_ROW: i32 = 14;
const CHANGED_ROW: i32 = 19;
const STATUS_ROW: i32 = 20;
const READBACK_ROW: i32 = 21;
/// Bits of each port's input register that this probe or the USB and SWD pins drive, left out of
/// the changed-bits line: PA2, PA3, PA8, PA11..PA15; PB2..PB9, PB12, PB15; PC13.
const DRIVEN: [u16; 3] = [0xF90C, 0x93FC, 0x2000];
const POWER_OFF_LOOPS: u32 = 150; // about 1.5 s of 10 ms loops, the vendor hold time

/// ES8388 analog bypass from input 1, where the PA8 tone enters, into the output mixers.
/// Output enables (register 0x04) and volume (0x2E..0x31) are written separately.
const ES8388_BYPASS: &[(u8, u8)] = &[
    (0x02, 0x00), // chip power: all blocks on
    (0x01, 0x50), // analog power on
    (0x03, 0x00), // ADC and input power on
    (0x26, 0x00), // mixer source: LIN1 and RIN1
    (0x27, 0x50), // left input to left mixer, 0 dB
    (0x2A, 0x50), // right input to right mixer, 0 dB
];

#[entry]
fn main() -> ! {
    // Every handler in our vector table is the default loop, so keep interrupts off.
    cortex_m::interrupt::disable();
    let mut core = cortex_m::Peripherals::take().unwrap();
    core.SYST.disable_interrupt();
    core.SYST.disable_counter();
    let dp = pac::Peripherals::take().unwrap();

    dp.CRM.apb2en().modify(|_, w| {
        w.iomux().set_bit().gpioa().set_bit().gpiob().set_bit().gpioc().set_bit();
        w.spi1().set_bit().tmr1().set_bit()
    });
    dp.CRM.apb1en().modify(|_, w| w.pwc().set_bit());
    let (gpioa, gpiob, gpioc) = (&*dp.GPIOA, &*dp.GPIOB, &*dp.GPIOC);
    // Read back so the PWC clock is running before its status is read.
    let _ = dp.CRM.apb1en().read();
    let woke_from_standby = dp.PWC.ctrlsts().read().sef().bit_is_set();
    dp.PWC.ctrl().modify(|_, w| w.clsef().set_bit().clswef().set_bit());

    gpio::set_high(gpioc, POWER_HOLD);
    gpio::configure(gpioc, POWER_HOLD, gpio::OUTPUT);

    clock::init(&dp.CRM);
    // JTAG off and SWD on frees PA15, PB3 and PB4. SPI1 moves to PB3/PB4/PB5.
    dp.IOMUX.remap().modify(|_, w| unsafe { w.swjtag_mux().bits(0b010).spi1_mux0().set_bit() });

    gpio::set_high(gpiob, BACKLIGHT);
    gpio::configure(gpiob, BACKLIGHT, gpio::OUTPUT);
    gpio::set_low(gpiob, SPEAKER_AMP);
    gpio::configure(gpiob, SPEAKER_AMP, gpio::OUTPUT);
    for pin in PORT_A_BUTTONS {
        gpio::set_low(gpioa, pin);
        gpio::configure(gpioa, pin, gpio::INPUT_PULL);
    }
    for pin in PORT_C_BUTTONS {
        gpio::set_low(gpioc, pin);
        gpio::configure(gpioc, pin, gpio::INPUT_PULL);
    }
    for pin in [TIP, RING] {
        gpio::set_high(gpiob, pin);
        gpio::configure(gpiob, pin, gpio::INPUT_PULL);
    }
    gpio::set_low(gpiob, DECODER_TONE);
    gpio::configure(gpiob, DECODER_TONE, gpio::INPUT_PULL);
    for pin in [KEY_OUT_TIP, KEY_OUT_RING] {
        gpio::set_low(gpioa, pin);
        gpio::configure(gpioa, pin, gpio::OUTPUT);
    }
    for pin in [LED, DECODER_POWER, PB15] {
        gpio::set_low(gpiob, pin);
        gpio::configure(gpiob, pin, gpio::OUTPUT);
    }

    // The panel type saved with F2 at power-on, else the controller's ID, else option byte Data1.
    let mut bus = I2c::new();
    let mut record = [0u8; panel::RECORD_LEN];
    let saved = if eeprom::read(&mut bus, store::PANEL_ADDRESS, &mut record) { panel::decode(&record) } else { None };
    let mut lcd = Lcd::new(dp.SPI1);
    lcd.reset();
    let detected = panel::detect(&lcd.read_id());
    lcd.init(panel::choose(saved, detected, unsafe { read_volatile(0x1FFF_F806 as *const u8) }).0);
    lcd.clear(Rgb565::BLACK).unwrap();
    let readback = readback_lines(&mut lcd);

    let tone = Tone::new(&dp.TMR1, gpioa);

    let mut line = Line::new();
    show(&mut lcd, 0, "HamCube hardware probe 6");
    let (serial, flash_kb) = unsafe { (read_volatile(0x1FFF_F7F0 as *const u32), read_volatile(0x1FFF_F7E0 as *const u16)) };
    let id = dp.DEBUG.idcode().read().bits();
    let _ = write!(line.start(), "SN {serial:08X}  ID {id:08X}  flash {flash_kb}K");
    show(&mut lcd, 1, line.as_str());
    // User system data from 0x1FFFF800: FAP, SSB, Data0, Data1, EPP0..3, EOPB0, -, Data2, Data3.
    let _ = write!(line.start(), "OB");
    for i in 0..12 {
        let half = unsafe { read_volatile((0x1FFF_F800 + 2 * i) as *const u16) };
        let _ = write!(line, " {:02X}", half & 0xFF);
    }
    show(&mut lcd, 2, line.as_str());
    scan(&mut lcd, &mut bus, &mut line);

    for (row, text) in [
        (8, "F1  PWM audio test: sine, noise, QSB, QRM"),
        (9, "F2  codec bypass from input 1, tone, amp on"),
        (10, "F3  next output: OUT1+OUT2, OUT1, OUT2"),
        (11, "F4  stop tone, amp off"),
        (12, "Dial left/right: volume.  Hold click 1.5 s: power off"),
        (13, "Paddle echo: dit to PA2, dah to PA3, either to PB2"),
    ] {
        show(&mut lcd, row, text);
    }
    show(&mut lcd, STATUS_ROW, if woke_from_standby { "woke from standby" } else { "cold start" });
    show(&mut lcd, READBACK_ROW, "LCD read back: pin, framing pulled up and down, bits");
    for (row, line) in (READBACK_ROW + 1..).zip(&readback) {
        show(&mut lcd, row, line.as_str());
    }

    let mut output = 0;
    let mut volume = VOLUME_START;
    show_audio(&mut lcd, &mut line, output, volume);

    let mut previous = [0u16; 3];
    let start = [gpio::inputs(gpioa), gpio::inputs(gpiob), gpio::inputs(gpioc)];
    let mut changed = [0u16; 3];
    let mut first = true;
    let mut power_armed = false;
    let mut power_held = 0;
    loop {
        let now = [gpio::inputs(gpioa), gpio::inputs(gpiob), gpio::inputs(gpioc)];
        if now != previous || first {
            for i in 0..3 {
                changed[i] |= (now[i] ^ start[i]) & !DRIVEN[i];
            }
            let _ = write!(line.start(), "changed since start  A {:04X}  B {:04X}  C {:04X}", changed[0], changed[1], changed[2]);
            show(&mut lcd, CHANGED_ROW, line.as_str());
            let _ = write!(line.start(), "inputs  A {:04X}  B {:04X}  C {:04X}", now[0], now[1], now[2]);
            show(&mut lcd, 5, line.as_str());
            let state = |down: bool| if down { "DOWN" } else { "up  " };
            let _ = write!(
                line.start(),
                "paddle  dit {}  dah {}   PB14 {}",
                state(now[1] & (1 << TIP) == 0),
                state(now[1] & (1 << RING) == 0),
                state(now[1] & (1 << DECODER_TONE) != 0),
            );
            show(&mut lcd, 6, line.as_str());
        }
        let pressed_a = if first { 0 } else { now[0] & !previous[0] };
        let pressed_c = if first { 0 } else { now[2] & !previous[2] };
        previous = now;
        first = false;

        if pressed_a & (1 << F_KEYS[0]) != 0 {
            pwm_test(&dp.TMR1, gpioa, gpiob, gpioc, &mut lcd, &mut bus, &mut line, output, &mut volume);
            Tone::new(&dp.TMR1, gpioa);
            show(&mut lcd, STATUS_ROW, "PWM test ended, tone off, PB12 low");
            show_audio(&mut lcd, &mut line, output, volume);
            previous = [gpio::inputs(gpioa), gpio::inputs(gpiob), gpio::inputs(gpioc)];
        }
        if pressed_a & (1 << F_KEYS[1]) != 0 {
            let acked = es8388_bypass(&mut bus) + set_output(&mut bus, output) + set_volume(&mut bus, volume);
            gpio::set_high(gpiob, SPEAKER_AMP);
            tone.on();
            dump(&mut lcd, &mut bus, &mut line);
            let _ = write!(line.start(), "bypass: {acked}/{} writes acked, tone on", ES8388_BYPASS.len() + 5);
            show(&mut lcd, STATUS_ROW, line.as_str());
        }
        if pressed_a & (1 << F_KEYS[2]) != 0 {
            output = (output + 1) % OUTPUTS.len();
            set_output(&mut bus, output);
            show_audio(&mut lcd, &mut line, output, volume);
        }
        if pressed_a & (1 << F_KEYS[3]) != 0 {
            tone.off();
            gpio::set_low(gpiob, SPEAKER_AMP);
            show(&mut lcd, STATUS_ROW, "tone off, PB12 low");
        }
        // The dial click also raises PC14, so a click must not count as dial left.
        let dial_left = pressed_c & (1 << DIAL_LEFT) != 0 && !gpio::is_high(gpioa, POWER_BUTTON);
        let dial_right = pressed_c & (1 << DIAL_RIGHT) != 0;
        if dial_left || dial_right {
            volume = if dial_right { (volume + 1).min(VOLUME_MAX) } else { volume.saturating_sub(1) };
            set_volume(&mut bus, volume);
            show_audio(&mut lcd, &mut line, output, volume);
        }

        // The click is still down from switching on, so wait for one release first.
        if gpio::is_high(gpioa, POWER_BUTTON) {
            power_held += 1;
            if power_held % 10 == 0 {
                let _ = write!(line.start(), "click held {power_held}/{POWER_OFF_LOOPS} loops, armed {power_armed}");
                show(&mut lcd, STATUS_ROW, line.as_str());
            }
            if power_armed && power_held >= POWER_OFF_LOOPS {
                tone.off();
                show(&mut lcd, STATUS_ROW, "release to power off");
                while gpio::is_high(gpioa, POWER_BUTTON) {}
                let pending = enter_standby(&mut lcd, &dp.PWC, &mut core, gpiob, gpioc);
                let status = dp.PWC.ctrlsts().read().bits();
                let _ = write!(line.start(), "standby failed: PWC CS {status:03X}, pending exception {pending}");
                show(&mut lcd, STATUS_ROW, line.as_str());
                power_armed = false;
                power_held = 0;
            }
        } else {
            power_armed = true;
            power_held = 0;
        }
        for _ in 0..10 {
            echo_paddle(gpioa, gpiob);
            delay_ms(1);
        }
    }
}

/// Copies the paddle contacts to the keyed outputs, which go high while a contact is closed.
fn echo_paddle(gpioa: &gpio::Port, gpiob: &gpio::Port) {
    let dit = !gpio::is_high(gpiob, TIP);
    let dah = !gpio::is_high(gpiob, RING);
    for (pin, port, on) in [(KEY_OUT_TIP, gpioa, dit), (KEY_OUT_RING, gpioa, dah), (LED, gpiob, dit || dah)] {
        if on {
            gpio::set_high(port, pin);
        } else {
            gpio::set_low(port, pin);
        }
    }
}

/// Lists every I2C2 address that acknowledges, on rows 3 and 4.
fn scan(lcd: &mut Lcd, bus: &mut I2c, line: &mut Line) {
    const PER_ROW: usize = 15;
    let _ = write!(line.start(), "I2C2:");
    let mut found = 0;
    for address in 0x08..0x78 {
        if bus.probe(address) {
            if found == PER_ROW {
                show(lcd, 3, line.as_str());
                line.start();
            }
            let _ = write!(line, " {address:02X}");
            found += 1;
        }
    }
    if found == 0 {
        let _ = write!(line, " no devices");
    }
    if found > PER_ROW {
        show(lcd, 4, line.as_str());
    } else {
        show(lcd, 3, line.as_str());
        show(lcd, 4, "");
    }
}

/// Shows codec registers 0x00..0x3F and 0xFC..0xFF. `--` marks a register that was not acknowledged.
fn dump(lcd: &mut Lcd, bus: &mut I2c, line: &mut Line) {
    for (row, base) in [0x00u8, 0x10, 0x20, 0x30].into_iter().enumerate() {
        let _ = write!(line.start(), "{base:02X}:");
        for register in base..base + 16 {
            write_register(line, bus.read_register(CODEC, register));
        }
        show(lcd, DUMP_ROW + row as i32, line.as_str());
    }
    let _ = write!(line.start(), "FC:");
    for register in 0xFC..=0xFF {
        write_register(line, bus.read_register(CODEC, register));
    }
    show(lcd, DUMP_ROW + 4, line.as_str());
}

fn write_register(line: &mut Line, value: Option<u8>) {
    let _ = match value {
        Some(v) => write!(line, " {v:02X}"),
        None => write!(line, " --"),
    };
}

/// Returns how many register writes were acknowledged.
fn write_registers(bus: &mut I2c, writes: &[(u8, u8)]) -> usize {
    writes.iter().filter(|&&(register, value)| bus.write(CODEC, &[register, value])).count()
}

fn es8388_bypass(bus: &mut I2c) -> usize {
    write_registers(bus, ES8388_BYPASS)
}

fn set_output(bus: &mut I2c, output: usize) -> usize {
    write_registers(bus, &[(0x04, OUTPUTS[output].0)])
}

/// Same level on LOUT1, ROUT1, LOUT2 and ROUT2.
fn set_volume(bus: &mut I2c, volume: u8) -> usize {
    write_registers(bus, &[(0x2E, volume), (0x2F, volume), (0x30, volume), (0x31, volume)])
}

fn show_audio(lcd: &mut Lcd, line: &mut Line, output: usize, volume: u8) {
    let tenths_db = (volume as i32 - 0x1E) * 15;
    let sign = if tenths_db < 0 { "-" } else { "+" };
    let (whole, tenth) = (tenths_db.abs() / 10, tenths_db.abs() % 10);
    let _ = write!(line.start(), "audio   {}  volume {volume:02X}  {sign}{whole}.{tenth} dB", OUTPUTS[output].1);
    show(lcd, AUDIO_ROW, line.as_str());
}

/// The vendor power off: backlight, PC13 and speaker amplifier low, then standby until the dial is clicked.
/// Wakeup from standby is a reset, so a return means standby was refused. Returns the VECTPENDING field.
fn enter_standby(
    lcd: &mut Lcd,
    pwc: &pac::PWC,
    core: &mut cortex_m::Peripherals,
    gpiob: &gpio::Port,
    gpioc: &gpio::Port,
) -> u32 {
    lcd.sleep();
    gpio::set_low(gpiob, BACKLIGHT);
    gpio::set_low(gpioc, POWER_HOLD);
    gpio::set_low(gpiob, SPEAKER_AMP);
    // An enabled, pending interrupt ends WFI at once even with interrupts masked; the bootloader leaves USB set up.
    for i in 0..core.NVIC.icer.len() {
        unsafe {
            core.NVIC.icer[i].write(u32::MAX);
            core.NVIC.icpr[i].write(u32::MAX);
        }
    }
    cortex_m::peripheral::SCB::clear_pendst();
    cortex_m::peripheral::SCB::clear_pendsv();
    pwc.ctrl().modify(|_, w| w.clsef().set_bit());
    pwc.ctrlsts().modify(|_, w| w.swpen().set_bit());
    // Enabling the wakeup pin can set the wakeup flag, so clear it afterwards.
    pwc.ctrl().modify(|_, w| w.clswef().set_bit());
    pwc.ctrl().modify(|_, w| w.lpsel().set_bit());
    core.SCB.set_sleepdeep();
    cortex_m::asm::dsb();
    cortex_m::asm::wfi();

    let pending = core.SCB.icsr.read() >> 12 & 0x1FF;
    core.SCB.clear_sleepdeep();
    pwc.ctrl().modify(|_, w| w.lpsel().clear_bit());
    gpio::set_high(gpioc, POWER_HOLD);
    lcd.wake();
    gpio::set_high(gpiob, BACKLIGHT);
    pending
}

/// Writes `READBACK_PATTERN` at the top left and reads it back. A line for each of PB4 and PB5:
/// the framing found with the pin pulled up and pulled down (skipped bits, format, channel order),
/// then the first 64 bits read pulled up.
fn readback_lines(lcd: &mut Lcd) -> [Line; 2] {
    let area = Rectangle::new(Point::zero(), Size::new(panel::READBACK_PATTERN.len() as u32, 1));
    lcd.fill_contiguous(&area, panel::READBACK_PATTERN.map(|raw| Rgb565::from(RawU16::new(raw)))).unwrap();
    let [up, down] = [true, false].map(|pull_up| lcd.read_memory::<{ panel::READBACK_WORDS }>(&area, pull_up));
    core::array::from_fn(|pin| {
        let mut line = Line::new();
        let _ = write!(line, "PB{}", 4 + pin);
        for (pull, bits) in [("up", &up[pin]), ("dn", &down[pin])] {
            let _ = match panel::find_framing(bits, &panel::READBACK_PATTERN) {
                Some(f) => {
                    let format = if f.format == panel::PixelFormat::Rgb666 { 666 } else { 565 };
                    write!(line, " {pull} {}/{format}/{}", f.skip_bits, if f.bgr { "BGR" } else { "RGB" })
                }
                None => write!(line, " {pull} NONE"),
            };
        }
        let _ = write!(line, " {:08X}{:08X}", up[pin][0], up[pin][1]);
        line
    })
}

fn show(lcd: &mut Lcd, row: i32, text: &str) {
    let style = MonoTextStyleBuilder::new()
        .font(&FONT_6X10)
        .text_color(Rgb565::WHITE)
        .background_color(Rgb565::BLACK)
        .build();
    // Pad to the full width so a shorter line erases the old one.
    let mut padded = Line::new();
    let _ = write!(padded, "{text:<COLUMNS$}");
    Text::with_baseline(padded.as_str(), Point::new(1, row * ROW_HEIGHT), style, Baseline::Top)
        .draw(lcd)
        .unwrap();
}

/// TMR1 counts 0..255 at the full timer clock, so PA8 carries a 281 kHz pulse train whose duty
/// sets the level; the codec input and the headphones smooth it into audio.
const PWM_STEPS: u32 = 256;
/// The duty changes once every this many pulses: 17.6 kHz samples.
const PWM_REPEAT: u32 = 16;
const SAMPLE_HZ: f32 = (clock::TIMER_HZ / PWM_STEPS / PWM_REPEAT) as f32;
const PWM_MID: f32 = (PWM_STEPS / 2) as f32;
/// Samples between button polls: about 15 ms.
const POLL_SAMPLES: u32 = 256;
/// Key edges ramp over 5 ms, so they don't click.
const RAMP_SAMPLES: f32 = SAMPLE_HZ * 0.005;
/// RMS of the band-pass filter's output for white noise in -1..1, measured in simulation.
const NOISE_RMS: f32 = 3.9;
const NOISE_BANDWIDTH_HZ: f32 = 500.0;
const QSB_HZ: f32 = 0.2;
const QRM_HZ: f32 = 830.0;

struct Sound {
    name: &'static str,
    /// Peak level of the main signal, 0..1 of full scale.
    signal: f32,
    /// RMS level of the noise.
    noise: f32,
    qsb: bool,
    qrm: bool,
    square: bool,
}

const SOUNDS: [Sound; 6] = [
    Sound { name: "sine, keyed", signal: 0.9, noise: 0.0, qsb: false, qrm: false, square: false },
    Sound { name: "square, keyed (today's tone)", signal: 0.9, noise: 0.0, qsb: false, qrm: false, square: true },
    Sound { name: "band noise only", signal: 0.0, noise: 0.25, qsb: false, qrm: false, square: false },
    Sound { name: "sine in noise", signal: 0.5, noise: 0.2, qsb: false, qrm: false, square: false },
    Sound { name: "sine in noise with QSB", signal: 0.6, noise: 0.2, qsb: true, qrm: false, square: false },
    Sound { name: "two stations (QRM) in noise", signal: 0.45, noise: 0.12, qsb: false, qrm: true, square: false },
];

/// Sine of a small angle, from its Taylor series: within 0.01% below 0.5 rad.
fn sin_small(x: f32) -> f32 {
    x - x * x * x / 6.0
}

/// Sine oscillator with no table: the "magic circle" rotation, stable in single precision.
struct Oscillator {
    sin: f32,
    cos: f32,
    k: f32,
}

impl Oscillator {
    fn new(hz: f32) -> Self {
        Oscillator { sin: 0.0, cos: 1.0, k: 2.0 * sin_small(core::f32::consts::PI * hz / SAMPLE_HZ) }
    }

    fn next(&mut self) -> f32 {
        self.sin += self.k * self.cos;
        self.cos -= self.k * self.sin;
        self.sin
    }
}

/// Keys `text` in Morse forever and gives the key envelope, 0..1, one sample at a time.
struct Sender {
    text: &'static [u8],
    unit: u32,
    char: usize,
    element: usize,
    on: bool,
    left: u32,
    level: f32,
}

impl Sender {
    fn new(text: &'static [u8], wpm: f32) -> Self {
        Sender { text, unit: (1.2 / wpm * SAMPLE_HZ) as u32, char: 0, element: 0, on: false, left: 0, level: 0.0 }
    }

    fn next(&mut self) -> f32 {
        if self.left == 0 {
            self.advance();
        }
        self.left -= 1;
        let step = 1.0 / RAMP_SAMPLES;
        self.level = if self.on { (self.level + step).min(1.0) } else { (self.level - step).max(0.0) };
        self.level
    }

    fn advance(&mut self) {
        let c = self.text[self.char];
        let code = morse::code(c).unwrap_or("").as_bytes();
        if c == b' ' {
            // A word space is 7 units; the character before it already waited 3.
            self.on = false;
            self.left = 4 * self.unit;
            self.char = (self.char + 1) % self.text.len();
        } else if self.on {
            self.on = false;
            self.element += 1;
            self.left = if self.element < code.len() { self.unit } else { 3 * self.unit };
            if self.element == code.len() {
                self.element = 0;
                self.char = (self.char + 1) % self.text.len();
            }
        } else {
            self.on = true;
            self.left = if code[self.element] == b'-' { 3 } else { 1 } * self.unit;
        }
    }
}

/// White noise through a two-pole band-pass at the tone pitch, like a receiver's CW filter.
struct BandNoise {
    rng: Rng,
    a1: f32,
    a2: f32,
    y1: f32,
    y2: f32,
}

impl BandNoise {
    fn new(hz: f32) -> Self {
        let r = 1.0 - core::f32::consts::PI * NOISE_BANDWIDTH_HZ / SAMPLE_HZ;
        let w = 2.0 * core::f32::consts::PI * hz / SAMPLE_HZ;
        let cos_w = 1.0 - w * w / 2.0 + w * w * w * w / 24.0;
        BandNoise { rng: Rng::new(12345), a1: 2.0 * r * cos_w, a2: r * r, y1: 0.0, y2: 0.0 }
    }

    /// One sample with an RMS of about 1.
    fn next(&mut self) -> f32 {
        let x = self.rng.below(65536) as f32 / 32768.0 - 1.0;
        let y = x + self.a1 * self.y1 - self.a2 * self.y2;
        self.y2 = self.y1;
        self.y1 = y;
        y / NOISE_RMS
    }
}

struct Synth {
    tone: Oscillator,
    sender: Sender,
    qrm_tone: Oscillator,
    qrm_sender: Sender,
    qsb: Oscillator,
    noise: BandNoise,
}

impl Synth {
    fn new() -> Self {
        Synth {
            tone: Oscillator::new(TONE_HZ as f32),
            sender: Sender::new(b"CQ CQ DE K5MME K5MME K ", 20.0),
            qrm_tone: Oscillator::new(QRM_HZ),
            qrm_sender: Sender::new(b"TEST W1AW TEST ", 26.0),
            qsb: Oscillator::new(QSB_HZ),
            noise: BandNoise::new(TONE_HZ as f32),
        }
    }

    /// The next sample of `sound`, -1..1.
    fn next(&mut self, sound: &Sound) -> f32 {
        let wave = self.tone.next();
        let wave = if sound.square { if wave >= 0.0 { 1.0 } else { -1.0 } } else { wave };
        let fade = if sound.qsb { 0.6 + 0.4 * self.qsb.next() } else { 1.0 };
        let mut out = sound.signal * fade * self.sender.next() * wave;
        let qrm = self.qrm_sender.next() * self.qrm_tone.next();
        if sound.qrm {
            out += sound.signal * 0.7 * qrm;
        }
        out += sound.noise * self.noise.next();
        out.clamp(-1.0, 1.0)
    }
}

/// Plays the test sounds through PA8 as PWM audio until F4. F1 picks the next sound and the dial
/// sets the codec volume.
#[allow(clippy::too_many_arguments)]
fn pwm_test(
    timer: &pac::tmr1::RegisterBlock,
    gpioa: &gpio::Port,
    gpiob: &gpio::Port,
    gpioc: &gpio::Port,
    lcd: &mut Lcd,
    bus: &mut I2c,
    line: &mut Line,
    output: usize,
    volume: &mut u8,
) {
    es8388_bypass(bus);
    set_output(bus, output);
    set_volume(bus, *volume);
    gpio::set_high(gpiob, SPEAKER_AMP);
    unsafe {
        timer.cctrl().write(|w| w.c1en().clear_bit());
        timer.div().write(|w| w.bits(0));
        timer.pr().write(|w| w.bits(PWM_STEPS as u16 - 1));
        timer.rpr().write(|w| w.bits(PWM_REPEAT as u8 - 1));
        timer.c1dt().write(|w| w.bits(PWM_MID as u16));
        timer.swevt().write(|w| w.ovfswtr().set_bit());
        timer.ists().write(|w| w.bits(!1));
    }
    timer.cctrl().write(|w| w.c1en().set_bit());

    let mut synth = Synth::new();
    let mut sound = 0;
    let _ = write!(line.start(), "PWM: {}", SOUNDS[sound].name);
    show(lcd, STATUS_ROW, line.as_str());
    show(lcd, STATUS_ROW + 1, "F1 next sound  F4 stop  dial: volume");
    let mut previous = [gpio::inputs(gpioa), gpio::inputs(gpioc)];
    let mut samples = 0u32;
    loop {
        // The overflow flag sets at each update event: every PWM_REPEAT pulses.
        while timer.ists().read().ovfif().bit_is_clear() {}
        unsafe { timer.ists().write(|w| w.bits(!1)) };
        let out = synth.next(&SOUNDS[sound]);
        unsafe { timer.c1dt().write(|w| w.bits((PWM_MID + out * (PWM_MID - 1.0)) as u16)) };

        samples += 1;
        if !samples.is_multiple_of(POLL_SAMPLES) {
            continue;
        }
        let now = [gpio::inputs(gpioa), gpio::inputs(gpioc)];
        let pressed_a = now[0] & !previous[0];
        let pressed_c = now[1] & !previous[1];
        previous = now;
        if pressed_a & (1 << F_KEYS[3]) != 0 {
            break;
        }
        if pressed_a & (1 << F_KEYS[0]) != 0 {
            sound = (sound + 1) % SOUNDS.len();
            let _ = write!(line.start(), "PWM: {}", SOUNDS[sound].name);
            show(lcd, STATUS_ROW, line.as_str());
        }
        let dial_left = pressed_c & (1 << DIAL_LEFT) != 0 && !gpio::is_high(gpioa, POWER_BUTTON);
        let dial_right = pressed_c & (1 << DIAL_RIGHT) != 0;
        if dial_left || dial_right {
            *volume = if dial_right { (*volume + 1).min(VOLUME_MAX) } else { volume.saturating_sub(1) };
            set_volume(bus, *volume);
            show_audio(lcd, line, output, *volume);
        }
    }
    timer.cctrl().write(|w| w.c1en().clear_bit());
    gpio::set_low(gpiob, SPEAKER_AMP);
    show(lcd, STATUS_ROW + 1, "");
}

/// 50% square wave on PA8 from TMR1 channel 1.
struct Tone<'a> {
    timer: &'a pac::tmr1::RegisterBlock,
}

impl<'a> Tone<'a> {
    fn new(timer: &'a pac::tmr1::RegisterBlock, gpioa: &gpio::Port) -> Self {
        let period = (TIMER_TICK_HZ / TONE_HZ) as u16;
        unsafe {
            timer.div().write(|w| w.bits((clock::TIMER_HZ / TIMER_TICK_HZ - 1) as u16));
            timer.pr().write(|w| w.bits(period - 1));
            timer.c1dt().write(|w| w.bits(period / 2));
            timer.rpr().write(|w| w.bits(0));
            // C1OCTRL = 110 (PWM mode A), C1OBEN = 1 (buffered compare).
            timer.cm1_output().write(|w| w.bits(0b110 << 4 | 1 << 3));
            timer.swevt().write(|w| w.ovfswtr().set_bit());
            timer.brk().write(|w| w.oen().set_bit());
            timer.ctrl1().write(|w| w.tmren().set_bit());
        }
        gpio::configure(gpioa, SIDETONE, gpio::PERIPHERAL_OUTPUT);
        Tone { timer }
    }

    fn on(&self) {
        self.timer.cctrl().write(|w| w.c1en().set_bit());
    }

    fn off(&self) {
        self.timer.cctrl().write(|w| w.c1en().clear_bit());
    }
}

/// Fixed-size text buffer for formatting without a heap.
struct Line {
    buf: [u8; 64],
    len: usize,
}

impl Line {
    fn new() -> Self {
        Line { buf: [0; 64], len: 0 }
    }

    fn start(&mut self) -> &mut Self {
        self.len = 0;
        self
    }

    fn as_str(&self) -> &str {
        core::str::from_utf8(&self.buf[..self.len]).unwrap()
    }
}

impl Write for Line {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let end = self.len + s.len();
        if end > self.buf.len() {
            return Err(core::fmt::Error);
        }
        self.buf[self.len..end].copy_from_slice(s.as_bytes());
        self.len = end;
        Ok(())
    }
}
