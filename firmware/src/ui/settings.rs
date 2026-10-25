//! Settings: one screen with tabs. F1 moves between tabs, the dial moves between rows, and a
//! click starts or ends editing the selected row. The CALL tab takes your callsign from the
//! paddle.

use core::fmt::Write;

use embedded_graphics::pixelcolor::Rgb565;
use pocketcw_logic::{
    config,
    decoder::Decoded,
    input::Action,
    keyer::Output,
    settings::{Field, MAX_BRIGHTNESS, PALETTE_COUNT, Pileup, Settings, UsbOut},
};

use super::{Ui, mode_name};
use crate::gfx::{TINY_W, blocks, para, rect, text, text_right, tiny};
use crate::lcd::Lcd;
use crate::text::Text;
use crate::theme::{self, DAILY, MAIN, Theme};

const TABS: [&str; 7] = ["KEYER", "SOUND", "RECV", "QSO", "THEME", "CALL", "MEM"];
const TAB_Y: i32 = 26;
const TAB_H: i32 = 16;
pub(super) const RECEIVE_TAB: usize = 2;
pub(super) const QSO_TAB: usize = 3;
pub(super) const KEYER_TAB: usize = 0;
pub(super) const SOUND_TAB: usize = 1;
pub(super) const THEME_TAB: usize = 4;
const ROW_Y: i32 = 48;
const ROW_STEP: i32 = 19;
const CALL_TAB: usize = 5;
const MEM_TAB: usize = 6;
/// Space inside a tab on each side of its name, and between tabs: seven tabs fill the width.
/// The pad matches the 4 px slant of a Hazard selection, so no letter sits outside it.
const TAB_PAD: i32 = 4;
const TAB_GAP: i32 = 1;

const KEYER_ROWS: [(&str, Field); 7] = [
    ("MODE", Field::Mode),
    ("PADDLES", Field::Swap),
    ("SPEED", Field::Speed),
    ("KEY OUT", Field::KeyOut),
    ("OUT TYPE", Field::Output),
    ("OUT LINES", Field::OutputSwap),
    ("USB OUT", Field::UsbOut),
];
const SOUND_ROWS: [(&str, Field); 5] = [
    ("TONE", Field::Tone),
    ("VOLUME", Field::Volume),
    ("WAVE", Field::Wave),
    ("NOISE", Field::Noise),
    ("FADING", Field::Fading),
];
const RECEIVE_ROWS: [(&str, Field); 4] = [
    ("CHAR SPEED", Field::RxSpeed),
    ("FARNSWORTH", Field::RxEffective),
    ("CHEW SHOWS", Field::Reveal),
    ("DECODER BOARD", Field::DecoderBoard),
];
const QSO_ROWS: [(&str, Field); 5] = [
    ("CHAR SPEED", Field::RxSpeed),
    ("FARNSWORTH", Field::RxEffective),
    ("PILEUP", Field::Pileup),
    ("FIST", Field::Fist),
    ("SPEED SPREAD", Field::Spread),
];
const THEME_ROWS: [(&str, Field); 3] =
    [("STYLE", Field::Style), ("PALETTE", Field::Palette), ("BRIGHTNESS", Field::Brightness)];
/// Space between the last row and the tab's note.
const NOTE_GAP: i32 = 7;

const SWATCH_Y: i32 = 112;
/// Six fit across MAIN at a 44 px step, so all 16 palettes fit in three rows above the note.
const SWATCHES_PER_ROW: u8 = 6;
const PALETTE_NOTE_Y: i32 = 202;

const CALL_PX: i32 = 4;
const CALL_GAP: i32 = 3;
const CALL_Y: i32 = 60;

fn rows(tab: usize) -> &'static [(&'static str, Field)] {
    match tab {
        KEYER_TAB => &KEYER_ROWS,
        SOUND_TAB => &SOUND_ROWS,
        RECEIVE_TAB => &RECEIVE_ROWS,
        QSO_TAB => &QSO_ROWS,
        THEME_TAB => &THEME_ROWS,
        _ => &[],
    }
}

impl Ui {
    pub(super) fn draw_settings(&mut self, lcd: &mut Lcd, s: &Settings) {
        let t = &self.theme;
        self.header(lcd, s, "SETTINGS", &[]);
        let keys = match self.tab {
            CALL_TAB => ["TAB", "DEL", "CLR", "BACK"],
            MEM_TAB => self.mem_tab_keys(),
            _ => ["TAB", "-", "+", "BACK"],
        };
        t.softkeys(lcd, keys);
        let mut x = 4;
        for (i, name) in TABS.iter().enumerate() {
            let w = name.len() as i32 * TINY_W + 2 * TAB_PAD;
            let (fg, _, bg) = if i == self.tab { t.select(lcd, x, TAB_Y, w, TAB_H) } else { (t.muted, t.muted, t.bg) };
            tiny(lcd, x + TAB_PAD, TAB_Y + 2, name, fg, bg);
            x += w + TAB_GAP;
        }
        rect(lcd, 0, TAB_Y + TAB_H + 1, MAIN, 1, t.rule());
        for i in 0..rows(self.tab).len() {
            self.draw_row(lcd, s, i);
        }
        let note = match self.tab {
            SOUND_TAB => "NOISE AND FADING: DRILLS, QSO, HUNT",
            RECEIVE_TAB => "SPEED OF WHAT YOU HEAR: BOTS, DRILLS",
            QSO_TAB => "SAME SPEEDS AS THE RECV TAB",
            _ => "",
        };
        let note_y = ROW_Y + rows(self.tab).len() as i32 * ROW_STEP + NOTE_GAP;
        tiny(lcd, 10, note_y, note, t.muted, t.bg);
        if self.tab == THEME_TAB {
            self.draw_swatches(lcd, s);
        }
        if self.tab == CALL_TAB {
            self.draw_callsign(lcd, s);
        }
        if self.tab == MEM_TAB {
            self.draw_mem_tab(lcd);
        }
    }

    fn draw_row(&self, lcd: &mut Lcd, s: &Settings, i: usize) {
        let t = &self.theme;
        let (label, field) = rows(self.tab)[i];
        let y = ROW_Y + i as i32 * ROW_STEP;
        rect(lcd, 0, y - 1, MAIN, ROW_STEP, t.bg);
        let (fg, value_fg, bg) = if i == self.row {
            let (fg, _, bg) = t.select(lcd, 2, y - 1, MAIN - 6, ROW_STEP);
            (fg, fg, bg)
        } else {
            (t.fg, t.accent, t.bg)
        };
        text(lcd, 10, y + 1, label, fg, bg);
        let value = value(s, field);
        let shown = if self.editing && i == self.row {
            Text::<20>::format(format_args!("< {} >", value.as_str()))
        } else {
            Text::format(format_args!("{}", value.as_str()))
        };
        text_right(lcd, MAIN - 12, y + 1, shown.as_str(), value_fg, bg);
    }

    fn draw_swatches(&self, lcd: &mut Lcd, s: &Settings) {
        let t = &self.theme;
        for i in 0..PALETTE_COUNT {
            let x = 6 + (i % SWATCHES_PER_ROW) as i32 * 44;
            let y = SWATCH_Y + (i / SWATCHES_PER_ROW) as i32 * 30;
            rect(lcd, x - 2, y - 2, 42, 26, if i == s.palette { t.fg } else { t.bg });
            rect(lcd, x, y, 38, 22, Rgb565::new(0, 0, 0));
            swatch(lcd, t, self.boot_count, i, x + 2, y + 2, 34, 18);
        }
        if s.palette == DAILY {
            tiny(lcd, 4, PALETTE_NOTE_Y, "A NEW FLAG EACH BOOT", t.muted, t.bg);
        }
    }

    fn draw_callsign(&self, lcd: &mut Lcd, s: &Settings) {
        let t = &self.theme;
        rect(lcd, 0, CALL_Y - 4, MAIN, 7 * CALL_PX + 8, t.bg);
        let end = blocks(lcd, 6, CALL_Y, s.callsign(), CALL_PX, CALL_GAP, t.slant(), t.fg, None);
        let cursor = if s.callsign().is_empty() { 6 } else { end + CALL_GAP };
        para(lcd, cursor, CALL_Y + 7 * CALL_PX - 3, 5 * CALL_PX, 3, t.skew(), t.accent);
        tiny(lcd, 6, 104, "KEY YOUR CALL ON THE PADDLE", t.muted, t.bg);
        tiny(lcd, 6, 118, "USED BY THE QSO BOT AND AT BOOT", t.muted, t.bg);
    }

    pub(super) fn settings_act(&mut self, lcd: &mut Lcd, s: &mut Settings, action: Action) {
        match action {
            Action::Soft(0) => return self.next_tab(lcd, s),
            Action::Soft(3) => return self.close_settings(lcd, s),
            _ => {}
        }
        if self.tab == CALL_TAB {
            return self.call_act(lcd, s, action);
        }
        if self.tab == MEM_TAB {
            return self.mem_tab_act(lcd, action);
        }
        let field = rows(self.tab)[self.row].1;
        match action {
            Action::Turn(delta) if self.editing => self.adjust(lcd, s, field, delta),
            Action::Turn(delta) => {
                let next = (self.row as i32 + delta as i32).clamp(0, rows(self.tab).len() as i32 - 1) as usize;
                if next != self.row {
                    let old = core::mem::replace(&mut self.row, next);
                    self.draw_row(lcd, s, old);
                    self.draw_row(lcd, s, next);
                }
            }
            Action::Click => {
                self.editing = !self.editing;
                self.draw_row(lcd, s, self.row);
            }
            Action::Soft(key @ (1 | 2)) => self.adjust(lcd, s, field, if key == 1 { -1 } else { 1 }),
            _ => {}
        }
    }

    fn next_tab(&mut self, lcd: &mut Lcd, s: &Settings) {
        self.tab = (self.tab + 1) % TABS.len();
        self.row = 0;
        self.editing = false;
        self.draw(lcd, s);
    }

    fn adjust(&mut self, lcd: &mut Lcd, s: &mut Settings, field: Field, delta: i8) {
        s.adjust(field, delta);
        if matches!(field, Field::Style | Field::Palette) {
            self.theme = Theme::new(s.style, s.palette, self.boot_count);
            self.draw(lcd, s);
        } else if field == Field::Output {
            // OUT LINES names its values by the output type.
            for i in 0..rows(self.tab).len() {
                self.draw_row(lcd, s, i);
            }
        } else {
            self.draw_row(lcd, s, self.row);
        }
    }

    fn call_act(&mut self, lcd: &mut Lcd, s: &mut Settings, action: Action) {
        match action {
            Action::Soft(1) => {
                s.pop_callsign();
                self.draw_callsign(lcd, s);
            }
            Action::Soft(2) => {
                while !s.callsign().is_empty() {
                    s.pop_callsign();
                }
                self.draw_callsign(lcd, s);
            }
            _ => {}
        }
    }

    pub(super) fn settings_decoded(&mut self, lcd: &mut Lcd, s: &mut Settings, decoded: Decoded) {
        if self.tab == MEM_TAB {
            return self.mem_tab_decoded(lcd, decoded);
        }
        if self.tab == CALL_TAB
            && let Decoded::Char(c) = decoded
            && s.push_callsign(c as char)
        {
            self.draw_callsign(lcd, s);
        }
    }
}

/// A palette's colors as stripes; the daily flag shows a sample of the flags it picks from.
#[allow(clippy::too_many_arguments)]
fn swatch(lcd: &mut Lcd, t: &Theme, boot_count: u16, palette: u8, x: i32, y: i32, w: i32, h: i32) {
    if palette == DAILY {
        const SHOWN: i32 = 5;
        for k in 0..SHOWN {
            let (accent, _) = theme::palette_pair(DAILY + 1 + k as u8, boot_count);
            rect(lcd, x + k * w / SHOWN, y, (k + 1) * w / SHOWN - k * w / SHOWN, h, accent);
        }
        return;
    }
    let stripes = theme::palette_stripes(palette, boot_count);
    let n = stripes.len() as i32;
    for (i, &color) in stripes.iter().enumerate() {
        let i = i as i32;
        rect(lcd, x, y + i * h / n, w, (i + 1) * h / n - i * h / n, color);
    }
    if n == 1 {
        let (_, echo) = theme::palette_pair(palette, boot_count);
        para(lcd, x + w - 12, y, 8, h, t.skew(), echo);
    }
}

pub(super) fn value(s: &Settings, field: Field) -> Text<{ config::SHOWN_LEN }> {
    let on_off = |on: bool| if on { "ON" } else { "OFF" };
    let mut t = Text::new();
    let _ = match field {
        Field::Speed => write!(t, "{} WPM", s.wpm),
        Field::Mode => write!(t, "{}", mode_name(s.mode)),
        Field::Swap => write!(t, "{}", if s.swap { "REVERSED" } else { "NORMAL" }),
        Field::KeyOut => write!(t, "{}", on_off(s.key_out)),
        Field::Output => write!(t, "{}", if s.output == Output::Paddle { "PADDLE" } else { "STRAIGHT" }),
        Field::OutputSwap => write!(
            t,
            "{}",
            match (s.output, s.output_swap) {
                (Output::Straight, false) => "TIP KEYS",
                (Output::Straight, true) => "RING KEYS",
                (Output::Paddle, false) => "DIT ON TIP",
                (Output::Paddle, true) => "DIT ON RING",
            }
        ),
        Field::Tone => write!(t, "{} HZ", s.tone_hz),
        Field::Volume => write!(t, "{}", s.volume),
        Field::Style => write!(t, "{}", theme::style_name(s.style)),
        Field::Palette => write!(t, "{}", theme::palette_name(s.palette)),
        Field::RxSpeed if s.rx_wpm == 0 => write!(t, "MATCH"),
        Field::RxSpeed => write!(t, "{} WPM", s.rx_wpm),
        Field::RxEffective if s.rx_effective == 0 => write!(t, "OFF"),
        Field::RxEffective => write!(t, "{} WPM", s.rx_effective),
        Field::Pileup => write!(
            t,
            "{}",
            match s.pileup {
                Pileup::Off => "OFF",
                Pileup::TakeTurns => "TAKE TURNS",
                Pileup::Overlap => "OVERLAP",
            }
        ),
        Field::Fist => write!(t, "{}", if s.human_fist { "HUMAN" } else { "CLEAN" }),
        Field::Spread => write!(t, "{}", on_off(s.spread)),
        Field::Reveal => write!(t, "{}", if s.reveal_lines { "EACH LINE" } else { "WHOLE OVER" }),
        Field::Wave => write!(t, "{}", if s.square { "SQUARE" } else { "SINE" }),
        Field::DecoderBoard => write!(t, "{}", on_off(s.decoder_board)),
        Field::UsbOut => write!(
            t,
            "{}",
            match s.usb_out {
                UsbOut::Off => "OFF",
                UsbOut::Text => "TEXT",
                UsbOut::Vband => "VBAND",
                UsbOut::Radio => "RADIO",
                UsbOut::Serial => "SERIAL",
            }
        ),
        Field::Noise | Field::Fading => {
            let level = if field == Field::Noise { s.noise } else { s.fading };
            if level == 0 { write!(t, "OFF") } else { write!(t, "{level}") }
        }
        Field::Brightness => write!(t, "{}%", s.brightness as u16 * 100 / MAX_BRIGHTNESS as u16),
    };
    t
}
