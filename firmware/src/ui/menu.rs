//! The menu: a top list and the lists it opens. Each row names a screen or a list; the footer
//! says what the selected row holds, and whose idea it was.

use pocketcw_logic::{drill::Kind, input::Action, qso::Program, settings::Settings};

use super::{Screen, Session, Ui, settings::Section};
use crate::gfx::{TINY_W, rect, text, tiny};
use crate::lcd::Lcd;
use crate::text::Text;
use crate::theme::MAIN;

const MENU_Y: i32 = 25;
const MENU_STEP: i32 = 21;
const ABOUT_Y: i32 = 190;
const CREDIT_Y: i32 = 204;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum List {
    Top,
    Receive,
    OnAir,
    Stats,
}
pub(super) const LISTS: usize = 4;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Item {
    Resume,
    List(List),
    Open(Screen),
    Drill(Kind),
    Chew { quiz: bool },
    Qso(Program),
    Hunt,
}

struct Row {
    name: &'static str,
    about: &'static str,
    credit: &'static str,
    item: Item,
}

const fn row(name: &'static str, about: &'static str, credit: &'static str, item: Item) -> Row {
    Row { name, about, credit, item }
}

const TOP: [Row; 7] = [
    row("RESUME", "PICK UP WHERE YOU LEFT OFF", "", Item::Resume),
    row("RECEIVE", "KOCH  WORDS  CALLS  NINJA  HEAD", "", Item::List(List::Receive)),
    row("ON AIR", "POTA  SOTA  HUNT  RAGCHEW", "", Item::List(List::OnAir)),
    row("TIMING", "ZOOMED TAPE AND HISTOGRAMS", "", Item::Open(Screen::Timing)),
    row("STATS", "LETTER ACCURACY AND PROGRESS", "", Item::List(List::Stats)),
    row("SETTINGS", "KEYER, SOUND, MEMORIES, CALL, THEME", "", Item::Open(Screen::Settings)),
    row("SYSTEM", "VERSION  HARDWARE CHECKS  BATTERY", "", Item::Open(Screen::System)),
];

const RECEIVE: [Row; 5] = [
    row("KOCH", "ONE MORE CHARACTER AT 90% RIGHT", "KOCH METHOD  LUDWIG KOCH", Item::Drill(Kind::Koch)),
    row("WORDS", "COMMON WORDS TO COPY AND KEY BACK", "", Item::Drill(Kind::Words)),
    row("CALLS", "CALLSIGNS TO COPY AND KEY BACK", "", Item::Drill(Kind::Calls)),
    row("NINJA", "A WORD IN MORSE, THEN SPOKEN", "MORSE CODE NINJA  AD0WE", Item::Drill(Kind::Ninja)),
    row("HEAD", "COPY WHOLE WORDS IN YOUR HEAD", "", Item::Drill(Kind::Head)),
];

/// RADIO stays last: it shows only with the decoder board on.
const ON_AIR: [Row; 6] = [
    row("POTA", "ACTIVATE A PARK AND WORK A PILEUP", "PARKS ON THE AIR", Item::Qso(Program::Pota)),
    row("SOTA", "ACTIVATE A SUMMIT, WORK A PILEUP", "SUMMITS ON THE AIR", Item::Qso(Program::Sota)),
    row("HUNT", "ANSWER ACTIVATORS ON THE AIR", "", Item::Hunt),
    row("CHEW", "LISTEN TO TWO STATIONS RAGCHEW", "", Item::Chew { quiz: false }),
    row("QUIZ", "A RAGCHEW WITH A QUESTION PER OVER", "", Item::Chew { quiz: true }),
    row("RADIO", "THE DECODER BOARD OF THE PRO", "", Item::Open(Screen::Radio)),
];

const STATS: [Row; 2] = [
    row("LETTERS", "ACCURACY AND MIXUPS PER CHARACTER", "", Item::Open(Screen::Letters)),
    row("PROGRESS", "PRACTICE TIME AND SESSION TOTALS", "", Item::Open(Screen::Progress)),
];

fn rows(list: List) -> &'static [Row] {
    match list {
        List::Top => &TOP,
        List::Receive => &RECEIVE,
        List::OnAir => &ON_AIR,
        List::Stats => &STATS,
    }
}

fn title(list: List) -> &'static str {
    match list {
        List::Top => "MENU",
        List::Receive => "RECEIVE",
        List::OnAir => "ON AIR",
        List::Stats => "STATS",
    }
}

const ALL: [List; LISTS] = [List::Top, List::Receive, List::OnAir, List::Stats];

impl Session {
    fn item(&self) -> Item {
        match self {
            Session::Qso(q) => Item::Qso(q.bot.program),
            Session::Hunt(_) => Item::Hunt,
            Session::Receive(rx) => Item::Drill(rx.kind()),
            Session::Ragchew(c) => Item::Chew { quiz: c.quiz },
        }
    }
}

impl Ui {
    fn shown(&self, s: &Settings, item: Item) -> bool {
        match item {
            Item::Resume => self.session.is_some(),
            Item::Open(Screen::Radio) => s.decoder_board,
            _ => true,
        }
    }

    /// The selected row's index in its list, moved off a row that no longer shows.
    fn cursor(&self, s: &Settings) -> usize {
        let rows = rows(self.list);
        let at = self.cursor[self.list as usize].min(rows.len() - 1);
        (at..rows.len()).chain((0..at).rev()).find(|&i| self.shown(s, rows[i].item)).unwrap_or(0)
    }

    pub(super) fn draw_menu(&mut self, lcd: &mut Lcd, s: &Settings) {
        let chips: &[&str] = if self.list == List::Top { &[s.callsign()] } else { &[] };
        self.header(lcd, s, title(self.list), chips);
        self.softkeys(lcd, ["SET", "", "", "BACK"]);
        self.cursor[self.list as usize] = self.cursor(s);
        rect(lcd, 0, ABOUT_Y - 6, MAIN, 1, self.theme.rule());
        for i in 0..rows(self.list).len() {
            self.draw_menu_row(lcd, s, i);
        }
        self.draw_menu_about(lcd);
    }

    /// Row `i` of the list in its slot among the shown rows; nothing when hidden.
    fn draw_menu_row(&self, lcd: &mut Lcd, s: &Settings, i: usize) {
        let rows = rows(self.list);
        let row = &rows[i];
        if !self.shown(s, row.item) {
            return;
        }
        let t = &self.theme;
        let slot = rows[..i].iter().filter(|r| self.shown(s, r.item)).count() as i32;
        let y = MENU_Y + slot * MENU_STEP;
        rect(lcd, 0, y, MAIN, MENU_STEP - 2, t.bg);
        let selected = i == self.cursor[self.list as usize];
        let (fg, sub_fg, bg) = if selected { t.select(lcd, 2, y, MAIN - 6, MENU_STEP - 2) } else { (t.fg, t.muted, t.bg) };
        text(lcd, 12, y + 2, row.name, fg, bg);
        let detail = self.detail(row.item);
        tiny(lcd, MAIN - 10 - detail.len() as i32 * TINY_W, y + 5, detail.as_str(), sub_fg, bg);
    }

    /// What shows at the right end of a row: where a drill stands, or that a list opens.
    fn detail(&self, item: Item) -> Text<12> {
        let paused = self.session.as_ref().map(Session::item);
        match item {
            Item::Resume => Text::format(format_args!("{}", self.session.as_ref().map_or("", Session::name))),
            _ if paused == Some(item) => Text::format(format_args!("PAUSED")),
            Item::List(_) => Text::format(format_args!(">")),
            Item::Drill(Kind::Koch) => Text::format(format_args!("LEVEL {}", self.letters.level)),
            Item::Drill(Kind::Head) => Text::format(format_args!("RUNG {}", self.head_rung)),
            _ => Text::new(),
        }
    }

    fn draw_menu_about(&self, lcd: &mut Lcd) {
        let t = &self.theme;
        let row = &rows(self.list)[self.cursor[self.list as usize]];
        rect(lcd, 0, ABOUT_Y, MAIN, CREDIT_Y + 12 - ABOUT_Y, t.bg);
        tiny(lcd, 2, ABOUT_Y, row.about, t.muted, t.bg);
        tiny(lcd, 2, CREDIT_Y, row.credit, t.dim, t.bg);
    }

    /// Opens `list` with `item` selected, or the list's last selection when it has no such row.
    fn open_list(&mut self, lcd: &mut Lcd, s: &Settings, list: List, item: Option<Item>) {
        if let Some(i) = rows(list).iter().position(|r| Some(r.item) == item) {
            self.cursor[list as usize] = i;
        }
        self.list = list;
        self.open(lcd, s, Screen::Menu);
    }

    /// Opens the top list from KEYER, on RESUME while a practice is paused.
    pub(super) fn open_menu(&mut self, lcd: &mut Lcd, s: &Settings) {
        let item = self.session.is_some().then_some(Item::Resume);
        self.open_list(lcd, s, List::Top, item);
    }

    /// Opens the list that holds this screen, with its row selected.
    pub(super) fn back(&mut self, lcd: &mut Lcd, s: &Settings) {
        let item = match (&self.session, self.screen) {
            (Some(session), screen) if session.screen() == screen => session.item(),
            (_, screen) => Item::Open(screen),
        };
        let list = ALL.into_iter().find(|&list| rows(list).iter().any(|r| r.item == item)).unwrap_or(List::Top);
        self.open_list(lcd, s, list, Some(item));
    }

    pub(super) fn menu_act(&mut self, lcd: &mut Lcd, s: &mut Settings, action: Action) {
        let rows = rows(self.list);
        let at = self.cursor[self.list as usize];
        match action {
            Action::Turn(delta) => {
                let mut shown = (0..rows.len()).filter(|&i| self.shown(s, rows[i].item));
                let next = if delta > 0 { shown.find(|&i| i > at) } else { shown.rev().find(|&i| i < at) };
                if let Some(next) = next {
                    self.cursor[self.list as usize] = next;
                    self.draw_menu_row(lcd, s, at);
                    self.draw_menu_row(lcd, s, next);
                    self.draw_menu_about(lcd);
                }
            }
            Action::Soft(0) => self.open_settings_at(lcd, s, Section::Keyer),
            Action::Click => self.menu_open(lcd, s, rows[at].item),
            Action::Soft(3) if self.list == List::Top => self.open(lcd, s, Screen::Home),
            Action::Soft(3) => self.open_list(lcd, s, List::Top, Some(Item::List(self.list))),
            _ => {}
        }
    }

    /// Opens what `item` names. A practice that is paused resumes; any other starts afresh.
    fn menu_open(&mut self, lcd: &mut Lcd, s: &Settings, item: Item) {
        if item == Item::Resume || self.session.as_ref().map(Session::item) == Some(item) {
            return self.resume(lcd, s);
        }
        match item {
            Item::Resume => {}
            Item::List(list) => self.open_list(lcd, s, list, None),
            Item::Open(Screen::Settings) => self.open_settings_at(lcd, s, Section::Keyer),
            Item::Open(screen) => self.open(lcd, s, screen),
            Item::Drill(kind) => self.open_receive(lcd, s, kind),
            Item::Chew { quiz } => self.open_ragchew(lcd, s, quiz),
            Item::Qso(program) => self.open_qso(lcd, s, program),
            Item::Hunt => self.open_hunt(lcd, s),
        }
    }
}
