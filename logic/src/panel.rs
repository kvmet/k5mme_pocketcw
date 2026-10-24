//! The two LCD panel types, which one a board gets, and the saved choice that overrides it.
//! The board gets the type its controller names when asked for its ID, else the type its
//! revision byte implies.

use crate::record::{self, body_mut, frame, unframe};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Panel {
    /// ST7789-style IPS, from board 2.63.
    Ips,
    Ili9341,
}

impl Panel {
    /// From option byte Data1, the board revision after "2.". An erased byte (0xFF) gets the
    /// ILI9341, as in the vendor firmware. The vendor activation writes 0x60 on any board, so an
    /// IPS board can read below 0x63: the ID read and the saved choice cover it.
    pub fn for_board(board: u8) -> Panel {
        if (0x63..0xFF).contains(&board) { Panel::Ips } else { Panel::Ili9341 }
    }

    pub fn other(self) -> Panel {
        match self {
            Panel::Ips => Panel::Ili9341,
            Panel::Ili9341 => Panel::Ips,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Panel::Ips => "IPS",
            Panel::Ili9341 => "ILI9341",
        }
    }
}

/// The panel's answer to RDDID (0x04) on one data pin: 32 bits, first bit in bit 31, read once
/// with the pin pulled up and once pulled down.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct IdRead {
    pub pulled_up: u32,
    pub pulled_down: u32,
}

impl IdRead {
    /// The answer when the panel drove the pin: both pulls read the same bits.
    pub fn driven(self) -> Option<u32> {
        (self.pulled_up == self.pulled_down).then_some(self.pulled_up)
    }
}

/// The ST7789V's RDDID answer as shipped. An ILI9341 answers 00 80 00 or a module maker's ID, and
/// its ID2 is always 0x80 or more, so it cannot match.
const ST7789V_ID: u32 = 0x85_85_52;
/// Both datasheets put one dummy clock before the 24 ID bits, so the ID ends 7 bits above the
/// bottom of the 32 read.
const ID_SHIFT: u32 = 7;

/// The panel type a driven answer names exactly, on any pin.
pub fn detect(reads: &[IdRead]) -> Option<Panel> {
    reads.iter().filter_map(|read| read.driven()).find(|answer| answer >> ID_SHIFT & 0xFF_FFFF == ST7789V_ID).map(|_| Panel::Ips)
}

/// What picked the panel type.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Source {
    /// A choice saved with F2 at power-on.
    Saved,
    /// The controller's ID.
    Id,
    /// The board revision.
    Board,
}

/// The saved choice wins, then the controller's ID, then the board revision.
pub fn choose(saved: Option<Panel>, detected: Option<Panel>, board: u8) -> (Panel, Source) {
    match (saved, detected) {
        (Some(panel), _) => (panel, Source::Saved),
        (None, Some(panel)) => (panel, Source::Id),
        (None, None) => (Panel::for_board(board), Source::Board),
    }
}

/// What to save so `panel` shows: None when `auto`, the type chosen without a saved choice,
/// already is `panel`.
pub fn saved_choice(panel: Panel, auto: Panel) -> Option<Panel> {
    (panel != auto).then_some(panel)
}

const MAGIC: [u8; 2] = *b"Rp";
const VERSION: u8 = 1;
/// Body: choice (0 none, 1 IPS, 2 ILI9341).
pub const RECORD_LEN: usize = 1 + record::OVERHEAD;

pub fn encode(saved: Option<Panel>) -> [u8; RECORD_LEN] {
    let choice = match saved {
        None => 0,
        Some(Panel::Ips) => 1,
        Some(Panel::Ili9341) => 2,
    };
    let mut record = [0; RECORD_LEN];
    body_mut(&mut record)[0] = choice;
    frame(&mut record, MAGIC, VERSION);
    record
}

/// The choice in the record at the start of `slot`; None for no choice, and for a blank,
/// foreign or damaged record.
pub fn decode(slot: &[u8]) -> Option<Panel> {
    match unframe(slot, MAGIC, VERSION)?.first()? {
        1 => Some(Panel::Ips),
        2 => Some(Panel::Ili9341),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn board_revision_picks_the_panel() {
        assert_eq!(Panel::for_board(0x60), Panel::Ili9341);
        assert_eq!(Panel::for_board(0x62), Panel::Ili9341);
        assert_eq!(Panel::for_board(0x63), Panel::Ips);
        assert_eq!(Panel::for_board(0x64), Panel::Ips);
        assert_eq!(Panel::for_board(0xFF), Panel::Ili9341, "erased option byte");
    }

    #[test]
    fn saved_choice_wins_then_id_then_board() {
        assert_eq!(choose(None, None, 0x64), (Panel::Ips, Source::Board));
        assert_eq!(choose(None, Some(Panel::Ips), 0x60), (Panel::Ips, Source::Id));
        assert_eq!(choose(Some(Panel::Ili9341), Some(Panel::Ips), 0x64), (Panel::Ili9341, Source::Saved));
    }

    #[test]
    fn flipping_twice_returns_to_no_choice() {
        for (detected, board) in [(None, 0x60), (Some(Panel::Ips), 0x60)] {
            let auto = choose(None, detected, board).0;
            let first = saved_choice(auto.other(), auto);
            assert_eq!(first, Some(auto.other()));
            let second = saved_choice(choose(first, detected, board).0.other(), auto);
            assert_eq!(second, None);
        }
    }

    /// `id` placed after `dummy_bits` leading bits, padded with ones below, as a pulled-up line
    /// reads after the panel stops driving.
    fn answer(id: u32, dummy_bits: u32) -> u32 {
        (id << (8 - dummy_bits)) | ((1 << (8 - dummy_bits)) - 1)
    }

    fn steady(bits: u32) -> IdRead {
        IdRead { pulled_up: bits, pulled_down: bits }
    }

    #[test]
    fn st7789v_id_after_one_dummy_clock_is_ips() {
        assert_eq!(detect(&[steady(answer(0x85_85_52, 1))]), Some(Panel::Ips));
        assert_eq!(detect(&[steady(0xFFFF_FFFF), steady(answer(0x85_85_52, 1))]), Some(Panel::Ips), "either pin");
    }

    #[test]
    fn other_answers_detect_nothing() {
        assert_eq!(detect(&[steady(answer(0x85_85_52, 0))]), None, "no dummy clock");
        assert_eq!(detect(&[steady(answer(0x85_85_52, 2))]), None, "two dummy clocks");
        assert_eq!(detect(&[steady(answer(0x00_80_00, 1))]), None, "ILI9341 default");
        assert_eq!(detect(&[]), None);
    }

    #[test]
    fn a_pin_nobody_drives_is_ignored() {
        let id = answer(0x85_85_52, 1);
        assert_eq!(IdRead { pulled_up: id, pulled_down: id }.driven(), Some(id));
        assert_eq!(IdRead { pulled_up: 0xFFFF_FFFF, pulled_down: 0 }.driven(), None);
        let one_bit_differs = IdRead { pulled_up: id, pulled_down: id & !1 };
        assert_eq!(one_bit_differs.driven(), None, "all 32 bits must agree");
        assert_eq!(detect(&[one_bit_differs]), None);
    }

    #[test]
    fn record_round_trips_and_rejects_damage() {
        for saved in [None, Some(Panel::Ips), Some(Panel::Ili9341)] {
            assert_eq!(decode(&encode(saved)), saved);
        }
        assert_eq!(decode(&[0xFF; RECORD_LEN]), None);
        let mut record = encode(Some(Panel::Ips));
        record[4] = 2;
        assert_eq!(decode(&record), None, "CRC no longer matches");
    }
}
