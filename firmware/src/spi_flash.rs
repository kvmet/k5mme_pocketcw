//! The 16 MB SPI flash on SPI1, beside the LCD, with standard commands: read 0x03, page program
//! 0x02, 4 KB sector erase 0x20. Every command sets the divider and PB4 itself, since the LCD
//! changes both, and leaves the divider at the LCD's.

use at32f4xx_pac::at32f403a as pac;

use crate::gpio::{self, port_b};
use crate::pins::{FLASH_CS, LCD_MISO as MISO};

pub const SECTOR: u32 = 4096;
pub const PAGE: u32 = 256;

const READ: u8 = 0x03;
const PROGRAM: u8 = 0x02;
const ERASE_4K: u8 = 0x20;
const WRITE_ENABLE: u8 = 0x06;
const JEDEC_ID: u8 = 0x9F;
const READ_SFDP: u8 = 0x5A;
/// Status registers 1, 2 and 3.
const READ_STATUS: [u8; 3] = [0x05, 0x35, 0x15];
const BUSY: u8 = 1;

/// SPI1 dividers, as in `lcd.rs`: APB2 / 2 = 18 MHz, the LCD's, and / 8 = 4.5 MHz.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Clock {
    Mhz18 = 0b000,
    Mhz4_5 = 0b010,
}

pub struct SpiFlash<'a> {
    spi: &'a pac::SPI1,
    pub clock: Clock,
}

impl<'a> SpiFlash<'a> {
    pub(crate) fn new(spi: &'a pac::SPI1) -> Self {
        SpiFlash { spi, clock: Clock::Mhz18 }
    }

    pub fn jedec_id(&mut self) -> [u8; 3] {
        let mut id = [0; 3];
        self.command(&[JEDEC_ID], &mut id);
        id
    }

    /// Status register 1, 2 or 3, as `n` 0, 1 or 2.
    pub fn status(&mut self, n: usize) -> u8 {
        let mut status = [0];
        self.command(&[READ_STATUS[n]], &mut status);
        status[0]
    }

    pub fn is_busy(&mut self) -> bool {
        self.status(0) & BUSY != 0
    }

    pub fn read(&mut self, address: u32, buf: &mut [u8]) {
        let [_, a2, a1, a0] = address.to_be_bytes();
        self.command(&[READ, a2, a1, a0], buf);
    }

    /// The SFDP table from `address`, after one dummy byte.
    pub fn read_sfdp(&mut self, address: u32, buf: &mut [u8]) {
        let [_, a2, a1, a0] = address.to_be_bytes();
        self.command(&[READ_SFDP, a2, a1, a0, 0], buf);
    }

    /// Starts erasing the 4 KB sector at `address`. The chip is busy until it ends.
    pub fn start_erase(&mut self, address: u32) {
        let [_, a2, a1, a0] = address.to_be_bytes();
        self.command(&[WRITE_ENABLE], &mut []);
        self.command(&[ERASE_4K, a2, a1, a0], &mut []);
    }

    /// Starts programming `data`, which must not cross a page. The chip is busy until it ends.
    pub fn start_program(&mut self, address: u32, data: &[u8]) {
        let [_, a2, a1, a0] = address.to_be_bytes();
        self.command(&[WRITE_ENABLE], &mut []);
        self.begin();
        for &byte in [PROGRAM, a2, a1, a0].iter().chain(data) {
            self.transfer(byte);
        }
        self.end();
    }

    pub fn wait(&mut self) {
        while self.is_busy() {}
    }

    /// Sends `out`, then reads `into`.
    fn command(&mut self, out: &[u8], into: &mut [u8]) {
        self.begin();
        for &byte in out {
            self.transfer(byte);
        }
        for byte in into.iter_mut() {
            *byte = self.transfer(0);
        }
        self.end();
    }

    fn begin(&mut self) {
        let b = port_b();
        gpio::set_high(b, MISO);
        gpio::configure(b, MISO, gpio::INPUT_PULL);
        self.set_divider(self.clock as u8);
        // LCD writes leave a byte and the overrun flag in the receiver; reading data then status
        // clears both.
        let _ = self.spi.dt().read();
        let _ = self.spi.sts().read();
        gpio::set_low(b, FLASH_CS);
    }

    fn end(&mut self) {
        while self.spi.sts().read().bf().bit_is_set() {}
        gpio::set_high(port_b(), FLASH_CS);
        self.set_divider(Clock::Mhz18 as u8);
    }

    fn transfer(&mut self, byte: u8) -> u8 {
        while self.spi.sts().read().tdbe().bit_is_clear() {}
        self.spi.dt().write(|w| unsafe { w.bits(byte as u16) });
        while self.spi.sts().read().rdbf().bit_is_clear() {}
        self.spi.dt().read().bits() as u8
    }

    /// The divider changes only while SPI1 is off.
    fn set_divider(&mut self, divider: u8) {
        self.spi.ctrl1().modify(|_, w| w.spien().clear_bit());
        self.spi.ctrl1().modify(|_, w| unsafe { w.mdiv2_0().bits(divider) });
        self.spi.ctrl1().modify(|_, w| w.spien().set_bit());
    }
}
