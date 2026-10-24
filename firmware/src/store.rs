//! What the EEPROM holds and where, loading it at startup, and writing records back.

use core::sync::atomic::{AtomicBool, AtomicU8, Ordering};

use pocketcw_logic::{
    battery::{self, Gauge},
    letters::{self, Letters},
    memories::{self, Memories},
    panel::{self, Panel},
    progress::{self, History},
    settings::{self, Settings},
};

use crate::eeprom;
use crate::i2c::I2c;

// The vendor firmware uses 0x000..0x368. Each record owns the bytes up to the next address, and
// startup reads all of them, so a longer record from later firmware still reads whole.
pub const LETTERS_ADDRESS: u16 = 0x400;
pub const HISTORY_ADDRESS: u16 = 0x480;
const _: () = assert!(LETTERS_ADDRESS as usize + letters::RECORD_LEN <= HISTORY_ADDRESS as usize);
pub const MEMORIES_ADDRESS: u16 = 0x600;
const _: () = assert!(HISTORY_ADDRESS as usize + progress::RECORD_LEN <= MEMORIES_ADDRESS as usize);
/// One address per settings copy.
const SETTINGS_ADDRESSES: [u16; settings::COPIES] = [0x700, 0x740];
const _: () = assert!(MEMORIES_ADDRESS as usize + memories::RECORD_LEN <= SETTINGS_ADDRESSES[0] as usize);
const _: () = assert!(SETTINGS_ADDRESSES[0] as usize + settings::RECORD_LEN <= SETTINGS_ADDRESSES[1] as usize);
pub const BATTERY_ADDRESS: u16 = 0x780;
const _: () = assert!(SETTINGS_ADDRESSES[1] as usize + settings::RECORD_LEN <= BATTERY_ADDRESS as usize);
const BOOT_COUNT_ADDRESS: u16 = 0x7A0;
const _: () = assert!(BATTERY_ADDRESS as usize + battery::RECORD_LEN <= BOOT_COUNT_ADDRESS as usize);
const _: () = assert!(BOOT_COUNT_ADDRESS as usize % eeprom::PAGE + settings::BOOT_RECORD_LEN <= eeprom::PAGE);
pub const PANEL_ADDRESS: u16 = 0x7B0;
const _: () = assert!(BOOT_COUNT_ADDRESS as usize + settings::BOOT_RECORD_LEN <= PANEL_ADDRESS as usize);
const _: () = assert!(PANEL_ADDRESS as usize % eeprom::PAGE + panel::RECORD_LEN <= eeprom::PAGE);
/// The end of the panel record's bytes; the rest of the EEPROM is free.
const PANEL_END: u16 = 0x7C0;
const _: () = assert!(PANEL_ADDRESS as usize + panel::RECORD_LEN <= PANEL_END as usize);

/// Bytes a record at `address` owns, up to `next`.
const fn span(address: u16, next: u16) -> usize {
    (next - address) as usize
}

/// False when a startup read failed: a write could then replace a good record with defaults.
static WRITABLE: AtomicBool = AtomicBool::new(false);
/// The save count of the next settings save, which picks its copy.
static SETTINGS_COUNT: AtomicU8 = AtomicU8::new(0);

/// What the EEPROM held at startup.
pub struct Saved {
    pub settings: Settings,
    pub letters: Letters,
    pub history: History,
    pub memories: Memories,
    /// The learned full-charge battery reading.
    pub battery_full: Option<u16>,
    /// The panel chosen with F2 at power-on, when it differs from the board's.
    pub panel: Option<Panel>,
}

/// Reads every record and counts this boot. Also returns whether the EEPROM worked. An
/// unreadable record reads as blank, so the device still starts, and every write is then refused.
pub fn load(bus: &mut I2c) -> (Saved, bool) {
    let mut reads_ok = true;
    let mut read = |bus: &mut I2c, address: u16, record: &mut [u8]| {
        if !eeprom::read(bus, address, record) {
            reads_ok = false;
            record.fill(0xFF);
        }
    };
    let mut copies = [[0u8; span(SETTINGS_ADDRESSES[0], SETTINGS_ADDRESSES[1])]; settings::COPIES];
    for (copy, &address) in copies.iter_mut().zip(&SETTINGS_ADDRESSES) {
        read(bus, address, copy);
    }
    let (settings, count) = Settings::load(copies.each_ref().map(|c| &c[..]));
    SETTINGS_COUNT.store(count, Ordering::Relaxed);
    let mut settings = settings.unwrap_or_default();
    let mut slot = [0u8; span(LETTERS_ADDRESS, HISTORY_ADDRESS)];
    read(bus, LETTERS_ADDRESS, &mut slot);
    let letters = Letters::decode(&slot).unwrap_or_default();
    let mut history_record = [0u8; progress::RECORD_LEN];
    read(bus, HISTORY_ADDRESS, &mut history_record);
    let history = History::decode(&history_record);
    let mut slot = [0u8; span(MEMORIES_ADDRESS, SETTINGS_ADDRESSES[0])];
    read(bus, MEMORIES_ADDRESS, &mut slot);
    let memories = Memories::decode(&slot).unwrap_or_else(|| Memories::new(settings.callsign()));
    let mut slot = [0u8; span(BATTERY_ADDRESS, BOOT_COUNT_ADDRESS)];
    read(bus, BATTERY_ADDRESS, &mut slot);
    let battery_full = Gauge::decode(&slot);
    let mut slot = [0u8; span(PANEL_ADDRESS, PANEL_END)];
    read(bus, PANEL_ADDRESS, &mut slot);
    let panel = panel::decode(&slot);
    let mut slot = [0u8; span(BOOT_COUNT_ADDRESS, PANEL_ADDRESS)];
    read(bus, BOOT_COUNT_ADDRESS, &mut slot);
    let boot_count = settings::decode_boot_count(&slot).unwrap_or(settings.boot_count);
    settings.boot_count = boot_count.wrapping_add(1);
    WRITABLE.store(reads_ok, Ordering::Relaxed);
    let ok = write(bus, BOOT_COUNT_ADDRESS, &settings::encode_boot_count(settings.boot_count));
    (Saved { settings, letters, history, memories, battery_full, panel }, ok)
}

/// Writes a record; false when it failed or a startup read failed.
pub fn write(bus: &mut I2c, address: u16, record: &[u8]) -> bool {
    WRITABLE.load(Ordering::Relaxed) && eeprom::write(bus, address, record)
}

/// Saves `settings` to the copy not holding the newest; false when the write failed.
pub fn write_settings(bus: &mut I2c, settings: &Settings) -> bool {
    let count = SETTINGS_COUNT.load(Ordering::Relaxed);
    let written = write(bus, SETTINGS_ADDRESSES[count as usize % settings::COPIES], &settings.encode(count));
    if written {
        SETTINGS_COUNT.store(count.wrapping_add(1), Ordering::Relaxed);
    }
    written
}

pub fn history_slot_address(slot: usize) -> u16 {
    HISTORY_ADDRESS + (slot * progress::SLOT_LEN) as u16
}
