//! The in-line logger on the SPI flash chip. See `docs/dev/roadmap.md`, "6. In-line logger".

use pocketcw_logic::decoder::Decoded;
use pocketcw_logic::log::{self, Cursor, Flash, MAX_FRAME, SECTORS, Source};
use pocketcw_logic::morse;
use pocketcw_logic::settings::Settings;

use crate::lcd::Lcd;
use crate::spi_flash::SpiFlash;

/// `log::Flash` on the chip, noting any write the driver refused.
struct Chip<'a> {
    flash: SpiFlash<'a>,
    refused: bool,
}

impl Flash for Chip<'_> {
    fn read(&mut self, address: u32, buf: &mut [u8]) {
        self.flash.read(address, buf);
    }

    fn erase(&mut self, address: u32) {
        self.refused |= !self.flash.start_erase(address);
    }

    fn program(&mut self, address: u32, data: &[u8]) {
        self.refused |= !self.flash.start_program(address, data);
    }
}

/// `log::Logger` on the chip. Each call is false when the driver refused a write.
pub struct Logger(log::Logger);

impl Logger {
    pub fn open(lcd: &mut Lcd) -> Logger {
        Logger(log::Logger::open(&mut chip(lcd), SECTORS, VERSION))
    }

    pub fn push_decoded(&mut self, lcd: &mut Lcd, s: &Settings, source: Source, decoded: Decoded, now: u32) -> bool {
        let text = match &decoded {
            Decoded::Char(c) => morse::text(c),
            Decoded::Unknown => "*",
            Decoded::Space => " ",
        };
        written(lcd, |flash| self.0.push(flash, s.callsign(), source, text, now))
    }

    pub fn push_memory(&mut self, lcd: &mut Lcd, s: &Settings, text: &str, now: u32) -> bool {
        written(lcd, |flash| self.0.push_memory(flash, s.callsign(), text, now))
    }

    pub fn played(&mut self, now: u32) {
        self.0.played(now);
    }

    pub fn deadline(&self) -> Option<u32> {
        self.0.deadline()
    }

    pub fn timeout(&mut self, lcd: &mut Lcd, s: &Settings, now: u32) -> bool {
        written(lcd, |flash| self.0.timeout(flash, s.callsign(), now))
    }

    /// Writes every open run and waits for the chip, before power off.
    pub fn close(&mut self, lcd: &mut Lcd, s: &Settings, now: u32) -> bool {
        let written = written(lcd, |flash| self.0.close(flash, s.callsign(), now));
        lcd.spi_flash().wait();
        written
    }

    pub fn cursor(&self) -> Cursor {
        self.0.cursor()
    }

    pub fn next_frame<'a>(&self, lcd: &mut Lcd, cursor: &mut Cursor, buf: &'a mut [u8; MAX_FRAME]) -> Option<&'a [u8]> {
        self.0.next_frame(&mut chip(lcd), cursor, buf)
    }
}

fn chip(lcd: &mut Lcd) -> Chip<'_> {
    Chip { flash: lcd.spi_flash(), refused: false }
}

fn written(lcd: &mut Lcd, write: impl FnOnce(&mut Chip)) -> bool {
    let mut chip = chip(lcd);
    write(&mut chip);
    !chip.refused
}

const VERSION: [u8; 3] = [
    number(env!("CARGO_PKG_VERSION_MAJOR")),
    number(env!("CARGO_PKG_VERSION_MINOR")),
    number(env!("CARGO_PKG_VERSION_PATCH")),
];

const fn number(digits: &str) -> u8 {
    let bytes = digits.as_bytes();
    let mut value = 0;
    let mut i = 0;
    while i < bytes.len() {
        value = value * 10 + (bytes[i] - b'0');
        i += 1;
    }
    value
}
