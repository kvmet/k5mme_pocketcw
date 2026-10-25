//! The content pack in SPI flash (format in `pocketcw_logic::pack`), read through the screen's
//! SPI bus.

use pocketcw_logic::pack::{self, CLIP_START, CLIPS, Crc32, Error, HEADER_MAX, Header, SECTION_START, Section, WORD_ENTRY, WORDS, Word};

use crate::lcd::Lcd;

/// Where `F000400.bin` lands.
pub const ADDRESS: u32 = 0x10_0000;
/// Bytes of the CRC check's reads.
const CHUNK: usize = 256;

pub struct Pack {
    words: Section,
    clips: Section,
    count: u16,
    /// The output sample rate divided by the clips' rate.
    pub factor: u8,
}

impl Pack {
    /// Reads the header and checks the word list's CRC. The clips are too large to check at
    /// every startup.
    pub fn open(lcd: &mut Lcd) -> Result<Pack, Error> {
        let mut flash = lcd.spi_flash();
        let mut header = [0u8; HEADER_MAX];
        flash.read(ADDRESS, &mut header);
        let header = Header::parse(&header)?;
        let (Some(words), Some(clips)) = (header.section(WORDS), header.section(CLIPS)) else {
            return Err(Error::Corrupt);
        };
        let mut crc = Crc32::default();
        let mut chunk = [0u8; CHUNK];
        for at in (0..words.len).step_by(CHUNK) {
            let n = (words.len - at).min(CHUNK as u32) as usize;
            flash.read(ADDRESS + words.offset + at, &mut chunk[..n]);
            crc.update(&chunk[..n]);
        }
        let mut start = [0u8; SECTION_START as usize];
        flash.read(ADDRESS + words.offset, &mut start);
        let count = pack::word_count(&start);
        let fits = |s: Section| s.len >= SECTION_START;
        if crc.finish() != words.crc || !fits(words) || !fits(clips) || pack::word_offset(count) > words.len {
            return Err(Error::Corrupt);
        }
        flash.read(ADDRESS + clips.offset, &mut start);
        Ok(Pack { words, clips, count, factor: start[0] })
    }

    pub fn count(&self) -> u16 {
        self.count
    }

    pub fn word(&self, lcd: &mut Lcd, index: u16) -> Option<Word> {
        if index >= self.count {
            return None;
        }
        let mut entry = [0u8; WORD_ENTRY];
        lcd.spi_flash().read(ADDRESS + self.words.offset + pack::word_offset(index), &mut entry);
        Word::parse(&entry)
    }

    /// The first word with this text.
    pub fn find(&self, lcd: &mut Lcd, text: &[u8]) -> Option<Word> {
        (0..self.count).filter_map(|i| self.word(lcd, i)).find(|w| w.text() == text)
    }

    /// Reads the word's clip into `buffer` and returns its length. None when the word has no
    /// clip, or the clip runs past its section or the buffer.
    pub fn load_clip(&self, lcd: &mut Lcd, word: &Word, buffer: &mut [u8]) -> Option<usize> {
        let at = word.clip?;
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
