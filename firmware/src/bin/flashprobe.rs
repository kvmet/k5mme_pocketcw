//! SPI flash probe: the chip's ID, status registers and SFDP table, a scan of all 16 MB for used
//! sectors, an erase and program test, and markers that show what a bootloader install erases.
//! It erases only sectors that read blank or that carry its own tags.
#![no_std]
#![no_main]

use core::fmt::Write;
use core::ops::Range;
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

use pocketcw_firmware::clock::{self, delay_ms};
use pocketcw_firmware::i2c::I2c;
use pocketcw_firmware::lcd::Lcd;
use pocketcw_firmware::pins::{BACKLIGHT, F_KEYS, POWER_BUTTON, SPEAKER_AMP};
use pocketcw_firmware::spi_flash::{Clock, PAGE, SECTOR, SpiFlash};
use pocketcw_firmware::{eeprom, gpio, power, store};
use pocketcw_logic::text::Text;
use pocketcw_logic::{panel, sfdp};

const FLASH_BYTES: u32 = 16 << 20;
/// The free area between the vendor files, rounded in to whole sectors. Every erase and program
/// stays inside it.
const WRITABLE: Range<u32> = 0x0C_6000..0xE0_0000;
/// Where `F000400.bin` lands, and what a release writes there.
const POCKETCW_FILE: u32 = 0x10_0000;
const POCKETCW_HEADER: &[u8; 9] = b"PocketCW\x01";
/// `tools/flashtest.py` tags every sector of its test file with this and the sector's address.
const TEST_TAG: &[u8; 8] = b"PCWTEST\0";
const MARK_TAG: &[u8; 8] = b"PCWMARK\0";
const TAG_LEN: usize = 12;
/// The end of the test file from `tools/flashtest.py`.
const TEST_END: u32 = POCKETCW_FILE + 511 * SECTOR;
/// Markers past the test file's end (in its 64 KB block, then 4 KB, 64 KB and 1 MB on) and in
/// the planned log range.
const MARKS: [u32; 6] = [TEST_END, TEST_END + 0x1000, TEST_END + 0x1_0000, TEST_END + 0x10_0000, 0xC0_0000, 0xDF_F000];
/// The erase and program test uses the first blank sectors from here.
const TEST_FROM: u32 = 0xD0_0000;
const MAX_RANGES: usize = 18;
const CYCLES_PER_MS: u32 = clock::SYSCLK_HZ / 1000;
const HOLD_OFF_LOOPS: u32 = 150; // 1.5 s of 10 ms loops
const COLUMNS: usize = 53;
const ROW_HEIGHT: i32 = 10;
const RANGES_ROW: i32 = 8;
const TAGS_ROW: i32 = 14;
const TEST_ROW: i32 = 16;
const MARKS_ROW: i32 = 18;
const OTHER_ROW: i32 = 20;
const STATUS_ROW: i32 = 21;

type Line = Text<64>;

/// What a scan of the whole chip found.
struct Scan {
    /// Runs of used sectors, start and end address.
    ranges: [(u32, u32); MAX_RANGES],
    shown: usize,
    /// Runs that did not fit in `ranges`.
    more: usize,
    used: u32,
    marks: u32,
    tests: u32,
    test_span: Option<(u32, u32)>,
    /// Used sectors in the writable area that are not the PocketCW file and carry no tag.
    others: u32,
    first_other: Option<(u32, [u8; TAG_LEN])>,
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
        w.spi1().set_bit()
    });
    power::hold();
    clock::init(&dp.CRM);
    // JTAG off and SWD on frees PA15, PB3 and PB4. SPI1 moves to PB3/PB4/PB5.
    dp.IOMUX.remap().modify(|_, w| unsafe { w.swjtag_mux().bits(0b010).spi1_mux0().set_bit() });
    let (gpioa, gpiob) = (gpio::port_a(), gpio::port_b());
    gpio::set_high(gpiob, BACKLIGHT);
    gpio::configure(gpiob, BACKLIGHT, gpio::OUTPUT);
    gpio::set_low(gpiob, SPEAKER_AMP);
    gpio::configure(gpiob, SPEAKER_AMP, gpio::OUTPUT);
    for pin in [POWER_BUTTON, F_KEYS[0], F_KEYS[1], F_KEYS[2], F_KEYS[3]] {
        gpio::set_low(gpioa, pin);
        gpio::configure(gpioa, pin, gpio::INPUT_PULL);
    }

    let mut bus = I2c::new();
    let mut record = [0u8; panel::RECORD_LEN];
    let saved = if eeprom::read(&mut bus, store::PANEL_ADDRESS, &mut record) { panel::decode(&record) } else { None };
    let mut lcd = Lcd::new(dp.SPI1);
    lcd.reset();
    let detected = panel::detect(&lcd.read_id());
    lcd.init(panel::choose(saved, detected, unsafe { read_volatile(0x1FFF_F806 as *const u8) }).0);
    lcd.clear(Rgb565::BLACK).unwrap();

    let mut line = Line::new();
    show(&mut lcd, 0, "PocketCW SPI flash probe 1");
    identify(&mut lcd, &mut line);
    compare_pocketcw_file(&mut lcd, &mut line);
    scan_and_show(&mut lcd, &mut line);
    show(&mut lcd, 15, "F1 ERASE TEST  F2 MARK  F3 CHECK  F4 CLEAN (TWICE)");
    show(&mut lcd, STATUS_ROW + 1, "HOLD THE DIAL 1.5 S: POWER OFF");

    let mut previous = gpio::inputs(gpioa);
    let mut clean_armed = false;
    // The click is still down from switching on, so a hold counts only after one release.
    let mut armed = false;
    let mut held = 0;
    loop {
        delay_ms(10);
        let now = gpio::inputs(gpioa);
        let pressed = |i: usize| now & !previous & (1 << F_KEYS[i]) != 0;
        if pressed(0) {
            erase_test(&mut lcd, &mut line);
        }
        if pressed(1) {
            write_marks(&mut lcd, &mut line);
        }
        if pressed(2) {
            check_marks(&mut lcd, &mut line);
            scan_and_show(&mut lcd, &mut line);
        }
        if pressed(3) {
            if clean_armed {
                clean(&mut lcd, &mut line);
                scan_and_show(&mut lcd, &mut line);
            } else {
                show(&mut lcd, STATUS_ROW, "F4 AGAIN ERASES EVERY TAGGED SECTOR");
            }
            clean_armed = !clean_armed;
        } else if (0..3).any(pressed) {
            clean_armed = false;
        }
        if now & (1 << POWER_BUTTON) != 0 {
            held += 1;
            if armed && held >= HOLD_OFF_LOOPS {
                show(&mut lcd, STATUS_ROW, "RELEASE TO POWER OFF");
                power::standby();
            }
        } else {
            armed = true;
            held = 0;
        }
        previous = now;
    }
}

/// Rows 1 to 5: JEDEC ID, status registers, write protection, and the SFDP basic table.
fn identify(lcd: &mut Lcd, line: &mut Line) {
    let mut flash = lcd.spi_flash();
    let id = flash.jedec_id();
    let status = [flash.status(0), flash.status(1), flash.status(2)];
    let mut header = [0u8; sfdp::HEADER_LEN];
    flash.read_sfdp(0, &mut header);
    let header = sfdp::header(&header);
    let mut table = [0u32; 23];
    let mut dwords = 0;
    if let Some(h) = header {
        dwords = (h.table_dwords as usize).min(table.len());
        let mut bytes = [0u8; 23 * 4];
        flash.read_sfdp(h.table_address, &mut bytes[..dwords * 4]);
        for (dword, chunk) in table.iter_mut().zip(bytes.chunks_exact(4)) {
            *dword = u32::from_le_bytes(chunk.try_into().unwrap());
        }
    }

    line.clear();
    let _ = write!(line, "JEDEC {:02X} {:02X} {:02X}  STATUS {:02X} {:02X} {:02X}", id[0], id[1], id[2], status[0], status[1], status[2]);
    show(lcd, 1, line.as_str());
    // Winbond layout: BP0..2 in status 1 bits 2..4, CMP in status 2 bit 6, WPS in status 3 bit 2.
    let block_protect = status[0] >> 2 & 0b111;
    let complement = status[1] >> 6 & 1;
    let per_block = status[2] >> 2 & 1;
    line.clear();
    let _ = write!(line, "PROTECT {}  BP {block_protect} CMP {complement} WPS {per_block}", if block_protect | complement | per_block == 0 { "NONE" } else { "SET" });
    match header {
        Some(h) => {
            let _ = write!(line, "  SFDP {}.{} {} DW", h.major, h.minor, h.table_dwords);
        }
        None => {
            let _ = write!(line, "  NO SFDP");
        }
    }
    show(lcd, 2, line.as_str());
    let basic = sfdp::basic(&table[..dwords]);
    line.clear();
    let _ = write!(line, "SIZE {}K  ", basic.bytes >> 10);
    let _ = match basic.erase_4k {
        Some(op) => write!(line, "4K ERASE {op:02X}  "),
        None => write!(line, "NO 4K ERASE  "),
    };
    let _ = match basic.page_bytes {
        Some(page) => write!(line, "PAGE {page}"),
        None => write!(line, "PAGE ?"),
    };
    show(lcd, 3, line.as_str());
    for (row, types) in (4..).zip(basic.erase_types.chunks(2)) {
        line.clear();
        for erase in types.iter().flatten() {
            let _ = write!(line, "{}K/{:02X}", erase.bytes >> 10, erase.opcode);
            if let Some((typical, max)) = erase.ms {
                let _ = write!(line, " {typical}-{max}MS");
            }
            let _ = write!(line, "   ");
        }
        show(lcd, row, line.as_str());
    }
}

/// Row 6: the PocketCW file at 0x100000, read at both clocks.
fn compare_pocketcw_file(lcd: &mut Lcd, line: &mut Line) {
    let mut buf = [0u8; SECTOR as usize];
    line.clear();
    let _ = write!(line, "F000400");
    for clock in [Clock::Mhz18, Clock::Mhz4_5] {
        let mut flash = lcd.spi_flash();
        flash.clock = clock;
        flash.read(POCKETCW_FILE, &mut buf);
        let expected = |i: usize| POCKETCW_HEADER.get(i).copied().unwrap_or(0xFF);
        let _ = write!(line, "  {}: ", if clock == Clock::Mhz18 { "18MHZ" } else { "4.5MHZ" });
        let _ = match buf.iter().enumerate().position(|(i, &b)| b != expected(i)) {
            None => write!(line, "MATCH"),
            Some(_) if buf.starts_with(TEST_TAG) => write!(line, "TEST FILE"),
            Some(at) => write!(line, "DIFF AT {at}"),
        };
    }
    show(lcd, 6, line.as_str());
}

/// Rows 7 to 14: reads every sector and shows the runs of used ones and the tags found.
fn scan_and_show(lcd: &mut Lcd, line: &mut Line) {
    let mut scan = Scan { ranges: [(0, 0); MAX_RANGES], shown: 0, more: 0, used: 0, marks: 0, tests: 0, test_span: None, others: 0, first_other: None };
    let mut buf = [0u8; SECTOR as usize];
    let mut run: Option<u32> = None;
    for address in (0..FLASH_BYTES).step_by(SECTOR as usize) {
        if address.is_multiple_of(0x10_0000) {
            line.clear();
            let _ = write!(line, "SCANNING {}/16 MB", address >> 20);
            show(lcd, 7, line.as_str());
        }
        lcd.spi_flash().read(address, &mut buf);
        let used = buf.iter().any(|&b| b != 0xFF);
        if used {
            scan.used += 1;
            if WRITABLE.contains(&address)
                && address != POCKETCW_FILE
                && !tagged(&buf, MARK_TAG, address)
                && !tagged(&buf, TEST_TAG, address)
            {
                scan.others += 1;
                scan.first_other.get_or_insert((address, buf[..TAG_LEN].try_into().unwrap()));
            }
            run.get_or_insert(address);
            if tagged(&buf, MARK_TAG, address) {
                scan.marks += 1;
            }
            if tagged(&buf, TEST_TAG, address) {
                scan.tests += 1;
                let span = scan.test_span.get_or_insert((address, address));
                span.1 = address;
            }
        }
        if let Some(start) = run
            && (!used || address + SECTOR == FLASH_BYTES)
        {
            let end = if used { address + SECTOR } else { address } - 1;
            if scan.shown < MAX_RANGES {
                scan.ranges[scan.shown] = (start, end);
                scan.shown += 1;
            } else {
                scan.more += 1;
            }
            run = None;
        }
    }
    line.clear();
    let _ = write!(line, "USED {} SECTORS IN {} RUNS", scan.used, scan.shown + scan.more);
    show(lcd, 7, line.as_str());
    for (row, ranges) in (RANGES_ROW..TAGS_ROW).zip(scan.ranges[..scan.shown].chunks(3).chain(core::iter::repeat(&[][..]))) {
        line.clear();
        for (start, end) in ranges {
            let _ = write!(line, "{start:06X}-{end:06X}  ");
        }
        if row == TAGS_ROW - 1 && scan.more > 0 {
            let _ = write!(line, "+{} MORE", scan.more);
        }
        show(lcd, row, line.as_str());
    }
    line.clear();
    let _ = write!(line, "TAGS: {} MARKS  {} TEST SECTORS", scan.marks, scan.tests);
    if let Some((first, last)) = scan.test_span {
        let _ = write!(line, " {first:06X}-{:06X}", last + SECTOR - 1);
    }
    show(lcd, TAGS_ROW, line.as_str());
    line.clear();
    if let Some((address, head)) = scan.first_other {
        let _ = write!(line, "OTHER {} {address:06X}:", scan.others);
        for byte in head {
            let _ = write!(line, " {byte:02X}");
        }
    }
    show(lcd, OTHER_ROW, line.as_str());
}

fn tagged(sector: &[u8], tag: &[u8; 8], address: u32) -> bool {
    sector.starts_with(tag) && sector[8..TAG_LEN] == address.to_le_bytes()
}

fn tag(tag: &[u8; 8], address: u32) -> [u8; TAG_LEN] {
    let mut bytes = [0u8; TAG_LEN];
    bytes[..8].copy_from_slice(tag);
    bytes[8..].copy_from_slice(&address.to_le_bytes());
    bytes
}

fn is_blank(flash: &mut SpiFlash, address: u32) -> bool {
    let mut buf = [0u8; SECTOR as usize];
    flash.read(address, &mut buf);
    buf.iter().all(|&b| b == 0xFF)
}

/// Cycles from `start` to the chip going idle.
fn wait_cycles(flash: &mut SpiFlash, start: u32) -> u32 {
    flash.wait();
    DWT::cycle_count().wrapping_sub(start)
}

fn erase(flash: &mut SpiFlash, address: u32) -> u32 {
    assert!(WRITABLE.contains(&address) && address.is_multiple_of(SECTOR));
    let start = DWT::cycle_count();
    flash.start_erase(address);
    wait_cycles(flash, start)
}

fn program(flash: &mut SpiFlash, address: u32, data: &[u8]) -> u32 {
    assert!(WRITABLE.contains(&address) && WRITABLE.contains(&(address + data.len() as u32 - 1)));
    let start = DWT::cycle_count();
    flash.start_program(address, data);
    wait_cycles(flash, start)
}

fn pattern(address: u32) -> u8 {
    (address ^ address >> 8 ^ address >> 16) as u8
}

/// Rows 16 and 17: erases two blank sectors, programs one in whole pages and the other's first
/// page in 16 pieces, reads both back, and erases them again.
fn erase_test(lcd: &mut Lcd, line: &mut Line) {
    show(lcd, STATUS_ROW, "ERASE TEST...");
    let mut flash = lcd.spi_flash();
    let mut sectors = (TEST_FROM..WRITABLE.end).step_by(SECTOR as usize).filter(|a| !MARKS.contains(a));
    let (Some(whole), Some(pieces)) = (sectors.find(|&a| is_blank(&mut flash, a)), sectors.find(|&a| is_blank(&mut flash, a))) else {
        show(lcd, TEST_ROW, "NO TWO BLANK SECTORS FROM D00000");
        return;
    };
    let mut erase_cycles = [erase(&mut flash, whole), erase(&mut flash, pieces), 0, 0];
    let mut page = [0u8; PAGE as usize];
    let mut program_max = 0;
    for page_address in (whole..whole + SECTOR).step_by(PAGE as usize) {
        for (i, byte) in page.iter_mut().enumerate() {
            *byte = pattern(page_address + i as u32);
        }
        program_max = program_max.max(program(&mut flash, page_address, &page));
    }
    const PIECE: usize = PAGE as usize / 16;
    for i in 0..16 {
        let piece: [u8; PIECE] = core::array::from_fn(|j| pattern(pieces + (i * PIECE + j) as u32));
        program(&mut flash, pieces + (i * PIECE) as u32, &piece);
    }
    let mut buf = [0u8; SECTOR as usize];
    flash.read(whole, &mut buf);
    let whole_ok = buf.iter().enumerate().all(|(i, &b)| b == pattern(whole + i as u32));
    flash.read(pieces, &mut buf[..PAGE as usize]);
    let pieces_ok = (0..16).filter(|&i| (0..PIECE).all(|j| buf[i * PIECE + j] == pattern(pieces + (i * PIECE + j) as u32))).count();
    erase_cycles[2] = erase(&mut flash, whole);
    erase_cycles[3] = erase(&mut flash, pieces);
    let blank_after = is_blank(&mut flash, whole) && is_blank(&mut flash, pieces);

    let ms = |cycles: u32| cycles / CYCLES_PER_MS;
    line.clear();
    let _ = write!(line, "AT {whole:06X} {pieces:06X}  ERASE MS {} {} {} {}", ms(erase_cycles[0]), ms(erase_cycles[1]), ms(erase_cycles[2]), ms(erase_cycles[3]));
    show(lcd, TEST_ROW, line.as_str());
    line.clear();
    let _ = write!(
        line,
        "PAGE MAX {}US  DATA {}  PIECES {pieces_ok}/16  BLANK {}",
        program_max / (CYCLES_PER_MS / 1000),
        if whole_ok { "OK" } else { "BAD" },
        if blank_after { "OK" } else { "BAD" }
    );
    show(lcd, TEST_ROW + 1, line.as_str());
    show(lcd, STATUS_ROW, "ERASE TEST DONE");
}

/// Rows 18 and 19: writes a tag at each mark that reads blank.
fn write_marks(lcd: &mut Lcd, line: &mut Line) {
    let mut flash = lcd.spi_flash();
    let mut results: [&str; MARKS.len()] = [""; MARKS.len()];
    for (result, &address) in results.iter_mut().zip(&MARKS) {
        let mut head = [0u8; TAG_LEN];
        flash.read(address, &mut head);
        *result = if tagged(&head, MARK_TAG, address) {
            "HAD IT"
        } else if is_blank(&mut flash, address) {
            program(&mut flash, address, &tag(MARK_TAG, address));
            "WROTE"
        } else {
            "USED: SKIP"
        };
    }
    show_marks(lcd, line, &results);
    show(lcd, STATUS_ROW, "MARKS WRITTEN: INSTALL THE TEST F FILE, THEN F3");
}

/// Rows 18 and 19: whether each mark is still there.
fn check_marks(lcd: &mut Lcd, line: &mut Line) {
    let mut flash = lcd.spi_flash();
    let mut results: [&str; MARKS.len()] = [""; MARKS.len()];
    for (result, &address) in results.iter_mut().zip(&MARKS) {
        let mut head = [0u8; TAG_LEN];
        flash.read(address, &mut head);
        *result = if tagged(&head, MARK_TAG, address) {
            "KEPT"
        } else if is_blank(&mut flash, address) {
            "ERASED"
        } else {
            "CHANGED"
        };
    }
    show_marks(lcd, line, &results);
    show(lcd, STATUS_ROW, "MARKS CHECKED");
}

fn show_marks(lcd: &mut Lcd, line: &mut Line, results: &[&str; MARKS.len()]) {
    for (row, first) in (MARKS_ROW..).zip((0..MARKS.len()).step_by(3)) {
        line.clear();
        for i in first..(first + 3).min(MARKS.len()) {
            let _ = write!(line, "{:06X} {:<10}", MARKS[i], results[i]);
        }
        show(lcd, row, line.as_str());
    }
}

/// Erases every sector in the writable area that starts with a probe or test file tag.
fn clean(lcd: &mut Lcd, line: &mut Line) {
    let mut erased = 0;
    for address in WRITABLE.step_by(SECTOR as usize) {
        let mut head = [0u8; TAG_LEN];
        let mut flash = lcd.spi_flash();
        flash.read(address, &mut head);
        if tagged(&head, MARK_TAG, address) || tagged(&head, TEST_TAG, address) {
            erase(&mut flash, address);
            erased += 1;
            if erased % 16 == 0 {
                line.clear();
                let _ = write!(line, "CLEANING: {erased} ERASED");
                show(lcd, STATUS_ROW, line.as_str());
            }
        }
    }
    line.clear();
    let _ = write!(line, "CLEANED: {erased} TAGGED SECTORS ERASED");
    show(lcd, STATUS_ROW, line.as_str());
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
    Label::with_baseline(padded.as_str(), Point::new(1, row * ROW_HEIGHT), style, Baseline::Top)
        .draw(lcd)
        .unwrap();
}
