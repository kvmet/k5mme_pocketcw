//! Memory keyer: the MEM row on Home sends a stored message on the radio, and the MEM settings
//! tab edits the messages on the paddle. Touching the key stops a message.

use core::fmt::Write;

use pocketcw_logic::{
    decoder::Decoded,
    input::Action,
    memories::{LEN, SLOTS},
    keyer::Output,
    player::{Keying, MAX_TEXT},
    text::wrap,
    settings::Settings,
};

use super::{COLS, Play, Ui};
use crate::gfx::{TINY_H, TINY_W, rect, text, text_cols, text_right, tiny};
use crate::lcd::Lcd;
use crate::text::Text;
use crate::theme::MAIN;

const HOME_Y: i32 = 148;
const HOME_STEP: i32 = 18;
const HOME_TEXT_X: i32 = 24;

const TAB_ROW_Y: i32 = 48;
const MEMORY_STEP: i32 = 28;
const TINY_COLS: usize = ((MAIN - 30) as usize) / TINY_W as usize;
/// The EVERY and AFTER SEND rows, after the memories.
const OPTION_Y: i32 = TAB_ROW_Y + SLOTS as i32 * MEMORY_STEP + 4;
const OPTION_STEP: i32 = 19;
const EVERY_ROW: usize = SLOTS;
const AFTER_ROW: usize = SLOTS + 1;
const ROWS: usize = SLOTS + 2;
const HINT_Y: i32 = OPTION_Y + 2 * OPTION_STEP + 4;

/// A memory being sent from Home.
#[derive(Clone, Copy)]
pub(super) struct Sending {
    slot: usize,
    /// Tick time of the next repeat, while waiting between repeats.
    pub(super) again_at: Option<u32>,
}

impl Ui {
    pub(super) fn open_mem_row(&mut self, lcd: &mut Lcd) {
        self.mem_row = true;
        self.theme.softkeys(lcd, ["M1", "M2", "M3", "M4"]);
        let t = &self.theme;
        for i in 0..SLOTS {
            let y = HOME_Y + i as i32 * HOME_STEP;
            rect(lcd, 0, y, MAIN, HOME_STEP, t.bg);
            let label = Text::<3>::format(format_args!("M{}", i + 1));
            if self.memories.repeats[i] {
                rect(lcd, 0, y + 1, HOME_TEXT_X - 4, TINY_H + 2, t.accent);
                tiny(lcd, 2, y + 3, label.as_str(), t.bg, t.accent);
            } else {
                tiny(lcd, 2, y + 3, label.as_str(), t.muted, t.bg);
            }
            let shown = self.memories.text(i);
            let (shown, color) = if shown.is_empty() { ("EMPTY", t.muted) } else { (shown, t.fg) };
            text_cols(lcd, HOME_TEXT_X, y, shown, COLS - 3, color, t.bg);
        }
        self.idle_status(lcd);
    }

    /// The status line when no memory is sending: what the MEM row does, if it is open.
    fn idle_status(&self, lcd: &mut Lcd) {
        let line = match (self.mem_row, self.memories.repeats.contains(&true)) {
            (false, _) => "",
            (true, true) => "LIT M REPEATS  CLICK CLOSES",
            (true, false) => "CLICK CLOSES",
        };
        self.status(lcd, line);
    }

    pub(super) fn close_mem_row(&mut self, lcd: &mut Lcd) {
        self.mem_row = false;
        self.theme.softkeys(lcd, super::home::KEYS);
        self.draw_decode_all(lcd);
    }

    /// Sends memory `slot` on the radio at your keyer speed.
    pub(super) fn send_memory(&mut self, lcd: &mut Lcd, s: &Settings, slot: usize) {
        let mut line = Text::<COLS>::new();
        let memory = self.memories.text(slot);
        if memory.is_empty() {
            self.sending = None;
            let _ = write!(line, "M{} IS EMPTY (SETTINGS, MEM)", slot + 1);
            return self.status(lcd, line.as_str());
        }
        let message = Text::<MAX_TEXT>::format(format_args!("{memory}"));
        let seed = self.next_seed();
        let keying = Keying { wpm: s.wpm, effective: s.wpm, fist: 0 };
        self.play = Some(Play { text: message, keying, seed, delay_ms: 0, pitch_hz: s.tone_hz, transmit: true });
        self.sending = Some(Sending { slot, again_at: None });
        let _ = if s.output == Output::Paddle {
            write!(line, "M{} TONE ONLY: OUT IS PADDLE", slot + 1)
        } else {
            write!(line, "SENDING M{}  KEY TO STOP", slot + 1)
        };
        self.status(lcd, line.as_str());
    }

    /// Stops a memory being sent or waiting to repeat.
    pub(super) fn stop_sending(&mut self, lcd: &mut Lcd) {
        if self.sending.take().is_some() {
            self.stop_players();
            self.idle_status(lcd);
        }
    }

    /// A memory ended: the repeating one waits and goes again.
    pub(super) fn memory_played(&mut self, lcd: &mut Lcd, now_ms: u32) {
        let Some(sending) = &mut self.sending else { return };
        if sending.again_at.is_some() {
            return;
        }
        if !self.memories.repeats[sending.slot] {
            self.sending = None;
            return self.idle_status(lcd);
        }
        sending.again_at = Some(now_ms.wrapping_add(self.memories.every_s as u32 * 1000));
        let line = Text::<COLS>::format(format_args!("M{} AGAIN IN {}S  KEY TO STOP", sending.slot + 1, self.memories.every_s));
        self.status(lcd, line.as_str());
    }

    pub(super) fn memory_timeout(&mut self, lcd: &mut Lcd, s: &Settings) {
        if let Some(sending) = self.sending {
            self.send_memory(lcd, s, sending.slot);
        }
    }

    pub(super) fn draw_mem_tab(&self, lcd: &mut Lcd) {
        for row in 0..ROWS {
            self.draw_mem_tab_row(lcd, row);
        }
    }

    /// The line under the options: how to use the selected row.
    fn draw_mem_hint(&self, lcd: &mut Lcd) {
        let t = &self.theme;
        let mut hint = Text::<40>::new();
        let _ = match self.row {
            EVERY_ROW => write!(hint, "CLICK, THEN TURN TO SET"),
            AFTER_ROW => write!(hint, "CLICK TO CHANGE"),
            row => write!(hint, "KEY TO TYPE  CLICK: REPEAT  {} LEFT", LEN - self.memories.text(row).len()),
        };
        rect(lcd, 0, HINT_Y, MAIN, TINY_H, t.bg);
        tiny(lcd, 10, HINT_Y, hint.as_str(), t.muted, t.bg);
    }

    pub(super) fn mem_tab_keys(&self) -> [&'static str; 4] {
        if self.row < SLOTS { ["TAB", "DEL", "CLR", "BACK"] } else { ["TAB", "-", "+", "BACK"] }
    }

    fn draw_mem_tab_row(&self, lcd: &mut Lcd, row: usize) {
        let t = &self.theme;
        let selected = row == self.row;
        if row < SLOTS {
            let y = TAB_ROW_Y + row as i32 * MEMORY_STEP;
            rect(lcd, 0, y - 1, MAIN, MEMORY_STEP, t.bg);
            let (fg, muted, bg) =
                if selected { t.select(lcd, 2, y - 1, MAIN - 6, MEMORY_STEP - 2) } else { (t.fg, t.muted, t.bg) };
            let label = Text::<3>::format(format_args!("M{}", row + 1));
            tiny(lcd, 8, y + 1, label.as_str(), muted, bg);
            let memory = self.memories.text(row);
            let mut lines = wrap(memory, TINY_COLS);
            let (first, second) = (lines.next().unwrap_or(""), lines.next().unwrap_or(""));
            // Two rows fit; text that wraps to three breaks mid-word instead.
            let (first, second) = if lines.next().is_some() { memory.split_at(TINY_COLS) } else { (first, second) };
            tiny(lcd, 28, y + 1, first, fg, bg);
            tiny(lcd, 28, y + 14, second, fg, bg);
            if self.memories.repeats[row] {
                tiny(lcd, 6, y + 14, "RPT", muted, bg);
            }
            if selected {
                self.draw_mem_hint(lcd);
            }
            return;
        }
        let y = OPTION_Y + (row - SLOTS) as i32 * OPTION_STEP;
        rect(lcd, 0, y - 1, MAIN, OPTION_STEP, t.bg);
        let (fg, value_fg, bg) = if selected {
            self.draw_mem_hint(lcd);
            let (fg, _, bg) = t.select(lcd, 2, y - 1, MAIN - 6, OPTION_STEP);
            (fg, fg, bg)
        } else {
            (t.fg, t.accent, t.bg)
        };
        let mut value = Text::<12>::new();
        let label = if row == EVERY_ROW {
            let _ = write!(value, "{} S", self.memories.every_s);
            "REPEAT EVERY"
        } else {
            let _ = write!(value, "{}", if self.memories.stay_open { "STAY OPEN" } else { "CLOSE" });
            "AFTER SEND"
        };
        text(lcd, 10, y + 1, label, fg, bg);
        let shown = if self.editing && selected {
            Text::<16>::format(format_args!("< {} >", value.as_str()))
        } else {
            Text::format(format_args!("{}", value.as_str()))
        };
        text_right(lcd, MAIN - 12, y + 1, shown.as_str(), value_fg, bg);
    }

    fn step_every(&mut self, lcd: &mut Lcd, delta: i8) {
        self.memories.step_every(delta);
        self.draw_mem_tab_row(lcd, EVERY_ROW);
    }

    /// The MEM tab: the dial picks a row, DEL and CLR edit a memory, a click on a memory turns
    /// its repeat on or off, a click on EVERY edits it with the dial, and a click on AFTER SEND
    /// changes it.
    pub(super) fn mem_tab_act(&mut self, lcd: &mut Lcd, action: Action) {
        let row = self.row;
        match action {
            Action::Turn(delta) if self.editing => self.step_every(lcd, delta),
            Action::Turn(delta) => {
                let next = (row as i32 + delta as i32).clamp(0, ROWS as i32 - 1) as usize;
                if next != row {
                    self.row = next;
                    self.draw_mem_tab_row(lcd, row);
                    self.draw_mem_tab_row(lcd, next);
                    if (row < SLOTS) != (next < SLOTS) {
                        self.theme.softkeys(lcd, self.mem_tab_keys());
                    }
                }
            }
            Action::Click if row == EVERY_ROW => {
                self.editing = !self.editing;
                self.draw_mem_tab_row(lcd, row);
            }
            Action::Click | Action::Soft(1 | 2) if row == AFTER_ROW => {
                self.memories.stay_open = !self.memories.stay_open;
                self.draw_mem_tab_row(lcd, row);
            }
            Action::Click => {
                self.memories.repeats[row] = !self.memories.repeats[row];
                self.draw_mem_tab_row(lcd, row);
            }
            Action::Soft(1) if row < SLOTS => {
                self.memories.pop(row);
                self.draw_mem_tab_row(lcd, row);
            }
            Action::Soft(2) if row < SLOTS => {
                self.memories.clear(row);
                self.draw_mem_tab_row(lcd, row);
            }
            Action::Soft(key @ (1 | 2)) if row == EVERY_ROW => self.step_every(lcd, if key == 1 { -1 } else { 1 }),
            _ => {}
        }
    }

    pub(super) fn mem_tab_decoded(&mut self, lcd: &mut Lcd, decoded: Decoded) {
        if self.row >= SLOTS {
            return;
        }
        let byte = match decoded {
            Decoded::Char(c) => c.to_ascii_uppercase(),
            Decoded::Space => b' ',
            Decoded::Unknown => return,
        };
        if self.memories.push(self.row, byte) {
            self.draw_mem_tab_row(lcd, self.row);
        } else {
            self.status(lcd, "MEMORY FULL");
        }
    }
}
