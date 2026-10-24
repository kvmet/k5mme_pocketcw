use crate::clock::delay_us;
use crate::gpio::{self, port_b};
use crate::pins::{I2C_SCL as SCL, I2C_SDA as SDA};

const HALF_PERIOD_US: u32 = 5;

/// Bit-banged I2C master on the I2C2 pins, about 65 kHz: each bit waits HALF_PERIOD_US three
/// times. Relies on the board's pull-ups.
pub struct I2c {
    _private: (),
}

impl I2c {
    /// Takes over the bus pins and frees a stuck device: hardware setup, not a default value.
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let port = port_b();
        for pin in [SCL, SDA] {
            gpio::set_high(port, pin);
            gpio::configure(port, pin, gpio::OUTPUT_OPEN_DRAIN);
        }
        let mut bus = I2c { _private: () };
        bus.recover();
        bus
    }

    /// Clocks out a device left mid-byte holding SDA low.
    fn recover(&mut self) {
        for _ in 0..9 {
            if gpio::is_high(port_b(), SDA) {
                break;
            }
            self.scl(false);
            self.scl(true);
        }
        self.stop();
    }

    fn scl(&mut self, high: bool) {
        if high {
            gpio::set_high(port_b(), SCL);
        } else {
            gpio::set_low(port_b(), SCL);
        }
        delay_us(HALF_PERIOD_US);
    }

    /// Waits after the change, so data is set up before the clock next rises.
    fn sda(&mut self, high: bool) {
        if high {
            gpio::set_high(port_b(), SDA);
        } else {
            gpio::set_low(port_b(), SDA);
        }
        delay_us(HALF_PERIOD_US);
    }

    fn start(&mut self) {
        self.sda(true);
        self.scl(true);
        self.sda(false);
        delay_us(HALF_PERIOD_US);
        self.scl(false);
    }

    fn stop(&mut self) {
        self.sda(false);
        self.scl(true);
        self.sda(true);
        delay_us(HALF_PERIOD_US);
    }

    /// Returns true when the device acknowledges.
    fn write_byte(&mut self, byte: u8) -> bool {
        for bit in (0..8).rev() {
            self.sda(byte & (1 << bit) != 0);
            self.scl(true);
            self.scl(false);
        }
        self.sda(true);
        self.scl(true);
        let ack = !gpio::is_high(port_b(), SDA);
        self.scl(false);
        ack
    }

    /// Writes `bytes` to the 7-bit `address`. Returns true when every byte was acknowledged.
    pub fn write(&mut self, address: u8, bytes: &[u8]) -> bool {
        self.start();
        let mut acked = self.write_byte(address << 1);
        for &b in bytes {
            if !acked {
                break;
            }
            acked = self.write_byte(b);
        }
        self.stop();
        acked
    }

    pub fn probe(&mut self, address: u8) -> bool {
        self.write(address, &[])
    }

    /// Reads one register: register address write, repeated start, one byte read. None on a missing ACK.
    pub fn read_register(&mut self, address: u8, register: u8) -> Option<u8> {
        let mut value = [0];
        self.read(address, register, &mut value).then_some(value[0])
    }

    /// Fills `buf` starting at `register`. Returns false when the device does not acknowledge.
    pub fn read(&mut self, address: u8, register: u8, buf: &mut [u8]) -> bool {
        self.start();
        let selected = self.write_byte(address << 1)
            && self.write_byte(register)
            && {
                self.start();
                self.write_byte(address << 1 | 1)
            };
        if selected {
            let last = buf.len().saturating_sub(1);
            for (i, byte) in buf.iter_mut().enumerate() {
                *byte = self.read_byte(i != last);
            }
        }
        self.stop();
        selected
    }

    /// Reads a byte. `ack` asks for another one; a NACK ends the read.
    fn read_byte(&mut self, ack: bool) -> u8 {
        self.sda(true);
        let mut byte = 0;
        for _ in 0..8 {
            self.scl(true);
            byte = byte << 1 | gpio::is_high(port_b(), SDA) as u8;
            self.scl(false);
        }
        self.sda(!ack);
        self.scl(true);
        self.scl(false);
        self.sda(true);
        byte
    }
}
