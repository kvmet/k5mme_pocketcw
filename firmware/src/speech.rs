//! Speaking text from the content pack: each word the pack holds, and every other word spelled
//! a character at a time. A word on its own gets its meaning after it. `ui` reads the clips into one
//! buffer while the synth holds none, then lends the buffer to the synth until the last clip
//! ends. The clips are read outside the audio lock, so the sample interrupt never waits on the
//! SPI flash.

use core::cell::UnsafeCell;

use pocketcw_logic::adpcm::{Clips, MAX_CLIPS, Upsample, played_samples};
use rtic::Mutex;

use crate::audio::{Audio, SAMPLE_HZ};
use crate::lcd::Lcd;
use crate::pack::Pack;

/// About 3.7 s at 8789 Hz: the longest word and meaning `tools/pack/pack.py` makes, and all of
/// one `say`.
const BUFFER_LEN: usize = 16 * 1024;
const GAP_MS: f32 = 120.0;
/// With the 50 ms of silence the clips' ends hold, about the pause Kokoro makes after a sentence.
const MEANING_GAP_MS: f32 = 200.0;

struct Buffer(UnsafeCell<[u8; BUFFER_LEN]>);

// SAFETY: `say` is the only writer, and it writes only while the synth holds no clips.
unsafe impl Sync for Buffer {}

static BUFFER: Buffer = Buffer(UnsafeCell::new([0; BUFFER_LEN]));

/// Where each clip ends in the buffer.
struct Ends {
    ends: [u16; MAX_CLIPS],
    count: usize,
    used: usize,
}

impl Ends {
    /// Reads the clip at `clip` after the others. False once the buffer or the clip count is
    /// full.
    fn add(&mut self, lcd: &mut Lcd, pack: &Pack, clip: Option<u32>, buffer: &mut [u8]) -> bool {
        let Some(at) = clip else { return true };
        if self.count == MAX_CLIPS {
            return false;
        }
        let Some(len) = pack.load_clip(lcd, at, &mut buffer[self.used..]) else { return false };
        self.used += len;
        self.ends[self.count] = self.used as u16;
        self.count += 1;
        true
    }
}

/// Stops any speech and speaks `text`, upper case, as far as the buffer holds. Returns how long
/// the speech lasts in ms; 0 when nothing in `text` has a clip.
pub fn say(lcd: &mut Lcd, pack: &Pack, text: &[u8], audio: &mut impl Mutex<T = Audio>) -> u32 {
    audio.lock(|audio| audio.synth.say(None));
    // SAFETY: the synth holds no clips now, and only this function gives it some, so nothing
    // else reads the buffer while it fills.
    let buffer = unsafe { &mut *BUFFER.0.get() };
    let mut ends = Ends { ends: [0; MAX_CLIPS], count: 0, used: 0 };
    let tokens = || text.split(|&b| b == b' ').filter(|t| !t.is_empty());
    let alone = tokens().count() == 1;
    let mut gap_ms = GAP_MS;
    'words: for token in tokens() {
        if let Some(word) = pack.find(lcd, token) {
            if !ends.add(lcd, pack, word.clip, buffer) {
                break;
            }
            if alone && word.meaning.is_some() {
                gap_ms = MEANING_GAP_MS;
                ends.add(lcd, pack, word.meaning, buffer);
            }
            continue;
        }
        for &c in token {
            if let Some(word) = pack.char(lcd, c)
                && !ends.add(lcd, pack, word.clip, buffer)
            {
                break 'words;
            }
        }
    }
    if ends.count == 0 {
        return 0;
    }
    // SAFETY: the mutable borrow above has ended; the synth only reads the clips.
    let clips: &'static [u8] = unsafe { &(&*BUFFER.0.get())[..ends.used] };
    let gap = (SAMPLE_HZ / pack.factor.max(1) as f32 * gap_ms / 1000.0) as u16;
    audio.lock(|audio| {
        audio.synth.say(Some(Upsample::new(Clips::new(clips, &ends.ends[..ends.count], gap), pack.factor)));
        audio.wake();
    });
    (played_samples(ends.used, ends.count, gap, pack.factor) as f32 * 1000.0 / SAMPLE_HZ) as u32
}
