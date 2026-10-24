//! USB probe: enumerates as a USB keyboard. The paddle holds Left Ctrl (dit) and Right Ctrl
//! (dah), as VBand reads them, and F1 types a test line.
#![no_std]
#![no_main]

use core::ptr::read_volatile;

use at32f4xx_pac::at32f403a as pac;
use cortex_m::peripheral::DWT;
use cortex_m_rt::entry;
use embedded_graphics::{
    mono_font::{MonoTextStyleBuilder, ascii::FONT_6X10},
    pixelcolor::Rgb565,
    prelude::*,
    text::{Baseline, Text as Label},
};
use usb_device::device::{StringDescriptors, UsbDeviceBuilder, UsbDeviceState, UsbVidPid};
use usbd_hid::descriptor::{KeyboardReport, SerializedDescriptor};
use usbd_hid::hid_class::HIDClass;

use pocketcw_firmware::clock::{self, SYSCLK_HZ};
use pocketcw_firmware::gpio;
use pocketcw_firmware::i2c::I2c;
use pocketcw_firmware::lcd::Lcd;
use pocketcw_firmware::pins::{BACKLIGHT, F_KEYS, LED, POWER_BUTTON, RING, SPEAKER_AMP, TIP};
use pocketcw_firmware::text::Text;
use pocketcw_firmware::{eeprom, power, store, usb};
use pocketcw_logic::panel;

/// pid.codes test VID and PID, for devices that never leave the bench.
const VID_PID: UsbVidPid = UsbVidPid(0x1209, 0x0001);
const TEST_LINE: &[u8] = b"cq test de pocketcw\n";
const LEFT_CTRL: u8 = 0x01;
const RIGHT_CTRL: u8 = 0x10;
const CYCLES_PER_MS: u32 = SYSCLK_HZ / 1000;
const POWER_OFF_MS: u32 = 1500;
/// Drawing a row stops USB polling for a few ms, so the counters redraw at most this often.
const SHOW_EVERY_MS: u32 = 250;

const COLUMNS: usize = 53;
const ROW_HEIGHT: i32 = 10;

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
        w.spi1().set_bit()
    });
    power::hold();
    clock::init(&dp.CRM);
    // JTAG off and SWD on frees PA15, PB3 and PB4. SPI1 moves to PB3/PB4/PB5.
    dp.IOMUX.remap().modify(|_, w| unsafe { w.swjtag_mux().bits(0b010).spi1_mux0().set_bit() });

    let (gpioa, gpiob) = (gpio::port_a(), gpio::port_b());
    for (port, pin) in [(gpiob, BACKLIGHT), (gpiob, SPEAKER_AMP), (gpiob, LED)] {
        gpio::set_low(port, pin);
        gpio::configure(port, pin, gpio::OUTPUT);
    }
    for pin in [POWER_BUTTON, F_KEYS[0]] {
        gpio::set_low(gpioa, pin);
        gpio::configure(gpioa, pin, gpio::INPUT_PULL);
    }
    for pin in [TIP, RING] {
        gpio::set_high(gpiob, pin);
        gpio::configure(gpiob, pin, gpio::INPUT_PULL);
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
    gpio::set_high(gpiob, BACKLIGHT);

    show(&mut lcd, 0, "PocketCW USB probe 1");
    let clock = match clock::running(&dp.CRM) {
        Some(clock::Source::Crystal) => "XTAL",
        Some(clock::Source::Internal) => "HICK: USB timing may be off",
        None => "8 MHz HICK, no PLL: USB cannot work",
    };
    show(&mut lcd, 1, Text::<64>::format(format_args!("clock  {clock}")).as_str());
    show(&mut lcd, 7, "dit: Left Ctrl   dah: Right Ctrl   (VBand)");
    show(&mut lcd, 8, "F1: types \"cq test de pocketcw\" and Enter");
    show(&mut lcd, 9, "Hold click 1.5 s: power off");

    let usb_bus = usb::bus();
    // No boot subclass: usbd-hid refuses every report from a boot keyboard in Report mode.
    let mut keyboard = HIDClass::new_ep_in(&usb_bus, KeyboardReport::desc(), 1);
    let mut device = UsbDeviceBuilder::new(&usb_bus, VID_PID)
        .strings(&[StringDescriptors::default().manufacturer("PocketCW").product("PocketCW USB probe")])
        .unwrap()
        .build();
    usb::attach();

    let mut state = UsbDeviceState::Default;
    let (mut resets, mut reports, mut errors) = (0u32, 0u32, 0u32);
    let mut last_error = None;
    // (modifier, key) of the last report the host took.
    let mut sent = (0u8, 0u8);
    // Even steps press TEST_LINE[step / 2], odd steps release it.
    let mut typing: Option<usize> = None;
    let mut f1_was = false;
    let mut click_armed = false;
    let mut click_since = 0u32;
    let mut shown_at = DWT::cycle_count();
    let mut dirty = true;
    loop {
        device.poll(&mut [&mut keyboard]);
        let now = DWT::cycle_count();
        if device.state() != state {
            if device.state() == UsbDeviceState::Default {
                resets += 1;
            }
            state = device.state();
            dirty = true;
        }

        let (a, b) = (gpio::inputs(gpioa), gpio::inputs(gpiob));
        let (dit, dah) = (b & (1 << TIP) == 0, b & (1 << RING) == 0);
        gpio::set(gpiob, LED, dit || dah);
        let f1 = a & (1 << F_KEYS[0]) != 0;
        if f1 && !f1_was && typing.is_none() {
            typing = Some(0);
        }
        f1_was = f1;

        let modifier = if dit { LEFT_CTRL } else { 0 } | if dah { RIGHT_CTRL } else { 0 };
        let key = match typing {
            Some(step) if step % 2 == 0 => usage(TEST_LINE[step / 2]),
            _ => 0,
        };
        let pending = typing.is_some() || (modifier, key) != sent;
        if state == UsbDeviceState::Configured && pending {
            let report = KeyboardReport { modifier, reserved: 0, leds: 0, keycodes: [key, 0, 0, 0, 0, 0] };
            match keyboard.push_input(&report) {
                Ok(_) => {
                    sent = (modifier, key);
                    reports += 1;
                    typing = typing.map(|step| step + 1).filter(|&step| step < 2 * TEST_LINE.len());
                    dirty = true;
                }
                Err(usb_device::UsbError::WouldBlock) => {}
                Err(error) => {
                    errors += 1;
                    last_error = Some(error);
                    dirty = true;
                }
            }
        }

        if dirty && now.wrapping_sub(shown_at) >= SHOW_EVERY_MS * CYCLES_PER_MS {
            let state_name = match state {
                UsbDeviceState::Default => "DEFAULT",
                UsbDeviceState::Addressed => "ADDRESSED",
                UsbDeviceState::Configured => "CONFIGURED",
                UsbDeviceState::Suspend => "SUSPEND",
            };
            show(&mut lcd, 3, Text::<64>::format(format_args!("state  {state_name}   bus resets {resets}")).as_str());
            show(&mut lcd, 4, Text::<64>::format(format_args!("reports sent {reports}   errors {errors}")).as_str());
            if let Some(error) = last_error {
                show(&mut lcd, 6, Text::<64>::format(format_args!("last error {error:?}")).as_str());
            }
            let down = |on: bool| if on { "DOWN" } else { "up" };
            show(&mut lcd, 5, Text::<64>::format(format_args!("paddle  dit {}  dah {}", down(dit), down(dah))).as_str());
            shown_at = now;
            dirty = false;
        }

        // The click is still down from switching on, so wait for one release first.
        if a & (1 << POWER_BUTTON) == 0 {
            click_armed = true;
            click_since = now;
        } else if click_armed && now.wrapping_sub(click_since) >= POWER_OFF_MS * CYCLES_PER_MS {
            usb::detach();
            show(&mut lcd, 5, "release to power off");
            power::standby();
        }
    }
}

/// The HID usage of a key on a US layout, typed without Shift.
fn usage(c: u8) -> u8 {
    match c {
        b'a'..=b'z' => 0x04 + c - b'a',
        b'1'..=b'9' => 0x1E + c - b'1',
        b'0' => 0x27,
        b'\n' => 0x28,
        b' ' => 0x2C,
        _ => 0,
    }
}

fn show(lcd: &mut Lcd, row: i32, text: &str) {
    let style = MonoTextStyleBuilder::new()
        .font(&FONT_6X10)
        .text_color(Rgb565::WHITE)
        .background_color(Rgb565::BLACK)
        .build();
    // Pad to the full width so a shorter line erases the old one.
    let padded = Text::<64>::format(format_args!("{text:<COLUMNS$}"));
    Label::with_baseline(padded.as_str(), Point::new(1, row * ROW_HEIGHT), style, Baseline::Top)
        .draw(lcd)
        .unwrap();
}
