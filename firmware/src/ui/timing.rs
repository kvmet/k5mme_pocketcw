//! Timing: a zoomed tape with the error of each element, and session histograms of mark and
//! gap lengths with the averages under them.

use core::fmt::Write;

use pocketcw_logic::{
    input::Action,
    settings::Settings,
    timing::{BINS, GAP_RANGE, Kind, MARK_RANGE, Quality},
};

use super::home::{number, units};
use super::{Ui, lanes, mode_name, settings::Section};
use crate::gfx::{TINY_W, rect, text_cols, tiny};
use crate::lcd::Lcd;
use crate::text::Text;
use crate::theme::{MAIN, quality_color};

const HIST_Y: i32 = 112;
const HIST_H: i32 = 26;
const HIST_W: i32 = 124;
/// Below the histograms' labels, so the last line ends above the footer.
const LINES_Y: i32 = HIST_Y + 14 + HIST_H + 16;
const LINE_STEP: i32 = 18;

impl Ui {
    pub(super) fn draw_timing(&mut self, lcd: &mut Lcd, s: &Settings) {
        self.header(lcd, s, "TIMING", &[mode_name(s.mode)]);
        self.softkeys(lcd, ["SET", "", "", "BACK"]);
        self.tape.clear(lcd, &self.theme, lanes(s));
        rect(lcd, 0, 108, MAIN, 1, self.theme.rule());
        self.draw_timing_stats(lcd);
    }

    pub(super) fn draw_timing_stats(&self, lcd: &mut Lcd) {
        let st = &self.stats;
        self.histogram(lcd, 4, "MARKS", &st.marks, MARK_RANGE, &[Kind::Dit, Kind::Dah], Kind::mark);
        self.histogram(lcd, 144, "GAPS", &st.gaps, GAP_RANGE, &[Kind::ElementGap, Kind::CharGap, Kind::WordGap], Kind::gap);

        let t = &self.theme;
        let ms = |m: &pocketcw_logic::timing::Moments| (m.mean().map(|v| v as i32), m.sd().map(|v| v as i32));
        let mut line = Text::<32>::new();
        let (mean, sd) = ms(&st.dit);
        let _ = match (mean, sd) {
            (Some(m), Some(d)) => write!(line, "DIT {m}MS SD{d}  RATIO "),
            _ => write!(line, "DIT --  RATIO "),
        };
        number(&mut line, st.ratio());
        text_cols(lcd, 0, LINES_Y, line.as_str(), super::COLS, t.fg, t.bg);

        line.clear();
        let (mean, sd) = ms(&st.dah);
        let _ = match (mean, sd) {
            (Some(m), Some(d)) => write!(line, "DAH {m}MS SD{d}  "),
            _ => write!(line, "DAH --  "),
        };
        number(&mut line, st.effective_wpm());
        let _ = write!(line, " WPM");
        text_cols(lcd, 0, LINES_Y + LINE_STEP, line.as_str(), super::COLS, t.fg, t.bg);

        line.clear();
        let _ = write!(line, "GAP ");
        units(&mut line, st.element_gap.mean());
        let _ = write!(line, " CHR ");
        units(&mut line, st.char_gap.mean());
        let _ = write!(line, " WRD ");
        units(&mut line, st.word_gap.mean());
        text_cols(lcd, 0, LINES_Y + 2 * LINE_STEP, line.as_str(), super::COLS, t.fg, t.bg);
    }

    /// Bars colored by the timing quality of each bin, with a mark at each kind's ideal length.
    /// `kind_of` reads a length in dots as the decoder does.
    #[allow(clippy::too_many_arguments)]
    fn histogram(&self, lcd: &mut Lcd, x: i32, title: &str, bins: &[u16; BINS], range: f32, kinds: &[Kind], kind_of: fn(f32) -> Kind) {
        let t = &self.theme;
        rect(lcd, x, HIST_Y, HIST_W, HIST_H + 30, t.bg);
        tiny(lcd, x, HIST_Y, title, t.muted, t.bg);
        let top = HIST_Y + 14;
        let peak = bins.iter().copied().max().unwrap_or(0).max(1) as i32;
        for target in kinds.iter().map(|k| k.ideal_units()) {
            let tx = x + (target / range * HIST_W as f32) as i32;
            for y in (top..top + HIST_H).step_by(3) {
                rect(lcd, tx, y, 1, 1, t.muted);
            }
            let label = Text::<2>::format(format_args!("{}", target as i32));
            tiny(lcd, tx - TINY_W / 2, top + HIST_H + 2, label.as_str(), t.muted, t.bg);
        }
        let bin_w = HIST_W / BINS as i32;
        for (i, &count) in bins.iter().enumerate() {
            let h = count as i32 * HIST_H / peak;
            if h == 0 {
                continue;
            }
            let units = (i as f32 + 0.5) * range / BINS as f32;
            let color = quality_color(Quality::of(kind_of(units), units));
            rect(lcd, x + i as i32 * bin_w, top + HIST_H - h, bin_w - 1, h, color);
        }
        rect(lcd, x, top + HIST_H, HIST_W, 1, t.rule());
    }

    pub(super) fn timing_act(&mut self, lcd: &mut Lcd, s: &mut Settings, action: Action) {
        match action {
            Action::Soft(0) => self.open_settings_at(lcd, s, Section::Keyer),
            Action::Soft(3) => self.back(lcd, s),
            _ => {}
        }
    }
}
