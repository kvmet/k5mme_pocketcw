//! Serial port commands the UI answers: settings and memories by name, and INFO.
//! Each reply line ends with `\n`.

use pocketcw_logic::{
    config::{self, Name},
    panel::Panel,
    settings::{Field, Settings},
};

use super::{Screen, Session, Ui, settings};
use crate::lcd::Lcd;
use crate::text::Text;
use crate::theme::Theme;

/// Room for the longest reply: an error that lists a setting's values.
pub const REPLY_LEN: usize = config::REASON_LEN + 32;
pub type Reply = Text<REPLY_LEN>;

impl Ui {
    /// `INFO VERSION=0.2.0 SCREEN=KEYER PANEL=IPS`
    pub fn info(&self) -> Reply {
        let screen = match self.screen {
            Screen::Home => "KEYER",
            Screen::Timing => "TIMING",
            Screen::Qso => "QSO",
            Screen::Hunt => "HUNT",
            Screen::Menu => "MENU",
            Screen::Settings => "SETTINGS",
            Screen::System => "SYSTEM",
            Screen::Receive => "RECEIVE",
            Screen::Ragchew => "RAGCHEW",
            Screen::Letters => "LETTERS",
            Screen::Progress => "PROGRESS",
            Screen::Radio => "RADIO",
        };
        let panel = match self.system.panel {
            Panel::Ips => "IPS",
            Panel::Ili9341 => "ILI9341",
        };
        Text::format(format_args!("INFO VERSION={} SCREEN={screen} PANEL={panel}\n", env!("CARGO_PKG_VERSION")))
    }

    /// `VALUE SPEED 25`
    pub fn get(&self, s: &Settings, name: Name) -> Reply {
        let value = config::get(name, s, &self.memories, settings::value);
        Text::format(format_args!("VALUE {} {}\n", name.as_str(), value.as_str()))
    }

    /// Sets `name` to `value` and redraws the screen. `OK SPEED 25`, or `ERROR SPEED: 5 TO 60`.
    pub fn set(&mut self, lcd: &mut Lcd, s: &mut Settings, name: Name, value: &str) -> Reply {
        if let Err(reason) = config::set(name, value, s, &mut self.memories, settings::value) {
            return Text::format(format_args!("ERROR {}: {}\n", name.as_str(), reason.as_str()));
        }
        if let Name::Setting(Field::Style | Field::Palette) = name {
            self.theme = Theme::new(s.style, s.palette, self.boot_count);
        }
        // The QSO and hunt stations take what the settings screen gives them on its way back.
        match name {
            Name::Setting(Field::RxSpeed | Field::Pileup | Field::Fist | Field::Spread) => self.bot_speed_changed(s),
            Name::Call => match &mut self.session {
                Some(Session::Qso(q)) => q.bot.set_call(s.callsign()),
                Some(Session::Hunt(h)) => h.bot.set_call(s.callsign()),
                _ => {}
            },
            _ => {}
        }
        if name == Name::Setting(Field::Speed) {
            self.speed_set(lcd, s);
        } else {
            self.draw(lcd, s);
        }
        let value = config::get(name, s, &self.memories, settings::value);
        Text::format(format_args!("OK {} {}\n", name.as_str(), value.as_str()))
    }
}
