//! Receive drills: the device sends a group, you key back what you copied, and each character
//! is scored against what was sent.

use core::fmt::Write;

use crate::letters::{self, FIRST_LEVEL, KOCH, Letters};
use crate::morse;
use crate::qso::us_call;
use crate::rng::Rng;
use crate::text::Text;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    /// Five-character groups from the first `level` Koch characters.
    Koch,
    Words,
    Calls,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Koch => "KOCH",
            Kind::Words => "WORDS",
            Kind::Calls => "CALLS",
        }
    }

    pub fn next(self) -> Kind {
        match self {
            Kind::Koch => Kind::Words,
            Kind::Words => Kind::Calls,
            Kind::Calls => Kind::Koch,
        }
    }
}

pub const GROUP_LEN: usize = 5;
pub const MAX_GROUP: usize = 10;
/// Copies longer than this are cut: nobody copies 24 characters of a 10-character group.
pub const MAX_COPY: usize = 24;
pub const MIN_LEVEL: u8 = FIRST_LEVEL;
pub const MAX_LEVEL: u8 = KOCH.len() as u8;
/// Koch's rule: add a character once you copy 90% of the current set. Accuracy here is one minus
/// the character error rate, (substituted + missed + extra) / sent, so keying extra characters
/// costs as much as missing them.
const LEVEL_WINDOW: u16 = 50;
const LEVEL_PERCENT: u16 = 90;

/// Common words and abbreviations heard on the air.
const WORDS: [&str; 64] = [
    "CQ", "DE", "TU", "73", "72", "5NN", "RST", "UR", "NAME", "QTH", "RIG", "ANT", "WX", "FB", "OM", "YL", "GM", "GA", "GE",
    "ES", "HR", "TNX", "AGN", "PSE", "QRS", "QRZ", "QSL", "QSO", "QRP", "POTA", "SOTA", "PARK", "<BK>", "<KN>", "<SK>", "HW",
    "CPY", "CUL", "GL", "BEST", "KEY", "THE", "AND", "FOR", "YOU", "ARE", "WITH", "THAT", "HAVE", "THIS", "FROM", "WILL",
    "WORK", "GOOD", "SUN", "RAIN", "COLD", "HOT", "WATTS", "DIPOLE", "VERT", "MILES", "TODAY", "AGAIN",
];

/// How one sent character was copied.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mark {
    Right,
    Wrong(u8),
    Missed,
}

#[derive(Clone, Copy, Debug)]
pub struct Score {
    /// One mark per sent character.
    pub marks: [Mark; MAX_GROUP],
    pub len: usize,
    pub right: u8,
    /// Copied characters that match nothing sent.
    pub extra: u8,
    /// The Koch level went up after this group.
    pub level_up: bool,
}

pub struct Drill {
    pub kind: Kind,
    pub level: u8,
    rng: Rng,
    window_right: u16,
    window_total: u16,
}

impl Drill {
    pub fn new(kind: Kind, level: u8, seed: u32) -> Drill {
        Drill { kind, level: level.clamp(MIN_LEVEL, MAX_LEVEL), rng: Rng::new(seed), window_right: 0, window_total: 0 }
    }

    pub fn set_level(&mut self, level: u8) {
        self.level = level.clamp(MIN_LEVEL, MAX_LEVEL);
        self.window_right = 0;
        self.window_total = 0;
    }

    /// Progress toward the next level over the current window: (right less extra, total sent).
    pub fn window(&self) -> (u16, u16) {
        (self.window_right, self.window_total)
    }

    /// Continues the window of an earlier drill at this level.
    pub fn resume(&mut self, (right, total): (u16, u16)) {
        (self.window_right, self.window_total) = (right, total);
    }

    /// The next group to send.
    pub fn next(&mut self, letters: &Letters) -> Text<MAX_GROUP> {
        let mut group = Text::new();
        match self.kind {
            Kind::Koch => {
                for _ in 0..GROUP_LEN {
                    group.push(self.pick(letters));
                }
            }
            Kind::Words => {
                for c in morse::chars(WORDS[self.rng.below(WORDS.len() as u32) as usize].as_bytes()) {
                    group.push(c);
                }
            }
            Kind::Calls => {
                let digit = self.rng.below(10) as u8;
                let _ = write!(group, "{}", us_call(digit, &mut self.rng).as_str());
            }
        }
        group
    }

    /// A Koch character, weighted toward weak ones and the newest.
    fn pick(&mut self, letters: &Letters) -> u8 {
        let n = self.level as usize;
        let weight = |i: usize| letters.weight(i) * if i == n - 1 { 2 } else { 1 };
        let total: u32 = (0..n).map(weight).sum();
        let mut r = self.rng.below(total);
        for (i, &c) in KOCH.iter().enumerate().take(n) {
            if r < weight(i) {
                return c;
            }
            r -= weight(i);
        }
        KOCH[n - 1]
    }

    /// Scores `copied` against `sent` as `score_copy` does, and moves the Koch level.
    pub fn score(&mut self, sent: &str, copied: &str, letters: &mut Letters) -> Score {
        let mut score = score_copy(sent, copied, letters);
        if self.kind == Kind::Koch {
            self.window_right += score.right.saturating_sub(score.extra) as u16;
            self.window_total += score.len as u16;
            let passed = self.window_right * 100 >= self.window_total * LEVEL_PERCENT;
            if self.window_total >= LEVEL_WINDOW && passed && self.level < MAX_LEVEL {
                self.set_level(self.level + 1);
                score.level_up = true;
            } else if self.window_total >= LEVEL_WINDOW {
                self.window_right = 0;
                self.window_total = 0;
            }
        }
        score
    }
}

/// Scores `copied` against `sent`, ignoring spaces and case, and records each sent character.
pub fn score_copy(sent: &str, copied: &str, letters: &mut Letters) -> Score {
    let sent = squash::<MAX_GROUP>(sent);
    let copied = squash::<MAX_COPY>(copied);
    let score = align(sent.as_str().as_bytes(), copied.as_str().as_bytes());
    for (&s, mark) in sent.as_str().as_bytes().iter().zip(&score.marks) {
        match *mark {
            Mark::Right => letters.record(s, Some(s)),
            Mark::Wrong(c) if letters::index(c).is_some() => letters.record(s, Some(c)),
            Mark::Wrong(_) | Mark::Missed => letters.record(s, None),
        }
    }
    score
}

pub(crate) fn squash<const N: usize>(s: &str) -> Text<N> {
    let mut out = Text::new();
    for c in morse::chars(s.as_bytes()).filter(|c| *c != b' ') {
        out.push(c.to_ascii_uppercase());
    }
    out
}

/// Edit-distance alignment of the copy against what was sent.
fn align(sent: &[u8], copied: &[u8]) -> Score {
    let (n, m) = (sent.len().min(MAX_GROUP), copied.len().min(MAX_COPY));
    let mut d = [[0u8; MAX_COPY + 1]; MAX_GROUP + 1];
    for (i, row) in d.iter_mut().enumerate().take(n + 1) {
        row[0] = i as u8;
    }
    for (j, cell) in d[0].iter_mut().enumerate().take(m + 1) {
        *cell = j as u8;
    }
    for i in 1..=n {
        for j in 1..=m {
            let change = d[i - 1][j - 1] + (sent[i - 1] != copied[j - 1]) as u8;
            d[i][j] = change.min(d[i - 1][j] + 1).min(d[i][j - 1] + 1);
        }
    }
    let mut score = Score { marks: [Mark::Missed; MAX_GROUP], len: n, right: 0, extra: 0, level_up: false };
    let (mut i, mut j) = (n, m);
    while i > 0 || j > 0 {
        if i > 0 && j > 0 && d[i][j] == d[i - 1][j - 1] + (sent[i - 1] != copied[j - 1]) as u8 {
            score.marks[i - 1] = if sent[i - 1] == copied[j - 1] { Mark::Right } else { Mark::Wrong(copied[j - 1]) };
            (i, j) = (i - 1, j - 1);
        } else if i > 0 && d[i][j] == d[i - 1][j] + 1 {
            score.marks[i - 1] = Mark::Missed;
            i -= 1;
        } else {
            score.extra += 1;
            j -= 1;
        }
    }
    score.right = score.marks[..n].iter().filter(|m| **m == Mark::Right).count() as u8;
    score
}

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;

    fn marks(sent: &str, copied: &str) -> std::vec::Vec<Mark> {
        let score = align(sent.as_bytes(), copied.as_bytes());
        score.marks[..score.len].to_vec()
    }

    #[test]
    fn alignment_finds_wrong_missed_and_extra() {
        use Mark::*;
        assert_eq!(marks("KMRSE", "KMRSE"), [Right; 5]);
        assert_eq!(marks("KMRSE", "KMXSE"), [Right, Right, Wrong(b'X'), Right, Right]);
        assert_eq!(marks("KMRSE", "KMSE"), [Right, Right, Missed, Right, Right]);
        let score = align(b"KMRSE", b"KMRRSE");
        assert_eq!((score.right, score.extra), (5, 1));
        assert_eq!(marks("KMRSE", ""), [Missed; 5]);
    }

    #[test]
    fn koch_groups_use_only_the_level_set() {
        let mut drill = Drill::new(Kind::Koch, 4, 3);
        let letters = Letters::default();
        for _ in 0..50 {
            let group = drill.next(&letters);
            assert_eq!(group.len(), GROUP_LEN);
            assert!(group.as_str().bytes().all(|b| KOCH[..4].contains(&b)), "{}", group.as_str());
        }
    }

    #[test]
    fn ninety_percent_over_fifty_characters_adds_a_character() {
        let mut drill = Drill::new(Kind::Koch, 2, 5);
        let mut letters = Letters::default();
        for _ in 0..9 {
            assert!(!drill.score("KMKMK", "KMKMK", &mut letters).level_up);
        }
        assert!(drill.score("KMKMK", "KMKMM", &mut letters).level_up);
        assert_eq!((drill.level, drill.window()), (3, (0, 0)));
        let k = letters::index(b'K').unwrap();
        assert_eq!(letters.accuracy(k), Some((96, 30)));
    }

    #[test]
    fn extra_characters_count_as_errors() {
        let mut drill = Drill::new(Kind::Koch, 2, 1);
        let mut letters = Letters::default();
        for _ in 0..10 {
            assert!(!drill.score("KMKMK", "KMKMKMKMKM", &mut letters).level_up);
        }
        assert_eq!(drill.level, 2);
    }

    #[test]
    fn spaces_and_case_do_not_count() {
        let mut drill = Drill::new(Kind::Words, 2, 1);
        let score = drill.score("POTA", "po ta", &mut Letters::default());
        assert_eq!(score.right, 4);
    }

    #[test]
    fn a_prosign_scores_as_one_character() {
        let mut drill = Drill::new(Kind::Koch, MAX_LEVEL, 1);
        let mut letters = Letters::default();
        let sent: Text<MAX_GROUP> = morse::chars(b"K<SK>").fold(Text::new(), |mut t, c| {
            t.push(c);
            t
        });
        let score = drill.score(sent.as_str(), "K <SK>", &mut letters);
        assert_eq!((score.len, score.right), (2, 2));
        let sk = letters::index(sent.as_str().as_bytes()[1]).unwrap();
        assert_eq!(letters.accuracy(sk), Some((100, 1)));
    }
}
