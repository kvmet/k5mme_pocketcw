//! 24C16 EEPROM: 2 KB as eight 256-byte blocks at I2C addresses 0x50..0x57, 16-byte pages.

use crate::clock::delay_us;
use crate::i2c::I2c;

const BASE_ADDRESS: u8 = 0x50;
pub const PAGE: usize = 16;
const BLOCK: usize = 256;
pub const SIZE: usize = 8 * BLOCK;

fn device(address: u16) -> u8 {
    BASE_ADDRESS | ((address >> 8) & 7) as u8
}

/// Fills `buf` from `address`, one block at a time: each block has its own I2C address.
pub fn read(bus: &mut I2c, mut address: u16, mut buf: &mut [u8]) -> bool {
    while !buf.is_empty() {
        let room = BLOCK - address as usize % BLOCK;
        let (chunk, rest) = buf.split_at_mut(room.min(buf.len()));
        if !bus.read(device(address), address as u8, chunk) {
            return false;
        }
        address += chunk.len() as u16;
        buf = rest;
    }
    true
}

/// Writes `data` at `address` page by page, waiting out each write cycle.
pub fn write(bus: &mut I2c, mut address: u16, mut data: &[u8]) -> bool {
    while !data.is_empty() {
        let room = PAGE - address as usize % PAGE;
        let (chunk, rest) = data.split_at(room.min(data.len()));
        let mut frame = [0u8; PAGE + 1];
        frame[0] = address as u8;
        frame[1..=chunk.len()].copy_from_slice(chunk);
        if !bus.write(device(address), &frame[..=chunk.len()]) || !wait_ready(bus, address) {
            return false;
        }
        address += chunk.len() as u16;
        data = rest;
    }
    true
}

/// The chip ignores its address until the write cycle ends; the datasheet maximum is 5 ms.
fn wait_ready(bus: &mut I2c, address: u16) -> bool {
    (0..20).any(|_| {
        delay_us(500);
        bus.probe(device(address))
    })
}
