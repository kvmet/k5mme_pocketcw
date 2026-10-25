use core::fmt;

/// Fixed-capacity string for formatting without a heap. Text past the capacity is an error.
#[derive(Clone, Copy)]
pub struct Text<const N: usize> {
    buf: [u8; N],
    len: usize,
}

/// Equal by contents: bytes past `len` are left over from `pop` and `clear`.
impl<const N: usize> PartialEq for Text<N> {
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}

impl<const N: usize> Eq for Text<N> {}

impl<const N: usize> fmt::Debug for Text<N> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self.as_str(), f)
    }
}

impl<const N: usize> Default for Text<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> Text<N> {
    pub const fn new() -> Self {
        Text { buf: [0; N], len: 0 }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn clear(&mut self) {
        self.len = 0;
    }

    /// Appends one ASCII byte; false when full.
    pub fn push(&mut self, byte: u8) -> bool {
        if self.len == N || !byte.is_ascii() {
            return false;
        }
        self.buf[self.len] = byte;
        self.len += 1;
        true
    }

    /// Removes the last character, all of its bytes.
    pub fn pop(&mut self) -> Option<char> {
        let c = self.as_str().chars().next_back()?;
        self.len -= c.len_utf8();
        Some(c)
    }

    pub fn as_str(&self) -> &str {
        // Only whole `&str` values are ever copied in, so the bytes are valid UTF-8.
        core::str::from_utf8(&self.buf[..self.len]).unwrap()
    }
}

impl<const N: usize> Text<N> {
    /// Formats `args`, cut at the capacity: too-long text shows cut off, not missing.
    pub fn format(args: fmt::Arguments) -> Self {
        let mut text = Self::new();
        // `Cut` never fails, so an error here can only come from a `Display` impl.
        let _ = fmt::Write::write_fmt(&mut Cut(&mut text), args);
        text
    }
}

/// Writes into a `Text`, dropping what does not fit.
struct Cut<'a, const N: usize>(&'a mut Text<N>);

impl<const N: usize> fmt::Write for Cut<'_, N> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let mut end = s.len().min(N - self.0.len);
        while !s.is_char_boundary(end) {
            end -= 1;
        }
        fmt::Write::write_str(self.0, &s[..end])
    }
}

impl<const N: usize> fmt::Write for Text<N> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let end = self.len + s.len();
        if end > N {
            return Err(fmt::Error);
        }
        self.buf[self.len..end].copy_from_slice(s.as_bytes());
        self.len = end;
        Ok(())
    }
}

/// A number with two decimals, rounded. It writes integers, so it needs no float formatting
/// code. Negative numbers write as 0.00.
pub struct Hundredths(pub f32);

impl fmt::Display for Hundredths {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let hundredths = (self.0 * 100.0 + 0.5) as u32;
        write!(f, "{}.{:02}", hundredths / 100, hundredths % 100)
    }
}

/// Splits `s` into lines of at most `cols` characters, breaking at spaces. A word longer than a
/// line is cut where the line ends.
pub fn wrap(s: &str, cols: usize) -> impl Iterator<Item = &str> {
    let mut rest = s.trim_start_matches(' ');
    core::iter::from_fn(move || {
        if rest.is_empty() || cols == 0 {
            return None;
        }
        let line = if rest.len() <= cols {
            rest
        } else {
            match rest[..=cols].rfind(' ') {
                Some(at) if at > 0 => &rest[..at],
                _ => &rest[..cols],
            }
        };
        rest = rest[line.len()..].trim_start_matches(' ');
        Some(line.trim_end_matches(' '))
    })
}

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    use std::vec::Vec;

    fn lines(s: &str, cols: usize) -> Vec<&str> {
        wrap(s, cols).collect()
    }

    #[test]
    fn format_cuts_at_capacity() {
        assert_eq!(Text::<8>::format(format_args!("LOG {}/{}", 3, 10)).as_str(), "LOG 3/10");
        assert_eq!(Text::<6>::format(format_args!("LOG {}/{}", 3, 10)).as_str(), "LOG 3/");
        assert_eq!(Text::<2>::format(format_args!("A{}", "é")).as_str(), "A");
    }

    #[test]
    fn pop_takes_a_whole_character() {
        let mut text = Text::<4>::format(format_args!("Aé"));
        assert_eq!((text.pop(), text.as_str()), (Some('é'), "A"));
        assert_eq!((text.pop(), text.pop()), (Some('A'), None));
    }

    #[test]
    fn wraps_at_spaces_and_cuts_long_words() {
        assert_eq!(lines("CQ POTA DE K5MME K5MME K", 10), ["CQ POTA DE", "K5MME", "K5MME K"]);
        assert_eq!(lines("K5MME  W1AW", 5), ["K5MME", "W1AW"]);
        assert_eq!(lines("ABCDEFGHIJKL MN", 5), ["ABCDE", "FGHIJ", "KL MN"]);
        assert_eq!(lines("  ", 5), Vec::<&str>::new());
        assert_eq!(lines("TU 72", 27), ["TU 72"]);
    }
}
