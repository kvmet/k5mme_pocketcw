//! Ragchew: two stations hold a whole contact for you to copy. One calls CQ, the other answers,
//! and they trade reports, names, QTHs, rigs, antennas and weather, then sign with 73. Each over
//! is split into lines short enough for the player; the quiz asks about one fact per over.

use core::fmt::{self, Write};

use crate::player::MAX_TEXT;
use crate::qso::{CALL_LEN, us_call};
use crate::rng::Rng;
use crate::text::Text;

/// Overs in one contact: CQ, answer, two exchanges each, and two sign-offs.
pub const OVERS: usize = 8;
pub const MAX_LINES: usize = 5;

const NAMES: [&str; 30] = [
    "BOB", "JIM", "TOM", "MIKE", "DAVE", "JOHN", "BILL", "STEVE", "ED", "AL", "RON", "DON", "GARY", "KEN", "JOE", "SUE",
    "ANN", "MARY", "LISA", "KAREN", "PAT", "CHRIS", "SAM", "ALEX", "JEN", "KIM", "ROSE", "MAX", "LEE", "RAY",
];

/// Cities and their states by call area digit, at most `MAX_GROUP` letters without spaces.
const CITIES_BY_AREA: [&[(&str, &str)]; 10] = [
    &[("DENVER", "CO"), ("WICHITA", "KS"), ("OMAHA", "NE"), ("DES MOINES", "IA"), ("ST PAUL", "MN"), ("FARGO", "ND")],
    &[("BOSTON", "MA"), ("PORTLAND", "ME"), ("BURLINGTON", "VT"), ("HARTFORD", "CT"), ("CONCORD", "NH")],
    &[("ALBANY", "NY"), ("BUFFALO", "NY"), ("TRENTON", "NJ"), ("NEWARK", "NJ")],
    &[("PITTSBURGH", "PA"), ("ERIE", "PA"), ("DOVER", "DE"), ("BALTIMORE", "MD")],
    &[("RICHMOND", "VA"), ("RALEIGH", "NC"), ("ATLANTA", "GA"), ("TAMPA", "FL"), ("NASHVILLE", "TN"), ("MOBILE", "AL")],
    &[("AUSTIN", "TX"), ("EL PASO", "TX"), ("TULSA", "OK"), ("SANTA FE", "NM"), ("JACKSON", "MS")],
    &[("FRESNO", "CA"), ("SAN DIEGO", "CA"), ("OAKLAND", "CA"), ("REDDING", "CA")],
    &[("SEATTLE", "WA"), ("EUGENE", "OR"), ("BOISE", "ID"), ("RENO", "NV"), ("TUCSON", "AZ"), ("BOZEMAN", "MT")],
    &[("DETROIT", "MI"), ("TOLEDO", "OH"), ("DAYTON", "OH"), ("WHEELING", "WV")],
    &[("CHICAGO", "IL"), ("PEORIA", "IL"), ("MADISON", "WI"), ("GARY", "IN")],
];

/// Rigs and the power they run.
const RIGS: [(&str, u8); 12] = [
    ("IC7300", 100),
    ("IC705", 10),
    ("FT991A", 100),
    ("FTDX10", 100),
    ("FT710", 100),
    ("FT891", 100),
    ("K3", 100),
    ("KX2", 10),
    ("KX3", 5),
    ("TS590", 100),
    ("QCX", 5),
    ("G90", 20),
];

const ANTENNAS: [&str; 10] = ["DIPOLE", "EFHW", "VERT", "YAGI", "HEXBEAM", "G5RV", "LOOP", "WINDOM", "BEAM", "LONGWIRE"];
const WEATHER: [&str; 7] = ["SUNNY", "CLOUDY", "RAIN", "SNOW", "FOG", "WINDY", "CLEAR"];
const SNOW: &str = "SNOW";
/// Ragchew reports are honest numbers, not the contest 5NN.
const REPORTS: [&str; 9] = ["599", "589", "579", "579", "569", "559", "459", "449", "339"];

#[derive(Clone, Copy, Debug)]
pub struct Station {
    pub call: Text<CALL_LEN>,
    name: &'static str,
    city: &'static str,
    state: &'static str,
    /// The report this station gives the other.
    report: &'static str,
    rig: &'static str,
    watts: u8,
    antenna: &'static str,
    weather: &'static str,
    temp_f: u8,
    /// Timing jitter in percent, used when the human fist setting is on.
    pub fist: u8,
    pub pitch_offset_hz: i16,
}

impl Station {
    fn new(rng: &mut Rng) -> Station {
        let digit = rng.below(10) as u8;
        let cities = CITIES_BY_AREA[digit as usize];
        let (city, state) = cities[rng.below(cities.len() as u32) as usize];
        let (rig, watts) = RIGS[rng.below(RIGS.len() as u32) as usize];
        let weather = WEATHER[rng.below(WEATHER.len() as u32) as usize];
        let temp_f = if weather == SNOW { 15 + rng.below(18) } else { 40 + rng.below(56) } as u8;
        Station {
            call: us_call(digit, rng),
            name: NAMES[rng.below(NAMES.len() as u32) as usize],
            city,
            state,
            report: REPORTS[rng.below(REPORTS.len() as u32) as usize],
            rig,
            watts,
            antenna: ANTENNAS[rng.below(ANTENNAS.len() as u32) as usize],
            weather,
            temp_f,
            fist: rng.below(13) as u8,
            pitch_offset_hz: (rng.below(21) as i16 - 10) * 10,
        }
    }
}

/// One over: who sends it and its lines.
#[derive(Clone, Copy, Debug)]
pub struct Over {
    /// Index into `Ragchew::stations`.
    pub from: usize,
    lines: [Text<MAX_TEXT>; MAX_LINES],
    len: usize,
}

impl Over {
    fn new(from: usize) -> Over {
        Over { from, lines: [Text::new(); MAX_LINES], len: 0 }
    }

    fn line(&mut self, args: fmt::Arguments) {
        let _ = self.lines[self.len].write_fmt(args);
        self.len += 1;
    }

    pub fn lines(&self) -> &[Text<MAX_TEXT>] {
        &self.lines[..self.len]
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Fact {
    Call,
    Report,
    Name,
    Qth,
    Rig,
    Power,
    Antenna,
    Weather,
    Temp,
}

/// A question about the over just sent, and the answer as sent.
#[derive(Clone, Copy, Debug)]
pub struct Question {
    pub prompt: &'static str,
    pub answer: Text<16>,
}

pub struct Ragchew {
    pub stations: [Station; 2],
    /// The current over, below `OVERS`.
    over: usize,
    rng: Rng,
}

impl Ragchew {
    pub fn new(seed: u32) -> Ragchew {
        let mut rng = Rng::new(seed);
        let stations = [Station::new(&mut rng), Station::new(&mut rng)];
        Ragchew { stations, over: 0, rng }
    }

    /// The station sending the current over.
    pub fn sender(&self) -> &Station {
        &self.stations[self.over % 2]
    }

    pub fn over(&self) -> Over {
        let (me, you) = (self.sender(), &self.stations[1 - self.over % 2]);
        let (my, ur) = (me.call.as_str(), you.call.as_str());
        let mut o = Over::new(self.over % 2);
        match self.over {
            0 => o.line(format_args!("CQ CQ CQ DE {my} {my} K")),
            1 => o.line(format_args!("{ur} DE {my} {my} K")),
            2 | 3 => {
                if self.over == 2 {
                    o.line(format_args!("{ur} DE {my} TNX FER CALL"));
                } else {
                    o.line(format_args!("{ur} DE {my} R FB {}", you.name));
                }
                o.line(format_args!("UR RST {} {} =", me.report, me.report));
                o.line(format_args!("NAME {} {} =", me.name, me.name));
                o.line(format_args!("QTH {} {} =", me.city, me.state));
                o.line(format_args!("HW CPY? {ur} DE {my} <KN>"));
            }
            4 | 5 => {
                o.line(format_args!("{ur} DE {my} TNX {}", you.name));
                o.line(format_args!("RIG {} {}W =", me.rig, me.watts));
                o.line(format_args!("ANT {} =", me.antenna));
                o.line(format_args!("WX {} {}F =", me.weather, me.temp_f));
                o.line(format_args!("{ur} DE {my} <KN>"));
            }
            6 => {
                o.line(format_args!("R TNX FER QSO {}", you.name));
                o.line(format_args!("73 ES GL"));
                o.line(format_args!("{ur} DE {my} <SK>"));
            }
            _ => {
                o.line(format_args!("TU {} 73 CUL", you.name));
                o.line(format_args!("{ur} DE {my} <SK> E E"));
            }
        }
        o
    }

    /// A question about one fact in the current over; None for an over without facts.
    pub fn question(&mut self) -> Option<Question> {
        let facts: &[Fact] = match self.over {
            0 | 1 => &[Fact::Call],
            2 | 3 => &[Fact::Report, Fact::Name, Fact::Qth],
            4 | 5 => &[Fact::Rig, Fact::Power, Fact::Antenna, Fact::Weather, Fact::Temp],
            _ => &[],
        };
        let &fact = facts.get(self.rng.below(facts.len() as u32) as usize)?;
        let me = self.sender();
        let mut answer = Text::new();
        let (prompt, _) = match fact {
            Fact::Call => ("SENDER'S CALL?", write!(answer, "{}", me.call.as_str())),
            Fact::Report => ("RST?", write!(answer, "{}", me.report)),
            Fact::Name => ("NAME?", write!(answer, "{}", me.name)),
            Fact::Qth => ("QTH CITY?", write!(answer, "{}", me.city)),
            Fact::Rig => ("RIG?", write!(answer, "{}", me.rig)),
            Fact::Power => ("POWER IN WATTS?", write!(answer, "{}", me.watts)),
            Fact::Antenna => ("ANTENNA?", write!(answer, "{}", me.antenna)),
            Fact::Weather => ("WX?", write!(answer, "{}", me.weather)),
            Fact::Temp => ("TEMPERATURE?", write!(answer, "{}", me.temp_f)),
        };
        Some(Question { prompt, answer })
    }

    /// Moves to the next over; after the last, two new stations start a new contact.
    pub fn next(&mut self) {
        self.over += 1;
        if self.over == OVERS {
            self.stations = [Station::new(&mut self.rng), Station::new(&mut self.rng)];
            self.over = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    use crate::drill::{MAX_GROUP, score_copy, squash};
    use crate::letters::Letters;

    /// An answer fits drill scoring once its spaces are gone.
    fn fits_scoring(answer: &str) -> bool {
        squash::<{ MAX_GROUP + 1 }>(answer).len() <= MAX_GROUP
    }

    #[test]
    fn every_line_fits_the_player_and_ends_whole() {
        let mut chat = Ragchew::new(7);
        for _ in 0..40 * OVERS {
            let over = chat.over();
            for line in over.lines() {
                assert!(!line.is_empty() && line.len() < MAX_TEXT, "{line:?}");
            }
            // BK hands the contact back, so it can only end an over.
            let (last, before) = over.lines().split_last().unwrap();
            assert!(before.iter().all(|l| !l.as_str().contains("<BK>")), "{over:?}");
            let last = last.as_str();
            assert!(["K", "<KN>", "<SK>", "E E"].iter().any(|end| last.ends_with(end)), "{last}");
            chat.next();
        }
    }

    #[test]
    fn stations_take_turns_and_a_new_contact_follows() {
        let mut chat = Ragchew::new(3);
        let first = chat.stations[0].call;
        let senders: std::vec::Vec<usize> = (0..OVERS).map(|_| (chat.over().from, chat.next()).0).collect();
        assert_eq!(senders, [0, 1, 0, 1, 0, 1, 0, 1]);
        assert!(chat.over().lines()[0].as_str().starts_with("CQ"));
        assert_ne!(chat.stations[0].call, first);
    }

    #[test]
    fn answers_are_in_the_over_and_score_as_copied() {
        let mut chat = Ragchew::new(11);
        let mut letters = Letters::default();
        for _ in 0..20 * OVERS {
            let over = chat.over();
            if let Some(q) = chat.question() {
                assert!(over.lines().iter().any(|l| l.as_str().contains(q.answer.as_str())), "{q:?}");
                assert!(fits_scoring(q.answer.as_str()), "{q:?}");
                let score = score_copy(q.answer.as_str(), &q.answer.as_str().to_ascii_lowercase(), &mut letters);
                assert_eq!(score.right as usize, score.len);
            }
            chat.next();
        }
    }

    #[test]
    fn every_city_fits_scoring() {
        for &(city, _) in CITIES_BY_AREA.iter().flat_map(|c| c.iter()) {
            assert!(fits_scoring(city), "{city}");
        }
    }
}
