//! Radio: what the Pro decoder board hears, decoded, with the board's filter and noise
//! reduction. F2 and F3 step them, as on the vendor decoder screen; a click clears the text.

use core::fmt::Write;

use pocketcw_logic::{
    board::{self, Event, MAX_FILTER, MAX_NOISE_REDUCTION},
    decode_lines::DecodeLines,
    decoder::Decoded,
    input::Action,
    settings::Settings,
    timing::Quality,
};

use super::{COLS, Screen, Ui, settings::RECEIVE_TAB};
use crate::gfx::{CHAR_W, TINY_H, TINY_W, rect, text, tiny};
use crate::lcd::Lcd;
use crate::link;
use crate::text::Text;
use crate::theme::MAIN;

const INFO_Y: i32 = 26;
const INFO_COLS: usize = ((MAIN - 2) / TINY_W) as usize;
const RULE_Y: i32 = 40;
const LINES_Y: i32 = 44;
const LINE_STEP: i32 = 18;
const ROWS: usize = 9;

pub(super) type Heard = DecodeLines<COLS, ROWS>;

/// What the decoder board last reported, and the speed it is heard at.
#[derive(Default)]
pub(super) struct Board {
    /// The board announced itself since power on.
    present: bool,
    filter: u8,
    noise_reduction: u8,
    wpm: u16,
}

impl Ui {
    pub(super) fn draw_radio(&self, lcd: &mut Lcd) {
        let t = &self.theme;
        t.header(lcd, "RADIO", &[]);
        t.softkeys(lcd, ["SET", "FILT", "NR", "BACK"]);
        self.draw_radio_info(lcd);
        rect(lcd, 0, RULE_Y, MAIN, 1, t.rule());
        for row in 0..ROWS {
            self.draw_heard_line(lcd, row);
        }
    }

    fn draw_radio_info(&self, lcd: &mut Lcd) {
        let t = &self.theme;
        let b = &self.board;
        let mut line = Text::<48>::new();
        let status = if b.present { "BOARD OK" } else { "NO REPLY" };
        let _ = write!(line, "{status} FIL {} NR {} ", b.filter, b.noise_reduction);
        let _ = match b.wpm {
            0 => write!(line, "--WPM"),
            wpm => write!(line, "{wpm}WPM"),
        };
        let errors = link::ERRORS.load(core::sync::atomic::Ordering::Relaxed);
        if errors > 0 {
            let _ = write!(line, " ERR {errors}");
        }
        let shown = &line.as_str()[..line.len().min(INFO_COLS)];
        tiny(lcd, 2, INFO_Y, shown, t.muted, t.bg);
        rect(lcd, 2 + shown.len() as i32 * TINY_W, INFO_Y, (INFO_COLS - shown.len()) as i32 * TINY_W, TINY_H, t.bg);
    }

    fn draw_heard_line(&self, lcd: &mut Lcd, row: usize) {
        let t = &self.theme;
        text(lcd, 0, LINES_Y + row as i32 * LINE_STEP, self.heard.line(row), t.fg, t.bg);
    }

    pub(super) fn radio_act(&mut self, lcd: &mut Lcd, s: &mut Settings, action: Action) {
        match action {
            Action::Soft(0) => self.open_settings_at(lcd, s, RECEIVE_TAB),
            Action::Soft(1) => {
                self.board.filter = (self.board.filter + 1) % (MAX_FILTER + 1);
                self.board_command = Some(board::filter(self.board.filter));
                self.draw_radio_info(lcd);
            }
            Action::Soft(2) => {
                self.board.noise_reduction = (self.board.noise_reduction + 1) % (MAX_NOISE_REDUCTION + 1);
                self.board_command = Some(board::noise_reduction(self.board.noise_reduction));
                self.draw_radio_info(lcd);
            }
            Action::Soft(3) => self.open(lcd, s, Screen::Menu),
            Action::Click => {
                self.heard = Heard::new();
                self.draw(lcd, s);
            }
            _ => {}
        }
    }

    /// A frame from the decoder board.
    pub fn board_event(&mut self, lcd: &mut Lcd, event: Event) {
        let b = &mut self.board;
        match event {
            Event::Hello { present, .. } => b.present = present,
            Event::Filter(level) => b.filter = level,
            Event::NoiseReduction(level) => b.noise_reduction = level,
            Event::Mode(_) | Event::Ignored | Event::Foreign | Event::Unknown => return,
        }
        if self.screen == Screen::Radio {
            self.draw_radio_info(lcd);
        }
    }

    /// A character decoded from the board's tone line, heard at `wpm`.
    pub fn heard(&mut self, lcd: &mut Lcd, decoded: Decoded, wpm: u16) {
        // The timing quality marks your own keying; heard text has none to show.
        self.heard.push(decoded, Quality::Good);
        let speed_changed = core::mem::replace(&mut self.board.wpm, wpm) != wpm;
        if self.screen != Screen::Radio {
            return;
        }
        if speed_changed {
            self.draw_radio_info(lcd);
        }
        if self.heard.scrolled() {
            return (0..ROWS).for_each(|row| self.draw_heard_line(lcd, row));
        }
        let (row, col) = self.heard.cursor();
        if let Some(col) = col.checked_sub(1) {
            let t = &self.theme;
            text(lcd, col as i32 * CHAR_W, LINES_Y + row as i32 * LINE_STEP, &self.heard.line(row)[col..=col], t.fg, t.bg);
        }
    }

    /// The decoder board setting changed. Off, the board is unpowered and must announce itself
    /// again.
    pub fn board_powered(&mut self, on: bool) {
        if !on {
            self.board.present = false;
        }
    }

    /// The board announced itself, so it takes volume changes.
    pub fn board_present(&self) -> bool {
        self.board.present
    }

    /// A command for the decoder board, once.
    pub fn take_board_command(&mut self) -> Option<board::Command> {
        self.board_command.take()
    }

    /// The RADIO screen is open, so the tone line should be decoded.
    pub fn listening(&self) -> bool {
        self.screen == Screen::Radio
    }
}
