//! The menu: one row per screen, with what it holds beside the name.

use pocketcw_logic::{drill::Kind, input::Action, qso::Program, settings::Settings};

use super::{Screen, Ui, settings::KEYER_TAB};
use crate::gfx::{TINY_W, rect, text, tiny};
use crate::lcd::Lcd;
use crate::text::Text;
use crate::theme::MAIN;

const MENU_Y: i32 = 25;
const MENU_STEP: i32 = 21;
/// With RADIO shown, eleven rows fit only closer together.
const MENU_STEP_RADIO: i32 = 19;

#[derive(Clone, Copy)]
enum Item {
    Open(Screen),
    Qso(Program),
    Receive,
    Hunt,
}

/// RADIO stays last: it shows only with the decoder board on.
const ITEMS: [(&str, &str, Item); 11] = [
    ("KEYER", "SPEED  DECODE  TAPE", Item::Open(Screen::Home)),
    ("TIMING", "TAPE  HISTOGRAMS", Item::Open(Screen::Timing)),
    ("RECEIVE", "DRILLS  RAGCHEW", Item::Receive),
    ("POTA", "ACTIVATE A PARK", Item::Qso(Program::Pota)),
    ("SOTA", "ACTIVATE A SUMMIT", Item::Qso(Program::Sota)),
    ("HUNT", "PARKS  SUMMITS  P2P", Item::Hunt),
    ("LETTERS", "ACCURACY  MIXUPS", Item::Open(Screen::Letters)),
    ("PROGRESS", "SESSIONS  TOTALS", Item::Open(Screen::Progress)),
    ("SETTINGS", "KEYER SOUND QSO MEM", Item::Open(Screen::Settings)),
    ("SYSTEM", "VERSION  BATTERY", Item::Open(Screen::System)),
    ("RADIO", "DECODER BOARD", Item::Open(Screen::Radio)),
];

fn shown(s: &Settings) -> usize {
    if s.decoder_board { ITEMS.len() } else { ITEMS.len() - 1 }
}

fn step(s: &Settings) -> i32 {
    if s.decoder_board { MENU_STEP_RADIO } else { MENU_STEP }
}

impl Ui {
    pub(super) fn draw_menu(&mut self, lcd: &mut Lcd, s: &Settings) {
        self.menu = self.menu.min(shown(s) - 1);
        self.theme.header(lcd, "MENU", &[s.callsign()]);
        self.theme.softkeys(lcd, ["SET", "", "", "BACK"]);
        for i in 0..shown(s) {
            self.draw_menu_item(lcd, s, i);
        }
    }

    fn draw_menu_item(&self, lcd: &mut Lcd, s: &Settings, i: usize) {
        let t = &self.theme;
        let step = step(s);
        let y = MENU_Y + i as i32 * step;
        let (name, sub, _) = ITEMS[i];
        rect(lcd, 0, y, MAIN, step - 2, t.bg);
        let (fg, sub_fg, bg) = if i == self.menu { t.select(lcd, 2, y, MAIN - 6, step - 2) } else { (t.fg, t.muted, t.bg) };
        // Centres the 16-row name in the row's highlight: 2 rows down at step 21, 1 at step 19.
        let pad = (step - 2 - 16) / 2 + 1;
        let number = Text::<2>::format(format_args!("{:02}", i + 1));
        tiny(lcd, 8, y + pad + 3, number.as_str(), sub_fg, bg);
        text(lcd, 30, y + pad, name, fg, bg);
        tiny(lcd, MAIN - 10 - sub.len() as i32 * TINY_W, y + pad + 3, sub, sub_fg, bg);
    }

    pub(super) fn menu_act(&mut self, lcd: &mut Lcd, s: &mut Settings, action: Action) {
        match action {
            Action::Turn(delta) => {
                let next = (self.menu as i32 + delta as i32).clamp(0, shown(s) as i32 - 1) as usize;
                if next != self.menu {
                    let old = core::mem::replace(&mut self.menu, next);
                    self.draw_menu_item(lcd, s, old);
                    self.draw_menu_item(lcd, s, next);
                }
            }
            Action::Soft(0) => self.open_settings_at(lcd, s, KEYER_TAB),
            Action::Click => match ITEMS[self.menu].2 {
                Item::Open(screen) => {
                    self.tab = 0;
                    self.open(lcd, s, screen);
                }
                Item::Qso(program) => self.open_qso(lcd, s, program),
                Item::Receive => self.open_receive(lcd, s, Kind::Koch),
                Item::Hunt => self.open_hunt(lcd, s),
            },
            Action::Soft(3) => self.open(lcd, s, Screen::Home),
            _ => {}
        }
    }
}
