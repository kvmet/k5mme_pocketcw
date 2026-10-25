//! Receive drills: the device sends a group at the receive speed, you key back what you copied,
//! and the group shows under your copy with each character marked.

use core::fmt::Write;

use pocketcw_logic::{
    decoder::{Decoded, copy_end},
    drill::{Drill, Kind, MAX_COPY, MAX_GROUP, Mark},
    input::Action,
    letters::{KOCH, Mastery},
    morse,
    player::{Keying, MAX_TEXT},
    settings::Settings,
};

use super::{COLS, Play, Screen, Session, Ui, char_cells, draw_char, push_scrolling, settings::RECEIVE_TAB};
use crate::fonts::{SMALL, TINY};
use crate::gfx::{CHAR_W, TINY_W, ink, rect, text_cols, tiny};
use crate::lcd::Lcd;
use crate::text::Text;
use crate::theme::{BAD, GOOD, MAIN, Theme, WARN};

const INFO_Y: i32 = 26;
const SET_Y: i32 = 40;
const SET_PER_ROW: usize = 38;
/// One more than TINY's height, so a prosign bar on the second row clears the first row.
const SET_STEP: i32 = 13;
const RULE_Y: i32 = 66;
const ROWS: usize = 7;
const ROW_Y: i32 = 70;
const ROW_STEP: i32 = 18;
const COPY_X: i32 = 110;
const NOW_Y: i32 = ROW_Y + ROWS as i32 * ROW_STEP + 4;
const FOOT_Y: i32 = 226;
/// Time after a group ends to start keying it back before it counts as missed.
const START_MS: u32 = 5000;
const NEXT_GROUP_MS: u32 = 900;
const GROUP_DELAY_MS: u32 = 400;

#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    Playing,
    /// The group ended; waiting for you to start keying.
    Waiting,
    Copying,
    /// Scored; the next group starts at the deadline.
    Between,
    Paused,
}

#[derive(Clone, Copy)]
struct Row {
    sent: Text<MAX_GROUP>,
    marks: [Mark; MAX_GROUP],
    right: u8,
}

pub(super) struct Receive {
    drill: Drill,
    group: Text<MAX_GROUP>,
    copy: Text<MAX_COPY>,
    state: State,
    pub(super) deadline: Option<u32>,
    rows: [Option<Row>; ROWS],
    session_right: u16,
    session_total: u16,
}

impl Receive {
    pub(super) fn new(kind: Kind, level: u8, seed: u32) -> Receive {
        Receive {
            drill: Drill::new(kind, level, seed),
            group: Text::new(),
            copy: Text::new(),
            state: State::Between,
            deadline: None,
            rows: [None; ROWS],
            session_right: 0,
            session_total: 0,
        }
    }

    /// Groups are playing or waiting on you, not paused.
    pub(super) fn running(&self) -> bool {
        self.state != State::Paused
    }
}

impl Ui {
    pub(super) fn draw_receive(&mut self, lcd: &mut Lcd, s: &Settings) {
        let Some(Session::Receive(rx)) = &self.session else { return };
        let t = &self.theme;
        t.header(lcd, "RECEIVE", &[rx.drill.kind.name(), speed_label(s).as_str()]);
        let pause = if rx.state == State::Paused { "GO" } else { "STOP" };
        t.softkeys(lcd, ["SET", "KIND", pause, "BACK"]);
        rect(lcd, 0, RULE_Y, MAIN, 1, t.rule());
        self.draw_receive_info(lcd);
        self.draw_receive_rows(lcd);
        self.draw_receive_now(lcd);
    }

    /// Koch level and progress, and the character set colored by your accuracy.
    fn draw_receive_info(&self, lcd: &mut Lcd) {
        let Some(Session::Receive(rx)) = &self.session else { return };
        let t = &self.theme;
        rect(lcd, 0, INFO_Y, MAIN, RULE_Y - INFO_Y, t.bg);
        let mut info = Text::<40>::new();
        let drill = &rx.drill;
        let set = match drill.kind {
            Kind::Koch => {
                let (right, total) = drill.window();
                let percent = if total > 0 { right as u32 * 100 / total as u32 } else { 0 };
                let _ = write!(info, "LEVEL {}  {}% OF {}  NEED 90% OF 50", drill.level, percent, total);
                &KOCH[..drill.level as usize]
            }
            _ => {
                let _ = write!(info, "{} DRILL  KEY BACK WHAT YOU HEAR", drill.kind.name());
                &KOCH[..]
            }
        };
        tiny(lcd, 2, INFO_Y, info.as_str(), t.muted, t.bg);
        let mut cell = 0;
        for (i, &c) in set.iter().enumerate() {
            let cells = char_cells(c);
            if cell % SET_PER_ROW + cells > SET_PER_ROW {
                cell += SET_PER_ROW - cell % SET_PER_ROW;
            }
            let x = 2 + (cell % SET_PER_ROW) as i32 * TINY_W;
            let y = SET_Y + (cell / SET_PER_ROW) as i32 * SET_STEP;
            cell += cells;
            // Weak characters show inverted, so they stand out without color.
            let (fg, bg) = match self.letters.mastery(pocketcw_logic::letters::index(c).unwrap_or(0)) {
                None => (t.muted, t.bg),
                Some(Mastery::Known) => (GOOD, t.bg),
                Some(Mastery::Unsure) => (WARN, t.bg),
                Some(Mastery::Weak) => (ink(BAD), BAD),
            };
            let w = draw_char(lcd, &TINY, x, y, c, fg, bg);
            if drill.kind == Kind::Koch && i + 1 == set.len() {
                rect(lcd, x, y + 11, w - 1, 1, t.accent);
            }
        }
    }

    fn draw_receive_rows(&self, lcd: &mut Lcd) {
        let Some(Session::Receive(rx)) = &self.session else { return };
        let t = &self.theme;
        for (k, row) in rx.rows.iter().enumerate() {
            let y = ROW_Y + k as i32 * ROW_STEP;
            rect(lcd, 0, y, MAIN, ROW_STEP, t.bg);
            let Some(row) = row else { continue };
            draw_marked(lcd, t, y, COPY_X, row.sent.as_str().bytes(), &row.marks, row.right);
        }
    }

    /// Your copy so far, what the drill waits for, and the session score.
    fn draw_receive_now(&self, lcd: &mut Lcd) {
        let Some(Session::Receive(rx)) = &self.session else { return };
        let t = &self.theme;
        let copy = rx.copy.as_str();
        let shown = &copy[copy.len().saturating_sub(COLS - 8)..];
        text_cols(lcd, 0, NOW_Y, shown, COLS - 7, t.fg, t.bg);
        if rx.state == State::Waiting || rx.state == State::Copying {
            rect(lcd, shown.len() as i32 * CHAR_W + 1, NOW_Y + 2, CHAR_W - 2, 13, t.accent);
        }
        let status = match rx.state {
            State::Playing => "LISTEN",
            State::Waiting => "YOUR TURN",
            State::Copying => "PAUSE ENDS",
            State::Between => "",
            State::Paused => "PAUSED",
        };
        rect(lcd, MAIN - 72, NOW_Y + 3, 72, 12, t.bg);
        tiny(lcd, MAIN - 2 - status.len() as i32 * TINY_W, NOW_Y + 3, status, t.echo, t.bg);
        draw_session(lcd, t, rx.session_right, rx.session_total, "KEY BACK WHAT YOU HEARD  ? REPEATS");
    }

    /// Starts the next group, or the same one again with `again`.
    fn play_group(&mut self, lcd: &mut Lcd, s: &Settings, again: bool) {
        let seed = self.next_seed();
        let Some(Session::Receive(rx)) = &mut self.session else { return };
        if !again {
            rx.group = rx.drill.next(&self.letters);
        }
        rx.copy.clear();
        rx.state = State::Playing;
        rx.deadline = None;
        let (wpm, effective) = s.receive_speed(s.wpm);
        let text = Text::<MAX_TEXT>::format(format_args!("{}", rx.group.as_str()));
        let keying = Keying { wpm, effective, fist: 0 };
        self.play = Some(Play { text, keying, seed, delay_ms: GROUP_DELAY_MS, pitch_hz: s.tone_hz, transmit: false });
        self.draw_receive_now(lcd);
    }

    pub(super) fn open_receive(&mut self, lcd: &mut Lcd, s: &Settings, kind: Kind) {
        let seed = self.next_seed();
        let mut rx = Receive::new(kind, self.letters.level, seed);
        if self.koch_window.0 == self.letters.level {
            rx.drill.resume(self.koch_window.1);
        }
        self.session = Some(Session::Receive(rx));
        self.open(lcd, s, Screen::Receive);
        self.play_group(lcd, s, false);
    }

    /// Back from settings: the next group plays at the new speed.
    pub(super) fn receive_resume(&mut self, lcd: &mut Lcd, s: &Settings) {
        self.open(lcd, s, Screen::Receive);
        self.play_group(lcd, s, false);
    }

    pub(super) fn receive_played(&mut self, lcd: &mut Lcd, now_ms: u32) {
        let Some(Session::Receive(rx)) = &mut self.session else { return };
        if rx.state == State::Playing {
            rx.state = State::Waiting;
            rx.deadline = Some(now_ms.wrapping_add(START_MS));
            self.draw_receive_now(lcd);
        }
    }

    pub(super) fn receive_key(&mut self, lcd: &mut Lcd, down: bool, at_ms: u32, dit_ms: u16) {
        let Some(Session::Receive(rx)) = &mut self.session else { return };
        if matches!(rx.state, State::Between | State::Paused) {
            return;
        }
        let was = rx.state;
        if down {
            if rx.state == State::Waiting {
                rx.state = State::Copying;
            }
            if rx.state == State::Copying {
                rx.deadline = None;
            }
        } else if rx.state == State::Copying {
            rx.deadline = Some(copy_end(at_ms, dit_ms));
        }
        if rx.state != was {
            self.draw_receive_now(lcd);
        }
    }

    pub(super) fn receive_decoded(&mut self, lcd: &mut Lcd, decoded: Decoded) {
        let Some(Session::Receive(rx)) = &mut self.session else { return };
        if !matches!(rx.state, State::Copying | State::Waiting | State::Playing) {
            return;
        }
        if decoded.write_to(&mut rx.copy) {
            self.draw_receive_now(lcd);
        }
    }

    /// The copy ended or timed out: score it. Or the pause between groups ended: play the next.
    pub(super) fn receive_timeout(&mut self, lcd: &mut Lcd, s: &Settings, now_ms: u32) {
        let Some(Session::Receive(rx)) = &mut self.session else { return };
        rx.deadline = None;
        if rx.state == State::Between {
            return self.play_group(lcd, s, false);
        }
        if !matches!(rx.state, State::Waiting | State::Copying) {
            return;
        }
        if rx.copy.as_str().trim() == "?" {
            return self.play_group(lcd, s, true);
        }
        let score = rx.drill.score(rx.group.as_str(), rx.copy.as_str(), &mut self.letters);
        self.letters.level = rx.drill.level;
        self.koch_window = (rx.drill.level, rx.drill.window());
        self.history.copied(score.right, score.len as u8);
        rx.session_right += score.right as u16;
        rx.session_total += score.len as u16;
        push_scrolling(&mut rx.rows, Row { sent: rx.group, marks: score.marks, right: score.right });
        rx.copy.clear();
        rx.state = State::Between;
        rx.deadline = Some(now_ms.wrapping_add(NEXT_GROUP_MS));
        self.draw_receive_rows(lcd);
        self.draw_receive_now(lcd);
        if score.level_up {
            self.draw_receive_info(lcd);
            let mut line = Text::<COLS>::new();
            if let Some(Session::Receive(rx)) = &self.session {
                let _ = write!(line, "LEVEL UP: ADDED {}", morse::text(&KOCH[rx.drill.level as usize - 1]));
            }
            self.status(lcd, line.as_str());
        } else if let Some(Session::Receive(rx)) = &self.session
            && rx.drill.kind == Kind::Koch
        {
            self.draw_receive_info(lcd);
        }
    }

    pub(super) fn receive_act(&mut self, lcd: &mut Lcd, s: &mut Settings, action: Action) {
        let Some(Session::Receive(rx)) = &mut self.session else { return };
        match action {
            Action::Turn(delta) if rx.drill.kind == Kind::Koch => {
                rx.drill.set_level((rx.drill.level as i32 + delta as i32).clamp(0, u8::MAX as i32) as u8);
                self.letters.level = rx.drill.level;
                self.koch_window = (rx.drill.level, rx.drill.window());
                self.draw_receive_info(lcd);
            }
            Action::Soft(1) => {
                let kind = rx.drill.kind;
                self.stop_players();
                match kind {
                    Kind::Calls => self.open_ragchew(lcd, s, false),
                    kind => self.open_receive(lcd, s, kind.next()),
                }
            }
            Action::Soft(0) => {
                rx.state = State::Paused;
                rx.deadline = None;
                rx.copy.clear();
                self.open_settings_at(lcd, s, RECEIVE_TAB);
            }
            Action::Soft(2) => {
                if rx.state == State::Paused {
                    self.play_group(lcd, s, false);
                } else {
                    rx.state = State::Paused;
                    rx.deadline = None;
                    rx.copy.clear();
                    self.stop_players();
                }
                self.draw(lcd, s);
            }
            Action::Soft(3) => self.open(lcd, s, Screen::Menu),
            _ => {}
        }
    }
}

/// The receive speed for a header: the character speed, then the Farnsworth speed when slower.
pub(super) fn speed_label(s: &Settings) -> Text<10> {
    let (wpm, effective) = s.receive_speed(s.wpm);
    if effective < wpm { Text::format(format_args!("{wpm}/{effective}WPM")) } else { Text::format(format_args!("{wpm}WPM")) }
}

/// One scored row at `y`: what was sent, your copy marked against it from `copy_x`, and the
/// score at the right end. Errors show inverted, so they stand out without color.
pub(super) fn draw_marked(lcd: &mut Lcd, t: &Theme, y: i32, copy_x: i32, sent: impl Iterator<Item = u8> + Clone, marks: &[Mark], right: u8) {
    let len = sent.clone().count();
    let mut x = 4;
    for c in sent.clone() {
        x += draw_char(lcd, &SMALL, x, y, c, t.muted, t.bg);
    }
    let mut x = copy_x;
    for (mark, sent_char) in marks.iter().zip(sent) {
        let (c, fg, bg) = match *mark {
            Mark::Right => (sent_char, GOOD, t.bg),
            Mark::Wrong(c) => (c, ink(BAD), BAD),
            Mark::Missed => (b'_', ink(BAD), BAD),
        };
        x += draw_char(lcd, &SMALL, x, y, c, fg, bg);
    }
    let score = Text::<6>::format(format_args!("{right}/{len}"));
    tiny(lcd, MAIN - 6 - score.len() as i32 * TINY_W, y + 3, score.as_str(), t.muted, t.bg);
}

/// The footer: the session's percent right, or `hint` before anything is scored.
pub(super) fn draw_session(lcd: &mut Lcd, t: &Theme, right: u16, total: u16, hint: &str) {
    let foot = if total > 0 {
        Text::<40>::format(format_args!("SESSION {}%  {} CHARS", right as u32 * 100 / total as u32, total))
    } else {
        Text::format(format_args!("{hint}"))
    };
    rect(lcd, 0, FOOT_Y, MAIN, 12, t.bg);
    tiny(lcd, 2, FOOT_Y, foot.as_str(), t.muted, t.bg);
}
