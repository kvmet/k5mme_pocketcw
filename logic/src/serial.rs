//! Commands a computer sends over the USB serial port, one per line.

use crate::buttons::Button;
use crate::player::MAX_TEXT;
use crate::text::Text;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Command {
    /// Send the screen.
    Shot,
    /// Press and release a button.
    Press(Button),
    /// Key this text, upper case, as a straight key at the keyer speed.
    Send(Text<MAX_TEXT>),
}

const SEND: &[u8] = b"SEND ";
/// The longest line that can hold a command; a longer one is ignored up to its end.
const LINE_LEN: usize = SEND.len() + MAX_TEXT;

const BUTTON_NAMES: [(&[u8], Button); 7] = [
    (b"F1", Button::F1),
    (b"F2", Button::F2),
    (b"F3", Button::F3),
    (b"F4", Button::F4),
    (b"LEFT", Button::Left),
    (b"RIGHT", Button::Right),
    (b"CLICK", Button::Click),
];

/// Collects bytes into lines and names the command on each. A line ends at `\n` or `\r`.
pub struct CommandLine {
    line: [u8; LINE_LEN],
    len: usize,
    /// The line outgrew `line`, so it is no command.
    overflow: bool,
}

impl Default for CommandLine {
    fn default() -> Self {
        CommandLine { line: [0; LINE_LEN], len: 0, overflow: false }
    }
}

impl CommandLine {
    /// The command that `byte` ends, if any.
    pub fn push(&mut self, byte: u8) -> Option<Command> {
        if byte == b'\n' || byte == b'\r' {
            let command = if self.overflow { None } else { parse(&self.line[..self.len]) };
            *self = CommandLine::default();
            return command;
        }
        if self.len == LINE_LEN {
            self.overflow = true;
        } else {
            self.line[self.len] = byte;
            self.len += 1;
        }
        None
    }
}

/// `SHOT`, `PRESS` and a button name, or `SEND` and text.
fn parse(line: &[u8]) -> Option<Command> {
    if line == b"SHOT" {
        return Some(Command::Shot);
    }
    if let Some(text) = line.strip_prefix(SEND) {
        let mut send = Text::new();
        for &b in text {
            send.push(b.to_ascii_uppercase());
        }
        return Some(Command::Send(send));
    }
    let name = line.strip_prefix(b"PRESS ")?;
    BUTTON_NAMES.iter().find(|(n, _)| *n == name).map(|&(_, button)| Command::Press(button))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn commands(bytes: &[u8]) -> [Option<Command>; 4] {
        let mut line = CommandLine::default();
        let mut found = [None; 4];
        let mut n = 0;
        for &b in bytes {
            if let Some(command) = line.push(b) {
                found[n] = Some(command);
                n += 1;
            }
        }
        found
    }

    #[test]
    fn shot_ends_at_either_line_end() {
        assert_eq!(commands(b"SHOT\n"), [Some(Command::Shot), None, None, None]);
        assert_eq!(commands(b"SHOT\r\nSHOT\r"), [Some(Command::Shot), Some(Command::Shot), None, None]);
    }

    #[test]
    fn other_lines_are_not_commands() {
        assert_eq!(commands(b"SHOT"), [None; 4], "no line end yet");
        assert_eq!(commands(b"shot\nSHOTS\n SHOT\n\n"), [None; 4]);
    }

    #[test]
    fn press_names_each_button() {
        for (name, button) in BUTTON_NAMES {
            let mut line = CommandLine::default();
            let found = [b"PRESS ".as_slice(), name, b"\n"].concat().iter().find_map(|&b| line.push(b));
            assert_eq!(found, Some(Command::Press(button)));
        }
        assert_eq!(commands(b"PRESS\nPRESS F5\nPRESS f1\nPRESS  F1\n"), [None; 4]);
    }

    fn send(text: &str) -> Option<Command> {
        Some(Command::Send(Text::format(format_args!("{text}"))))
    }

    #[test]
    fn send_takes_its_text_in_upper_case() {
        assert_eq!(commands(b"SEND cq de K5MME <ar>\n"), [send("CQ DE K5MME <AR>"), None, None, None]);
        assert_eq!(commands(b"SEND \nSEND\nsend E\n"), [send(""), None, None, None]);
    }

    #[test]
    fn a_long_line_is_ignored_and_the_next_line_still_counts() {
        let long = [b'X'; LINE_LEN];
        assert_eq!(commands(&[&long[..], b"SHOT\nSHOT\n"].concat()), [Some(Command::Shot), None, None, None]);
        let full_send = [SEND, &[b'E'; MAX_TEXT][..], b"\n"].concat();
        assert!(matches!(commands(&full_send)[0], Some(Command::Send(t)) if t.len() == MAX_TEXT));
        let over = [SEND, &[b'E'; MAX_TEXT + 1][..], b"\nSHOT\n"].concat();
        assert_eq!(commands(&over), [Some(Command::Shot), None, None, None]);
    }
}
