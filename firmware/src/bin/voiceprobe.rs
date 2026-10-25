//! Voice probe: plays one voice's word clips through PA8 so you can score by ear whether speech
//! is clear enough, at two sample rates. Also times the synth's worst case against the sample
//! period. Build the clips with `tools/voiceclips.py`, then:
//! `VOICE_CLIPS=firmware/target/voice-kokoro sh tools/mkimage.sh voiceprobe`.
#![no_std]
#![no_main]

use core::fmt::Write;
use core::ptr::read_volatile;

use at32f4xx_pac::at32f403a as pac;
use cortex_m::peripheral::DWT;
use cortex_m_rt::entry;
use embedded_graphics::{
    mono_font::{MonoTextStyleBuilder, ascii::FONT_10X20},
    pixelcolor::Rgb565,
    prelude::*,
    text::{Baseline, Text as Label},
};

use pocketcw_firmware::audio::SAMPLE_HZ;
use pocketcw_firmware::clock::{self, delay_ms};
use pocketcw_firmware::codec::Codec;
use pocketcw_firmware::i2c::I2c;
use pocketcw_firmware::lcd::Lcd;
use pocketcw_firmware::pins::{BACKLIGHT, DIAL_LEFT, DIAL_RIGHT, F_KEYS, POWER_BUTTON, SIDETONE, SPEAKER_AMP};
use pocketcw_firmware::{eeprom, gpio, power, store};
use pocketcw_logic::adpcm::{Clip as Samples, Upsample};
use pocketcw_logic::synth::{self, Sound, Synth, VOICES};
use pocketcw_logic::text::Text;
use pocketcw_logic::{panel, rng::Rng};

struct Clip {
    word: &'static str,
    /// The output sample rate divided by the clip's rate.
    factor: u8,
    data: &'static [u8],
}

include!(concat!(env!("OUT_DIR"), "/clips.rs"));

/// Timer counts per pulse and pulses per sample, as in `audio.rs`.
const STEPS: u16 = 256;
const REPEAT: u8 = 16;
const MID: u16 = STEPS / 2;
const CYCLES_PER_SAMPLE: u32 = clock::SYSCLK_HZ / SAMPLE_HZ as u32;
const SYNTH_RUN: u32 = 20_000;
const FACTORS: [u8; 2] = [2, 3];
const MAX_WORDS: usize = 32;
/// Words a rate needs right of its 20 to pass.
const PASS: u8 = 19;
const VOLUME_START: u8 = 23;
const VOLUME_MAX: u8 = 44;
const HOLD_OFF_LOOPS: u32 = 150; // 1.5 s of 10 ms loops
const COLUMNS: usize = 32;
const ROW_HEIGHT: i32 = 20;

#[derive(Clone, Copy, PartialEq)]
enum Step {
    /// F1 plays the next word.
    Ready,
    /// The word played and is hidden.
    Heard,
    /// The word shows; you score it.
    Shown,
}

/// One sample rate's words in play order, and the score so far.
struct Rate {
    factor: u8,
    order: [u8; MAX_WORDS],
    words: usize,
    at: usize,
    right: u8,
}

impl Rate {
    fn new(factor: u8, rng: &mut Rng) -> Rate {
        let mut rate = Rate { factor, order: [0; MAX_WORDS], words: 0, at: 0, right: 0 };
        for (i, clip) in CLIPS.iter().enumerate() {
            if clip.factor == factor && rate.words < MAX_WORDS {
                rate.order[rate.words] = i as u8;
                rate.words += 1;
            }
        }
        for i in (1..rate.words).rev() {
            rate.order.swap(i, rng.below(i as u32 + 1) as usize);
        }
        rate
    }

    fn hz(&self) -> u32 {
        SAMPLE_HZ as u32 / self.factor as u32
    }

    fn done(&self) -> bool {
        self.at >= self.words
    }

    fn clip(&self) -> &'static Clip {
        &CLIPS[self.order[self.at] as usize]
    }

    fn result(&self, line: &mut Text<64>) {
        let _ = write!(line, "{} HZ {}/{}", self.hz(), self.right, self.at);
        if self.done() {
            let _ = write!(line, " {}", if self.right >= PASS { "PASS" } else { "FAIL" });
        }
    }
}

#[entry]
fn main() -> ! {
    // The vector table holds only default handlers, so the probe polls with interrupts off.
    cortex_m::interrupt::disable();
    let mut core = cortex_m::Peripherals::take().unwrap();
    core.SYST.disable_interrupt();
    core.SYST.disable_counter();
    core.DCB.enable_trace();
    core.DWT.enable_cycle_counter();
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

    let mut line = Text::<64>::new();
    show(&mut lcd, 0, VOICE);
    show(&mut lcd, 11, "TIMING THE SYNTH...");
    let (worst, mean) = time_synth();
    line.clear();
    let _ = write!(line, "SYNTH {worst}/{CYCLES_PER_SAMPLE} MEAN {mean}");
    show(&mut lcd, 11, line.as_str());

    let codec = Codec::detect(&mut bus);
    let mut volume = VOLUME_START;
    if let Some(codec) = codec {
        codec.init(&mut bus, volume);
        gpio::set_high(gpiob, SPEAKER_AMP);
    }
    start_pwm(&dp.TMR1);
    gpio::configure(gpioa, SIDETONE, gpio::PERIPHERAL_OUTPUT);

    let mut rng = Rng::new(DWT::cycle_count());
    let mut rates = FACTORS.map(|factor| Rate::new(factor, &mut rng));
    let mut current = 0;
    let mut step = Step::Ready;
    let mut previous = [gpio::inputs(gpioa), gpio::inputs(gpioc)];
    // The click is still down from switching on, so a hold counts only after one release.
    let mut armed = false;
    let mut held = 0;
    let mut redraw = true;
    loop {
        if redraw {
            draw(&mut lcd, &mut line, &rates, current, step, codec, volume);
            redraw = false;
        }
        delay_ms(10);
        let now = [gpio::inputs(gpioa), gpio::inputs(gpioc)];
        let pressed = |port: usize, pin: u32| now[port] & !previous[port] & (1 << pin) != 0;
        let released = |port: usize, pin: u32| previous[port] & !now[port] & (1 << pin) != 0;
        let f = |i: usize| pressed(0, F_KEYS[i]);
        let rate = &mut rates[current];
        match step {
            Step::Ready if f(0) && !rate.done() => {
                play(&dp.TMR1, rate.clip());
                (step, redraw) = (Step::Heard, true);
            }
            Step::Heard if f(1) => play(&dp.TMR1, rate.clip()),
            Step::Heard if f(2) => (step, redraw) = (Step::Shown, true),
            Step::Shown if f(0) || f(3) => {
                rate.right += f(0) as u8;
                rate.at += 1;
                (step, redraw) = (Step::Ready, true);
            }
            _ => {}
        }
        // The dial click also raises PC14, so a click must not count as dial left.
        let left = pressed(1, DIAL_LEFT) && now[0] & (1 << POWER_BUTTON) == 0;
        if (left || pressed(1, DIAL_RIGHT)) && let Some(codec) = codec {
            volume = if left { volume.saturating_sub(1) } else { (volume + 1).min(VOLUME_MAX) };
            codec.set_volume(&mut bus, volume);
            redraw = true;
        }
        if now[0] & (1 << POWER_BUTTON) != 0 {
            held += 1;
            if armed && held >= HOLD_OFF_LOOPS {
                show(&mut lcd, 10, "RELEASE TO POWER OFF");
                power::standby();
            }
        } else {
            if armed && released(0, POWER_BUTTON) && step != Step::Shown {
                current = (current + 1) % rates.len();
                (step, redraw) = (Step::Ready, true);
            }
            armed = true;
            held = 0;
        }
        previous = now;
    }
}

/// Worst and mean cycles of one synth sample with every voice keyed, square waves, and the top
/// noise and fading levels: the heaviest load the sample interrupt carries.
fn time_synth() -> (u32, u32) {
    let sound = Sound { square: true, noise: synth::MAX_LEVEL, fading: synth::MAX_LEVEL, tone_hz: 700 };
    let mut synth = Synth::new(SAMPLE_HZ, sound);
    synth.set_band(true);
    for voice in 0..VOICES {
        synth.key(voice, true, 500 + 100 * voice as u16);
    }
    let (mut worst, mut total) = (0, 0u64);
    for _ in 0..SYNTH_RUN {
        let start = DWT::cycle_count();
        let duty = synth::duty(synth.sample(), STEPS);
        let cycles = DWT::cycle_count().wrapping_sub(start);
        core::hint::black_box(duty);
        worst = worst.max(cycles);
        total += cycles as u64;
    }
    (worst, (total / SYNTH_RUN as u64) as u32)
}

/// PA8's pulse train at the silent level, as `audio.rs` sets it up, but polled: no interrupt.
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

/// Plays a clip, one duty per update event, then returns to the silent level.
fn play(timer: &pac::TMR1, clip: &Clip) {
    for sample in Upsample::new(Samples::new(clip.data), clip.factor) {
        while timer.ists().read().ovfif().bit_is_clear() {}
        timer.ists().write(|w| w.ovfif().clear_bit());
        let duty = synth::duty(sample as f32 / 32768.0, STEPS);
        unsafe { timer.c1dt().write(|w| w.bits(duty)) };
    }
    unsafe { timer.c1dt().write(|w| w.bits(MID)) };
}

fn draw(lcd: &mut Lcd, line: &mut Text<64>, rates: &[Rate; 2], current: usize, step: Step, codec: Option<Codec>, volume: u8) {
    let rate = &rates[current];
    line.clear();
    let _ = write!(line, "RATE {} HZ", rate.hz());
    show(lcd, 1, line.as_str());
    line.clear();
    let _ = write!(line, "WORD {}/{}  RIGHT {}", (rate.at + 1).min(rate.words), rate.words, rate.right);
    show(lcd, 2, line.as_str());
    let word = match step {
        _ if rate.words == 0 => "NO CLIPS AT THIS RATE",
        _ if rate.done() => "DONE: CLICK FOR NEXT RATE",
        Step::Shown => rate.clip().word,
        _ => "?????",
    };
    show(lcd, 4, word);
    let prompt = match step {
        _ if rate.done() => "",
        Step::Ready => "F1 PLAY",
        Step::Heard => "F2 AGAIN  F3 SHOW",
        Step::Shown => "F1 RIGHT  F4 WRONG",
    };
    show(lcd, 6, prompt);
    for (row, rate) in (8..).zip(rates) {
        line.clear();
        rate.result(line);
        show(lcd, row, line.as_str());
    }
    line.clear();
    match codec {
        Some(codec) => {
            let _ = write!(line, "{} VOL {volume}  CLICK: RATE", codec.name());
        }
        None => {
            let _ = write!(line, "NO CODEC FOUND");
        }
    }
    show(lcd, 10, line.as_str());
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
