//! Boot screen: Morse rain in a pride flag's colors builds the callsign in block letters, then
//! a tagline and the startup checks appear. Drawn on the 32 x 15 text grid.

use embedded_graphics::{
    pixelcolor::Rgb565,
    prelude::*,
    primitives::Rectangle,
};

use crate::fonts::SMALL;
use crate::gfx::{self, rgb};
use crate::lcd::Lcd;
use crate::text::Text;
use crate::theme::{INK, Theme};

pub const COLS: usize = 32;
const ROWS: usize = 15;
const CELL_W: i32 = 10;
const CELL_H: i32 = 16;

const BIG_ROW: usize = 1;
const BIG_COL: usize = 1;
const BIG_ROWS: usize = 5;
/// Five block letters of 5 columns plus a 1-column gap fill 29 of the 32 columns; a sixth
/// letter fits only with the gaps closed.
const MAX_CALLSIGN: usize = 6;
const SPACED_CALLSIGN: usize = 5;
const TAGLINE_ROW: usize = 7;
const LOG_ROW: usize = 9;
pub const LOG_LINES: usize = 6;
const RESULT_COL: usize = 29;

// Timeline in units of 1/1.5 ms: the animation runs 1.5 times faster than these numbers read.
const TAGLINE_START: u32 = 2100;
const TAGLINE_CHAR: u32 = 25;
const LOG_START: u32 = 2500;
const LOG_STEP: u32 = 160;
const LOG_CHAR: u32 = 8;
const RESULT_DELAY: u32 = 80;
const END: u32 = LOG_START + LOG_LINES as u32 * LOG_STEP + 600;
const TRAIL: i32 = 7;

const FAIL: Rgb565 = rgb(0xE24B4A);

/// 5 x 5 block letters, one row per byte, bit 4 is the left column.
fn block_glyph(c: char) -> Option<[u8; 5]> {
    let rows = match c.to_ascii_uppercase() {
        'A' => [0b01110, 0b10001, 0b11111, 0b10001, 0b10001],
        'B' => [0b11110, 0b10001, 0b11110, 0b10001, 0b11110],
        'C' => [0b01111, 0b10000, 0b10000, 0b10000, 0b01111],
        'D' => [0b11110, 0b10001, 0b10001, 0b10001, 0b11110],
        'E' => [0b11111, 0b10000, 0b11110, 0b10000, 0b11111],
        'F' => [0b11111, 0b10000, 0b11110, 0b10000, 0b10000],
        'G' => [0b01111, 0b10000, 0b10111, 0b10001, 0b01111],
        'H' => [0b10001, 0b10001, 0b11111, 0b10001, 0b10001],
        'I' => [0b11111, 0b00100, 0b00100, 0b00100, 0b11111],
        'J' => [0b00111, 0b00010, 0b00010, 0b10010, 0b01100],
        'K' => [0b10001, 0b10010, 0b11100, 0b10010, 0b10001],
        'L' => [0b10000, 0b10000, 0b10000, 0b10000, 0b11111],
        'M' => [0b10001, 0b11011, 0b10101, 0b10001, 0b10001],
        'N' => [0b10001, 0b11001, 0b10101, 0b10011, 0b10001],
        'O' => [0b01110, 0b10001, 0b10001, 0b10001, 0b01110],
        'P' => [0b11110, 0b10001, 0b11110, 0b10000, 0b10000],
        'Q' => [0b01110, 0b10001, 0b10101, 0b10010, 0b01101],
        'R' => [0b11110, 0b10001, 0b11110, 0b10010, 0b10001],
        'S' => [0b01111, 0b10000, 0b01110, 0b00001, 0b11110],
        'T' => [0b11111, 0b00100, 0b00100, 0b00100, 0b00100],
        'U' => [0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'V' => [0b10001, 0b10001, 0b10001, 0b01010, 0b00100],
        'W' => [0b10001, 0b10001, 0b10101, 0b11011, 0b10001],
        'X' => [0b10001, 0b01010, 0b00100, 0b01010, 0b10001],
        'Y' => [0b10001, 0b01010, 0b00100, 0b00100, 0b00100],
        'Z' => [0b11111, 0b00010, 0b00100, 0b01000, 0b11111],
        '0' => [0b01110, 0b10011, 0b10101, 0b11001, 0b01110],
        '1' => [0b00100, 0b01100, 0b00100, 0b00100, 0b01110],
        '2' => [0b11110, 0b00001, 0b01110, 0b10000, 0b11111],
        '3' => [0b11110, 0b00001, 0b01110, 0b00001, 0b11110],
        '4' => [0b10001, 0b10001, 0b11111, 0b00001, 0b00001],
        '5' => [0b11111, 0b10000, 0b11110, 0b00001, 0b11110],
        '6' => [0b01110, 0b10000, 0b11110, 0b10001, 0b01110],
        '7' => [0b11111, 0b00001, 0b00010, 0b00100, 0b00100],
        '8' => [0b01110, 0b10001, 0b01110, 0b10001, 0b01110],
        '9' => [0b01110, 0b10001, 0b01111, 0b00001, 0b01110],
        '/' => [0b00001, 0b00010, 0b00100, 0b01000, 0b10000],
        _ => return None,
    };
    Some(rows)
}

/// One startup check: its description and whether it passed.
pub struct Check {
    pub text: Text<24>,
    pub ok: bool,
}

#[derive(Clone, Copy, PartialEq)]
enum Cell {
    Blank,
    Char(u8, Rgb565),
    /// Part of a block letter, colored by the flag stripe at its pixel rows.
    Block,
}

pub struct Boot {
    stripes: &'static [Rgb565],
    bg: Rgb565,
    fg: Rgb565,
    muted: Rgb565,
    seed: u32,
    big: [[bool; COLS]; ROWS],
    tagline: Text<COLS>,
    checks: [Check; LOG_LINES],
    shown: [[Cell; COLS]; ROWS],
}

impl Boot {
    /// `variant` picks the rain pattern; pass a number that changes every boot. The rain takes
    /// `stripes`, and the text and background take the theme's colors.
    pub fn new(variant: u16, theme: &Theme, stripes: &'static [Rgb565], callsign: &str, tagline: Text<COLS>, checks: [Check; LOG_LINES]) -> Self {
        let mut big = [[false; COLS]; ROWS];
        // The base call is the longest part around a '/': K5MME in VE3/K5MME/P.
        let call = callsign.split('/').max_by_key(|part| part.len()).unwrap_or("");
        let step = if call.len() > SPACED_CALLSIGN { 5 } else { 6 };
        for (i, c) in call.chars().take(MAX_CALLSIGN).enumerate() {
            let rows = block_glyph(c).unwrap_or([0b11111; 5]);
            for (r, bits) in rows.iter().enumerate() {
                for col in 0..5 {
                    big[BIG_ROW + r][BIG_COL + i * step + col] = bits & (0b10000 >> col) != 0;
                }
            }
        }
        Boot {
            stripes,
            bg: theme.bg,
            fg: theme.fg,
            muted: theme.muted,
            seed: hash(variant as u32, 0x5EED),
            big,
            tagline,
            checks,
            shown: [[Cell::Blank; COLS]; ROWS],
        }
    }

    /// Draws the frame `ms` milliseconds after the start. Returns false once the animation is over.
    pub fn draw(&mut self, lcd: &mut Lcd, ms: u32) -> bool {
        let t = (ms * 3 / 2).min(END);
        let frame = self.frame(t);
        for (r, row) in frame.iter().enumerate() {
            for (c, &cell) in row.iter().enumerate() {
                if cell != self.shown[r][c] {
                    self.draw_cell(lcd, r, c, cell);
                    self.shown[r][c] = cell;
                }
            }
        }
        t < END
    }

    fn frame(&self, t: u32) -> [[Cell; COLS]; ROWS] {
        let mut grid = [[Cell::Blank; COLS]; ROWS];
        for c in 0..COLS {
            let speed = 9.0 + unit(c as u32, self.seed) * 9.0; // rows per second
            let delay = unit(c as u32, self.seed.wrapping_add(1)) * 700.0;
            let head = (t as f32 - delay) / 1000.0 * speed;
            let head_row = floor(head);
            for k in 0..TRAIL {
                let r = head_row - k;
                if r < 0 || r >= ROWS as i32 {
                    continue;
                }
                let r = r as usize;
                if self.big[r][c] || (t > LOG_START && r >= LOG_ROW) {
                    continue;
                }
                let stripe = self.stripe((r as f32 + 0.5) / ROWS as f32);
                let ch = if unit(c as u32, (r as u32).wrapping_add(self.seed)) < 0.5 { b'.' } else { b'-' };
                grid[r][c] = Cell::Char(ch, if k == 0 { stripe } else { gfx::mix(stripe, self.bg, 60) });
            }
            for (r, (row, big)) in grid.iter_mut().zip(&self.big).enumerate() {
                if big[c] && head >= r as f32 {
                    row[c] = Cell::Block;
                }
            }
        }

        if t > TAGLINE_START {
            let tagline = self.tagline.as_str().as_bytes();
            let start = (COLS - tagline.len().min(COLS)) / 2;
            let shown = (((t - TAGLINE_START) / TAGLINE_CHAR) as usize).min(tagline.len()).min(COLS);
            for (i, &ch) in tagline[..shown].iter().enumerate() {
                grid[TAGLINE_ROW][start + i] = Cell::Char(ch, self.fg);
            }
        }

        let result_colors = self.stripes.iter().copied().filter(|&s| s != INK);
        for (i, (check, ok_color)) in self.checks.iter().zip(result_colors.cycle()).enumerate() {
            let t0 = LOG_START + i as u32 * LOG_STEP;
            if t < t0 {
                continue;
            }
            let text = check.text.as_str().as_bytes();
            let shown = (((t - t0) / LOG_CHAR) as usize).min(text.len());
            for (col, &ch) in text[..shown].iter().enumerate() {
                grid[LOG_ROW + i][1 + col] = Cell::Char(ch, self.muted);
            }
            if t > t0 + text.len() as u32 * LOG_CHAR + RESULT_DELAY {
                let (result, color) = if check.ok { ("ok", ok_color) } else { ("fail", FAIL) };
                let col = RESULT_COL + 2 - result.len();
                for (j, ch) in result.bytes().enumerate() {
                    grid[LOG_ROW + i][col + j] = Cell::Char(ch, color);
                }
            }
        }
        grid
    }

    fn stripe(&self, fraction: f32) -> Rgb565 {
        let n = self.stripes.len();
        self.stripes[((fraction * n as f32) as usize).min(n - 1)]
    }

    fn draw_cell(&self, lcd: &mut Lcd, r: usize, c: usize, cell: Cell) {
        let origin = Point::new(c as i32 * CELL_W, r as i32 * CELL_H);
        let area = Rectangle::new(origin, Size::new(CELL_W as u32, CELL_H as u32));
        let _ = match cell {
            Cell::Blank => lcd.fill_solid(&area, self.bg),
            Cell::Char(ch, color) => {
                let byte = [ch];
                let text = core::str::from_utf8(&byte).unwrap_or("?");
                SMALL.draw(lcd, origin, text, color, self.bg)
            }
            Cell::Block => {
                let top = (BIG_ROW as i32 * CELL_H) as f32;
                let height = (BIG_ROWS as i32 * CELL_H) as f32;
                let colors = (0..CELL_H).flat_map(|y| {
                    let stripe = self.stripe(((origin.y + y) as f32 - top) / height);
                    (0..CELL_W).map(move |x| if x == CELL_W - 1 || y == CELL_H - 1 { self.bg } else { stripe })
                });
                lcd.fill_contiguous(&area, colors)
            }
        };
    }
}

fn hash(a: u32, b: u32) -> u32 {
    let h = a.wrapping_mul(374_761_393).wrapping_add(b.wrapping_mul(668_265_263));
    let h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    h ^ (h >> 16)
}

/// Repeatable pseudo-random number in 0..1 for the pair (a, b).
fn unit(a: u32, b: u32) -> f32 {
    (hash(a, b) >> 8) as f32 / (1u32 << 24) as f32
}

fn floor(x: f32) -> i32 {
    let i = x as i32;
    if i as f32 > x { i - 1 } else { i }
}
