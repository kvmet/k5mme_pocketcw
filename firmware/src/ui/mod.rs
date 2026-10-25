//! Screens: the keyer, timing, QSO and hunt practice, receive drills and ragchews, letters,
//! progress, the menu, settings and system. Each event redraws only what it changed; a full
//! redraw takes about 70 ms.

mod dial;
mod exchange;
mod home;
mod hunt;
mod letters;
mod memory;
mod menu;
mod progress;
mod qso;
mod radio;
mod ragchew;
mod receive;
mod serial;
mod settings;
mod system;
mod tape;
mod timing;
mod transcript;

use embedded_graphics::{pixelcolor::Rgb565, prelude::Point};
use pocketcw_logic::{
    battery::Gauge,
    decoder::Decoded,
    dial::{KeyGuard, Target},
    input::Action,
    morse,
    letters::{Letters, RECORD_LEN as LETTERS_LEN},
    memories::{Memories, RECORD_LEN as MEMORIES_LEN},
    progress::{History, SLOT_LEN},
    keyer::{self, Mode},
    player::{Keying, MAX_TEXT},
    qso::{Options, Program},
    settings::{Field, Pileup, Settings},
    ticks,
    timing::{Interval, Line, Meter, PAUSE_UNITS, Stats},
};

use crate::font::Font;
use crate::gfx::{rect, text_cols};
use crate::lcd::Lcd;
use crate::text::Text;
use crate::theme::{Battery, MAIN, Theme};

use home::Decode;
use hunt::Hunting;
use qso::Qso;
use ragchew::Chew;
use receive::Receive;
use tape::Tape;

pub use serial::Reply;
pub use system::{PanelFrom, SystemInfo};

const COLS: usize = (MAIN / crate::gfx::CHAR_W) as usize;
const STATUS_Y: i32 = 222;
/// When each contact, tip then ring, stops keying.
const STUCK_MS: [u32; 2] = [keyer::TIP_STUCK_MS as u32, keyer::RING_STUCK_MS as u32];

const SCREENS: usize = 12;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Screen {
    Home,
    Timing,
    Qso,
    Hunt,
    Menu,
    Settings,
    System,
    Receive,
    Ragchew,
    Letters,
    Progress,
    Radio,
}
const _: () = assert!(Screen::Radio as usize + 1 == SCREENS);

/// The practice running on its screen, or on settings opened from it.
// Without a heap the large variant can't be boxed; one union still costs less than four fields.
#[allow(clippy::large_enum_variant)]
enum Session {
    Qso(Qso),
    Hunt(Hunting),
    Receive(Receive),
    Ragchew(Chew),
}

impl Session {
    fn screen(&self) -> Screen {
        match self {
            Session::Qso(_) => Screen::Qso,
            Session::Hunt(_) => Screen::Hunt,
            Session::Receive(_) => Screen::Receive,
            Session::Ragchew(_) => Screen::Ragchew,
        }
    }
}

/// Something for the tone player to do.
pub enum Command {
    Play(Play),
    Stop,
}

/// A transmission for the player, with everything it needs.
pub struct Play {
    pub text: Text<MAX_TEXT>,
    pub keying: Keying,
    pub seed: u32,
    pub delay_ms: u32,
    pub pitch_hz: u16,
    /// Key the radio too, as a memory keyer; touching the key stops it.
    pub transmit: bool,
}

pub struct Ui {
    theme: Theme,
    boot_count: u16,
    screen: Screen,
    /// Each screen's focused dial target, once a click has moved it.
    dial_focus: [Option<Target>; SCREENS],
    key_guard: KeyGuard,
    decode: Decode,
    menu: usize,
    tab: usize,
    /// Selected settings row.
    row: usize,
    /// The dial sets the selected row's value instead of moving the selection.
    editing: bool,
    meter: Meter,
    stats: Stats,
    tape: Tape,
    /// When each contact closed, while it is closed: tip, ring.
    contact_since: [Option<u32>; 2],
    /// The stuck warning is showing for this contact.
    stuck_shown: [bool; 2],
    session: Option<Session>,
    letters: Letters,
    /// `letters` as last saved.
    letters_saved: Letters,
    /// The Koch drill's window, kept while you leave the drill, and the level it counts toward.
    koch_window: (u8, (u16, u16)),
    history: History,
    memories: Memories,
    /// `memories` as last saved.
    memories_saved: Memories,
    /// The MEM row is open on Home: F1..F4 send M1..M4.
    mem_row: bool,
    sending: Option<memory::Sending>,
    /// CLR on the Letters screen was pressed once.
    confirm_clear: bool,
    /// Every player stops before anything else plays.
    stop: bool,
    /// A transmission to start, after any stop.
    play: Option<Play>,
    /// A PLAY from the serial port is sounding.
    serial_play: bool,
    seed: u32,
    system: SystemInfo,
    /// None when the ADC did not answer.
    battery: Option<crate::battery::Reading>,
    gauge: Gauge,
    /// What the header's battery icon shows now.
    battery_shown: Option<Battery>,
    /// Tick count at the last poll.
    now_ms: u32,
    /// The practice screen whose SET key opened settings; BACK returns to it.
    settings_from: Option<Screen>,
    heard: radio::Heard,
    board: radio::Board,
    /// A command for the decoder board, not sent yet.
    board_command: Option<pocketcw_logic::board::Command>,
}

impl Ui {
    pub fn new(settings: &Settings, system: SystemInfo, letters: Letters, history: History, memories: Memories, gauge: Gauge) -> Ui {
        Ui {
            theme: Theme::new(settings.style, settings.palette, settings.boot_count),
            boot_count: settings.boot_count,
            screen: Screen::Home,
            dial_focus: [None; SCREENS],
            key_guard: KeyGuard::default(),
            decode: Decode::new(),
            menu: 0,
            tab: 0,
            row: 0,
            editing: false,
            meter: Meter::default(),
            stats: Stats::default(),
            tape: Tape::new(false),
            contact_since: [None; 2],
            stuck_shown: [false; 2],
            session: None,
            letters,
            letters_saved: letters,
            koch_window: (letters.level, (0, 0)),
            history,
            memories,
            memories_saved: memories,
            mem_row: false,
            sending: None,
            confirm_clear: false,
            stop: false,
            play: None,
            serial_play: false,
            seed: settings.boot_count as u32 * 7919 + 1,
            system,
            battery: None,
            gauge,
            battery_shown: None,
            now_ms: 0,
            settings_from: None,
            heard: radio::Heard::new(),
            board: radio::Board::default(),
            board_command: None,
        }
    }

    /// Redraws the whole current screen.
    pub fn draw(&mut self, lcd: &mut Lcd, s: &Settings) {
        self.theme.frame(lcd);
        match self.screen {
            Screen::Home => self.draw_home(lcd, s),
            Screen::Timing => self.draw_timing(lcd, s),
            Screen::Qso => self.draw_qso(lcd, s),
            Screen::Hunt => self.draw_hunt(lcd, s),
            Screen::Menu => self.draw_menu(lcd, s),
            Screen::Settings => self.draw_settings(lcd, s),
            Screen::System => self.draw_system(lcd, s),
            Screen::Receive => self.draw_receive(lcd, s),
            Screen::Ragchew => self.draw_ragchew(lcd, s),
            Screen::Letters => self.draw_letters(lcd, s),
            Screen::Progress => self.draw_progress(lcd, s),
            Screen::Radio => self.draw_radio(lcd, s),
        }
        self.battery_shown = self.battery_icon();
        self.theme.battery(lcd, self.battery_shown);
    }

    /// Runs `action` on the current screen at `now_ms`. Changes land in `s`; the caller applies
    /// them.
    pub fn act(&mut self, lcd: &mut Lcd, s: &mut Settings, action: Action, now_ms: u32) {
        if self.dial_act(lcd, s, action, now_ms) {
            return;
        }
        match self.screen {
            Screen::Home => self.home_act(lcd, s, action, now_ms),
            Screen::Timing => self.timing_act(lcd, s, action),
            Screen::Qso => self.qso_act(lcd, s, action),
            Screen::Hunt => self.hunt_act(lcd, s, action),
            Screen::Menu => self.menu_act(lcd, s, action),
            Screen::Settings => self.settings_act(lcd, s, action),
            Screen::System => self.system_act(lcd, s, action),
            Screen::Receive => self.receive_act(lcd, s, action),
            Screen::Ragchew => self.ragchew_act(lcd, s, action),
            Screen::Letters => self.letters_act(lcd, s, action),
            Screen::Progress => self.progress_act(lcd, s, action),
            Screen::Radio => self.radio_act(lcd, s, action),
        }
    }

    /// Shows a keyer speed set from outside the screen, on the screens that show it.
    pub fn speed_set(&mut self, lcd: &mut Lcd, s: &Settings) {
        match self.screen {
            Screen::Home => self.draw_home_value(lcd, s, Field::Speed),
            Screen::Settings => self.draw_settings(lcd, s),
            _ => {
                if s.rx_wpm == 0 {
                    self.bot_speed_changed(s);
                }
                self.draw_dial_chips(lcd, s);
            }
        }
    }

    /// The bots' base speed changed: their stations move to it, each keeping its own offset.
    fn bot_speed_changed(&mut self, s: &Settings) {
        let options = Self::qso_options(s);
        match &mut self.session {
            Some(Session::Qso(q)) => q.bot.set_options(options, s.wpm),
            Some(Session::Hunt(h)) => h.bot.set_options(options, s.wpm),
            _ => {}
        }
    }

    /// One line of muted text across the bottom of the main area, until the next redraw.
    pub fn status(&self, lcd: &mut Lcd, line: &str) {
        text_cols(lcd, 0, STATUS_Y, line, COLS, self.theme.muted, self.theme.bg);
    }

    /// A character read from the paddles.
    pub fn decoded(&mut self, lcd: &mut Lcd, s: &mut Settings, decoded: Decoded) {
        let worst = self.meter.char_worst();
        let span = self.meter.char_span();
        self.decode.push(decoded, worst);
        if let Decoded::Char(_) = decoded {
            self.history.sent();
        }
        match self.screen {
            Screen::Home => {
                // The MEM row covers the decode lines; closing it draws them all.
                if !self.mem_row {
                    self.draw_decode_push(lcd);
                }
                if let Decoded::Char(c) = decoded {
                    self.tape.letter(lcd, &self.theme, c, span);
                }
                self.draw_home_stats(lcd);
            }
            Screen::Timing => {
                if let Decoded::Char(c) = decoded {
                    self.tape.letter(lcd, &self.theme, c, span);
                }
                self.draw_timing_stats(lcd);
            }
            Screen::Qso => self.qso_decoded(lcd, decoded),
            Screen::Hunt => self.hunt_decoded(lcd, decoded),
            Screen::Settings => self.settings_decoded(lcd, s, decoded),
            Screen::Receive => self.receive_decoded(lcd, decoded),
            Screen::Ragchew => self.ragchew_decoded(lcd, s, decoded),
            Screen::Menu | Screen::System | Screen::Letters | Screen::Progress | Screen::Radio => {}
        }
    }

    /// The key or a contact went down or up at `at_ms` on the millisecond tick count, while
    /// decoding used dots of `dit`.
    pub fn edge(&mut self, lcd: &mut Lcd, s: &Settings, line: Line, down: bool, at_ms: u32, dit: u16) {
        let on_tape = matches!(self.screen, Screen::Home | Screen::Timing);
        match line {
            Line::Key => {
                self.key_guard.edge(down, at_ms, dit);
                if down {
                    self.history.active(at_ms);
                }
                if down && on_tape {
                    let paused = self.meter_paused(at_ms, dit);
                    self.tape.key_down(lcd, &self.theme, at_ms, dit, paused, lanes(s));
                }
                if let Some(iv) = self.meter.edge(down, at_ms, dit) {
                    self.stats.add(&iv);
                    if on_tape {
                        self.tape_interval(lcd, s, &iv);
                    }
                }
                match self.screen {
                    Screen::Qso => self.qso_key(down, at_ms, dit),
                    Screen::Hunt => self.hunt_key(down, at_ms, dit),
                    Screen::Receive => self.receive_key(lcd, down, at_ms, dit),
                    Screen::Ragchew => self.ragchew_key(lcd, s, down, at_ms, dit),
                    _ => {}
                }
            }
            Line::Tip | Line::Ring => {
                let i = (line == Line::Ring) as usize;
                if down && self.screen == Screen::Home {
                    self.stop_sending(lcd);
                    if self.mem_row && !self.memories.stay_open {
                        self.close_mem_row(lcd);
                    }
                }
                if self.stuck_shown[i] {
                    self.stuck_shown[i] = false;
                    self.status(lcd, "");
                }
                if down {
                    self.contact_since[i] = Some(at_ms);
                } else if let Some(since) = self.contact_since[i].take()
                    && self.screen == Screen::Home
                {
                    let lane = if s.mode.dit_on_ring(s.swap) { 1 - i } else { i };
                    self.tape.contact(lcd, &self.theme, lane, since, at_ms, dit, lanes(s));
                }
            }
        }
    }

    fn meter_paused(&self, at_ms: u32, dit: u16) -> bool {
        let (_, last_end) = self.meter.char_span();
        at_ms.wrapping_sub(last_end) as f32 >= PAUSE_UNITS * dit as f32
    }

    fn tape_interval(&mut self, lcd: &mut Lcd, s: &Settings, iv: &Interval) {
        let iambic = s.mode != Mode::Straight;
        if iv.kind.is_mark() {
            self.tape.mark(lcd, &self.theme, iv, iambic, lanes(s));
        } else {
            self.tape.gap(lcd, &self.theme, iv);
        }
    }

    /// The player finished a transmission at `now_ms`.
    pub fn played(&mut self, lcd: &mut Lcd, s: &Settings, now_ms: u32) {
        if core::mem::take(&mut self.serial_play) {
            return;
        }
        match self.screen {
            Screen::Qso => self.qso_played(),
            Screen::Hunt => self.hunt_played(now_ms),
            Screen::Receive => self.receive_played(lcd, now_ms),
            Screen::Ragchew => self.ragchew_played(lcd, s, now_ms),
            Screen::Home => self.memory_played(lcd, now_ms),
            _ => {}
        }
    }

    /// The next thing the players should do: a stop first, so a screen's old transmission never
    /// plays on beside its new one.
    pub fn take_command(&mut self, s: &Settings) -> Option<Command> {
        if core::mem::take(&mut self.stop) {
            return Some(Command::Stop);
        }
        if let Some(play) = self.play.take() {
            return Some(Command::Play(play));
        }
        self.qso_next_play(s).or_else(|| self.hunt_next_play(s)).map(Command::Play)
    }

    /// Stops every player, and drops a transmission not yet started.
    fn stop_players(&mut self) {
        self.stop = true;
        self.play = None;
    }

    /// Band noise plays: practice while it runs, the SOUND tab, so you hear the noise level you
    /// set, and a PLAY from the serial port.
    pub fn band(&self) -> bool {
        let screen = match (self.screen, &self.session) {
            (Screen::Qso | Screen::Hunt, _) => true,
            (Screen::Receive, Some(Session::Receive(rx))) => rx.running(),
            (Screen::Ragchew, Some(Session::Ragchew(c))) => c.running(),
            (Screen::Settings, _) => self.tab == settings::SOUND_TAB,
            _ => false,
        };
        screen || self.serial_play
    }

    /// Tick count at which `timeout` wants to run.
    pub fn deadline(&self) -> Option<u32> {
        let stuck = (0..2)
            .filter(|&i| !self.stuck_shown[i])
            .filter_map(|i| Some(self.contact_since[i]?.wrapping_add(STUCK_MS[i])))
            .fold(None, earliest);
        [stuck, self.screen_deadline()].into_iter().flatten().fold(None, earliest)
    }

    fn screen_deadline(&self) -> Option<u32> {
        match (self.screen, &self.session) {
            (Screen::Qso, Some(Session::Qso(q))) => q.over_deadline,
            (Screen::Hunt, Some(Session::Hunt(h))) => h.over_deadline.or(h.cq_deadline),
            (Screen::Receive, Some(Session::Receive(rx))) => rx.deadline,
            (Screen::Ragchew, Some(Session::Ragchew(c))) => c.deadline,
            (Screen::Home, _) => self.sending?.again_at,
            _ => None,
        }
    }

    pub fn timeout(&mut self, lcd: &mut Lcd, s: &Settings, now_ms: u32) {
        let passed = |at: u32| ticks::reached(now_ms, at);
        for (i, stuck_ms) in STUCK_MS.into_iter().enumerate() {
            if let Some(since) = self.contact_since[i]
                && !self.stuck_shown[i]
                && passed(since.wrapping_add(stuck_ms))
            {
                self.stuck_shown[i] = true;
                let lane = if s.mode.dit_on_ring(s.swap) { 1 - i } else { i };
                let line = Text::<COLS>::format(format_args!("{} STUCK ON: CHECK PLUG", lanes(s)[1 + lane]));
                self.status(lcd, line.as_str());
            }
        }
        if !self.screen_deadline().is_some_and(passed) {
            return;
        }
        match self.screen {
            Screen::Qso => self.qso_over_done(lcd, s),
            Screen::Hunt => self.hunt_timeout(lcd, s, now_ms),
            Screen::Receive => self.receive_timeout(lcd, s, now_ms),
            Screen::Ragchew => self.ragchew_timeout(lcd, s, now_ms),
            Screen::Home => self.memory_timeout(lcd, s),
            _ => {}
        }
    }

    /// The letter statistics record when it has changed and should be saved now: after you
    /// leave a drill, or at any time with `now`.
    pub fn take_letters(&mut self, now: bool) -> Option<[u8; LETTERS_LEN]> {
        let due = self.letters != self.letters_saved && (now || !matches!(self.screen, Screen::Receive | Screen::Ragchew));
        if due {
            self.letters_saved = self.letters;
        }
        due.then(|| self.letters.encode())
    }

    /// The memories record when it has changed and you have left settings, or at any time with
    /// `now`.
    pub fn take_memories(&mut self, now: bool) -> Option<[u8; MEMORIES_LEN]> {
        let due = self.memories != self.memories_saved && (now || self.screen != Screen::Settings);
        if due {
            self.memories_saved = self.memories;
        }
        due.then(|| self.memories.encode())
    }

    /// This session's history slot and bytes when due for saving; see `History::take_save`.
    pub fn take_history(&mut self, now_ms: u32, force: bool) -> Option<(usize, [u8; SLOT_LEN])> {
        self.history.take_save(now_ms, force)
    }

    fn open(&mut self, lcd: &mut Lcd, s: &Settings, screen: Screen) {
        if matches!(self.screen, Screen::Qso | Screen::Hunt | Screen::Receive | Screen::Ragchew) && screen != self.screen {
            self.stop_players();
        }
        // A practice session stays alive while its settings are open.
        if screen != Screen::Settings && self.session.as_ref().map(Session::screen) != Some(screen) {
            self.session = None;
        }
        if self.sending.is_some() {
            self.sending = None;
            self.stop_players();
        }
        self.mem_row = false;
        self.confirm_clear = false;
        self.screen = screen;
        self.row = 0;
        self.editing = false;
        self.tape = Tape::new(screen == Screen::Timing);
        self.draw(lcd, s);
    }

    /// Opens settings at `tab`; BACK returns to this screen, with a practice session still
    /// running.
    fn open_settings_at(&mut self, lcd: &mut Lcd, s: &Settings, tab: usize) {
        self.settings_from = Some(self.screen);
        self.tab = tab;
        self.open(lcd, s, Screen::Settings);
    }

    fn close_settings(&mut self, lcd: &mut Lcd, s: &Settings) {
        match self.settings_from.take() {
            Some(Screen::Qso) => self.qso_resume(lcd, s),
            Some(Screen::Hunt) => self.hunt_resume(lcd, s),
            Some(Screen::Receive) => self.receive_resume(lcd, s),
            Some(Screen::Ragchew) => self.ragchew_resume(lcd, s),
            Some(Screen::Settings) | None => self.open(lcd, s, Screen::Menu),
            Some(screen) => self.open(lcd, s, screen),
        }
    }

    fn qso_options(s: &Settings) -> Options {
        Options { wpm: s.rx_wpm, pileup: s.pileup != Pileup::Off, spread: s.spread, human_fist: s.human_fist }
    }

    fn next_seed(&mut self) -> u32 {
        self.seed = self.seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.seed
    }

    fn open_qso(&mut self, lcd: &mut Lcd, s: &Settings, program: Program) {
        let seed = self.next_seed();
        self.session = Some(Session::Qso(Qso::new(program, s.callsign(), seed, Self::qso_options(s))));
        self.open(lcd, s, Screen::Qso);
    }
}

/// Adds `item` in the first free slot, or drops the oldest to make room at the end.
fn push_scrolling<T: Copy, const N: usize>(slots: &mut [Option<T>; N], item: T) {
    match slots.iter_mut().find(|slot| slot.is_none()) {
        Some(free) => *free = Some(item),
        None => {
            slots.copy_within(1.., 0);
            slots[N - 1] = Some(item);
        }
    }
}

fn earliest(a: Option<u32>, b: u32) -> Option<u32> {
    Some(a.map_or(b, |a| ticks::earlier(a, b)))
}

fn dit_ms(s: &Settings) -> u16 {
    keyer::dit_ms(s.wpm)
}

/// Tape lane labels: the keyer output and the two contacts.
fn lanes(s: &Settings) -> [&'static str; 3] {
    if s.mode == Mode::Straight { ["KEY", "TIP", "RNG"] } else { ["KEY", "DIT", "DAH"] }
}

fn mode_name(mode: Mode) -> &'static str {
    match mode {
        Mode::IambicA => "IAMBIC A",
        Mode::IambicB => "IAMBIC B",
        Mode::Straight => "STRAIGHT",
    }
}

/// Cells a character takes: two for a prosign, one for anything else.
pub(super) fn char_cells(c: u8) -> usize {
    if morse::prosign_letters(c).is_some() { 2 } else { 1 }
}

/// Character `c` in `font`, with a prosign as its two letters under a bar. Returns the width.
pub(super) fn draw_char(lcd: &mut Lcd, font: &Font, x: i32, y: i32, c: u8, fg: Rgb565, bg: Rgb565) -> i32 {
    let w = font.width as i32;
    let Some(letters) = morse::prosign_letters(c) else {
        let _ = font.draw(lcd, Point::new(x, y), morse::text(&c), fg, bg);
        return w;
    };
    let _ = font.draw(lcd, Point::new(x, y), letters, fg, bg);
    // Two rows above the capitals, which start font.height / 8 rows down: 2 in SMALL, 1 in TINY.
    rect(lcd, x + 1, y + font.height as i32 / 8 - 2, 2 * w - 2, 1, fg);
    2 * w
}
