//! What the QSO and Hunt screens share under their transcripts: a rule, a NOW row with the log
//! count, four chips for the contact, your over, and what to send next.

use embedded_graphics::pixelcolor::Rgb565;

use super::COLS;
use crate::gfx::{CHAR_W, TINY_W, rect, text, text_cols, tiny};
use crate::lcd::Lcd;
use crate::theme::{MAIN, Theme};

pub(super) const RULE_Y: i32 = 155;
const NOW_Y: i32 = 159;
const LOG_X: i32 = 112;
const CHIP_Y: i32 = 172;
/// Chip widths, sized to their values: a call up to 8 characters (K1ABC/P), a report or state,
/// and a place up to 6 (SUMMIT).
const CHIP_W: [i32; 4] = [88, 52, 52, 75];
const CHIP_H: i32 = 30;
/// Where a chip's value starts, from its left edge.
const VALUE_X: i32 = 5;
const _: () = assert!(2 + CHIP_W[0] + CHIP_W[1] + CHIP_W[2] + CHIP_W[3] + 3 <= MAIN);
const _: () = assert!(VALUE_X + 8 * CHAR_W < CHIP_W[0] - 1);
const _: () = assert!(VALUE_X + 6 * CHAR_W < CHIP_W[3] - 1);
/// Labels run up to 6 tiny characters: COPIED, MISSED, BUSTED.
const _: () = assert!(VALUE_X + 6 * TINY_W < CHIP_W[1] - 1 && VALUE_X + 6 * TINY_W < CHIP_W[2] - 1);
const OVER_Y: i32 = 207;
const FOOT_Y: i32 = 226;

pub(super) struct Chip<'a> {
    pub label: &'a str,
    /// Empty shows as "...".
    pub value: &'a str,
    /// Colors the chip's edge; the rule color without one.
    pub mark: Option<Rgb565>,
}

/// The NOW row: what the chips show, then the log count in `log_color`.
pub(super) fn draw_now(lcd: &mut Lcd, t: &Theme, label: &str, log: &str, log_color: Rgb565) {
    rect(lcd, 0, NOW_Y, MAIN, 12, t.bg);
    tiny(lcd, 2, NOW_Y, label, t.muted, t.bg);
    tiny(lcd, LOG_X, NOW_Y, log, log_color, t.bg);
}

pub(super) fn draw_chips(lcd: &mut Lcd, t: &Theme, chips: [Chip; 4]) {
    let mut x = 2;
    for (chip, w) in chips.into_iter().zip(CHIP_W) {
        let edge = chip.mark.unwrap_or(t.rule());
        rect(lcd, x, CHIP_Y, w, CHIP_H, edge);
        rect(lcd, x + 1, CHIP_Y + 1, w - 2, CHIP_H - 2, t.bg);
        rect(lcd, x, CHIP_Y, 3, CHIP_H, edge);
        tiny(lcd, x + VALUE_X, CHIP_Y + 3, chip.label, t.muted, t.bg);
        let (value, color) = if chip.value.is_empty() { ("...", t.muted) } else { (chip.value, t.fg) };
        text(lcd, x + VALUE_X, CHIP_Y + 13, value, color, t.bg);
        x += w + 1;
    }
}

/// The end of your over with a cursor after it, and the next step in the footer.
pub(super) fn draw_over(lcd: &mut Lcd, t: &Theme, over: &str, next_step: &str) {
    let shown = &over[over.len().saturating_sub(COLS - 1)..];
    text_cols(lcd, 0, OVER_Y, shown, COLS, t.fg, t.bg);
    rect(lcd, shown.len() as i32 * CHAR_W + 1, OVER_Y + 2, CHAR_W - 2, 13, t.accent);
    rect(lcd, 0, FOOT_Y, MAIN, 12, t.bg);
    tiny(lcd, 2, FOOT_Y, next_step, t.muted, t.bg);
}
