//! The content pack in SPI flash (format in `pocketcw_logic::pack`), read through the screen's
//! SPI bus.

use pocketcw_logic::pack::{self, CLIP_START, CLIPS, Crc32, Error, HEADER_MAX, Header, SECTION_START, Section, WORD_ENTRY, WORDS, Word};

use crate::lcd::Lcd;

/// Where `F000400.bin` lands.
pub const ADDRESS: u32 = 0x10_0000;
/// Entries per SPI flash read when scanning the word list.
const ENTRIES_PER_READ: usize = 10;
/// More than the 49 characters Morse sends.
const MAX_CHARS: usize = 64;

pub struct Pack {
    words: Section,
    clips: Section,
    count: u16,
    /// The output sample rate divided by the clips' rate.
    pub factor: u8,
    /// Each spelling character and its entry.
    chars: [(u8, u16); MAX_CHARS],
    char_count: usize,
    /// The first entry that is a word, not a character.
    first_word: u16,
}

impl Pack {
    /// Reads the header, checks the word list's CRC and finds the characters. The clips are
    /// too large to check at every startup.
    pub fn open(lcd: &mut Lcd) -> Result<Pack, Error> {
        let mut header = [0u8; HEADER_MAX];
        lcd.spi_flash().read(ADDRESS, &mut header);
        let header = Header::parse(&header)?;
        let (Some(words), Some(clips)) = (header.section(WORDS), header.section(CLIPS)) else {
            return Err(Error::Corrupt);
        };
        let mut start = [0u8; SECTION_START as usize];
        lcd.spi_flash().read(ADDRESS + words.offset, &mut start);
        let count = pack::word_count(&start);
        if words.len != pack::word_offset(count) || clips.len < SECTION_START {
            return Err(Error::Corrupt);
        }
        let mut crc = Crc32::default();
        crc.update(&start);
        let mut chars = [(0, 0); MAX_CHARS];
        let mut char_count = 0;
        let mut first_word = count;
        scan(lcd, words, count, |index, entry| {
            crc.update(entry);
            let word = Word::parse(entry);
            if let Some(word) = word
                && let &[c] = word.text()
                && word.char
                && char_count < MAX_CHARS
            {
                chars[char_count] = (c, index);
                char_count += 1;
            }
            if word.is_some_and(|w| !w.char) {
                first_word = first_word.min(index);
            }
            true
        });
        if crc.finish() != words.crc {
            return Err(Error::Corrupt);
        }
        lcd.spi_flash().read(ADDRESS + clips.offset, &mut start);
        Ok(Pack { words, clips, count, factor: start[0], chars, char_count, first_word })
    }

    pub fn count(&self) -> u16 {
        self.count
    }

    /// The words, not characters, as (first index, count). The pack lists the characters
    /// first.
    pub fn words(&self) -> (u16, u16) {
        (self.first_word, self.count - self.first_word)
    }

    pub fn word(&self, lcd: &mut Lcd, index: u16) -> Option<Word> {
        if index >= self.count {
            return None;
        }
        let mut entry = [0u8; WORD_ENTRY];
        lcd.spi_flash().read(ADDRESS + self.words.offset + pack::word_offset(index), &mut entry);
        Word::parse(&entry)
    }

    /// The first word, not character, with this text.
    pub fn find(&self, lcd: &mut Lcd, text: &[u8]) -> Option<Word> {
        let mut found = None;
        scan(lcd, self.words, self.count, |_, entry| {
            found = Word::parse(entry).filter(|w| !w.char && w.text() == text);
            found.is_none()
        });
        found
    }

    /// The character's entry, for spelling.
    pub fn char(&self, lcd: &mut Lcd, c: u8) -> Option<Word> {
        let &(_, index) = self.chars[..self.char_count].iter().find(|&&(k, _)| k == c)?;
        self.word(lcd, index)
    }

    /// Reads the clip at `at` in the `CLIP` section into `buffer` and returns its length. None
    /// when the clip runs past its section or the buffer.
    pub fn load_clip(&self, lcd: &mut Lcd, at: u32, buffer: &mut [u8]) -> Option<usize> {
        let mut flash = lcd.spi_flash();
        let mut start = [0u8; CLIP_START as usize];
        flash.read(ADDRESS + self.clips.offset + at, &mut start);
        let len = pack::clip_len(&start) as usize;
        let end = at as u64 + CLIP_START as u64 + len as u64;
        if end > self.clips.len as u64 || len > buffer.len() {
            return None;
        }
        flash.read(ADDRESS + self.clips.offset + at + CLIP_START, &mut buffer[..len]);
        Some(len)
    }
}

/// Calls `visit` with each entry's index and bytes, in order, until it returns false.
fn scan(lcd: &mut Lcd, words: Section, count: u16, mut visit: impl FnMut(u16, &[u8; WORD_ENTRY]) -> bool) {
    let mut chunk = [0u8; WORD_ENTRY * ENTRIES_PER_READ];
    let mut index = 0;
    while index < count {
        let n = (count - index).min(ENTRIES_PER_READ as u16) as usize;
        let bytes = &mut chunk[..n * WORD_ENTRY];
        lcd.spi_flash().read(ADDRESS + words.offset + pack::word_offset(index), bytes);
        for entry in bytes.chunks_exact(WORD_ENTRY) {
            if !visit(index, entry.try_into().unwrap()) {
                return;
            }
            index += 1;
        }
    }
}
