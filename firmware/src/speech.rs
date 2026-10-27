//! Speaking text from the content pack: each word the pack has a clip for, and every other word
//! spelled a character at a time. A word on its own gets its meaning after it. `ui` reads the clips into one
//! buffer while the synth holds none, then lends the buffer to the synth until the last clip
//! ends. The clips are read outside the audio lock, so the sample interrupt never waits on the
//! SPI flash.

use core::cell::UnsafeCell;

use pocketcw_logic::adpcm::{Clips, MAX_CLIPS, Slot, Upsample, played_samples};
use rtic::Mutex;

use crate::audio::{Audio, SAMPLE_HZ};
use crate::lcd::Lcd;
use crate::pack::Pack;

/// About 3.7 s at 8789 Hz: the longest word or spelling and meaning `tools/pack/pack.py` makes,
/// and all of one `say`.
const BUFFER_LEN: usize = 16 * 1024;
const GAP_MS: f32 = 120.0;
/// With the 50 ms of silence the clips' ends hold, about the pause Kokoro makes after a sentence.
const MEANING_GAP_MS: f32 = 200.0;

struct Buffer(UnsafeCell<[u8; BUFFER_LEN]>);

// SAFETY: `say` is the only writer, and it writes only while the synth holds no clips.
unsafe impl Sync for Buffer {}

static BUFFER: Buffer = Buffer(UnsafeCell::new([0; BUFFER_LEN]));

/// The clips read into the buffer so far.
struct Queue {
    slots: [Slot; MAX_CLIPS],
    count: usize,
    used: usize,
}

impl Queue {
    /// Reads the clip at `clip` after the others, `gap` samples after the last. False once the
    /// buffer or the clip count is full.
    fn add(&mut self, lcd: &mut Lcd, pack: &Pack, clip: Option<u32>, gap: u16, buffer: &mut [u8]) -> bool {
        let Some(at) = clip else { return true };
        if self.count == MAX_CLIPS {
            return false;
        }
        let Some(len) = pack.load_clip(lcd, at, &mut buffer[self.used..]) else { return false };
        self.used += len;
        self.slots[self.count] = Slot { end: self.used as u16, gap: if self.count == 0 { 0 } else { gap } };
        self.count += 1;
        true
    }

    /// Reads the clip of each character of `text` that the pack has, such as the Q, T and H of
    /// `QTH` and the A and R of `<AR>`.
    fn spell(&mut self, lcd: &mut Lcd, pack: &Pack, text: &[u8], gap: u16, buffer: &mut [u8]) -> bool {
        for &c in text {
            if let Some(word) = pack.char(lcd, c)
                && !self.add(lcd, pack, word.clip, gap, buffer)
            {
                return false;
            }
        }
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
    let mut queue = Queue { slots: [Slot::default(); MAX_CLIPS], count: 0, used: 0 };
    let gap = |ms: f32| (SAMPLE_HZ / pack.factor.max(1) as f32 * ms / 1000.0) as u16;
    let tokens = || text.split(|&b| b == b' ').filter(|t| !t.is_empty());
    let alone = tokens().count() == 1;
    for token in tokens() {
        let word = pack.find(lcd, token);
        // The pack holds a spelled word, such as `QTH`, with no clip of its own.
        let read = match word.and_then(|w| w.clip) {
            Some(clip) => queue.add(lcd, pack, Some(clip), gap(GAP_MS), buffer),
            None => queue.spell(lcd, pack, token, gap(GAP_MS), buffer),
        };
        if !read {
            break;
        }
        if alone && let Some(meaning) = word.and_then(|w| w.meaning) {
            queue.add(lcd, pack, Some(meaning), gap(MEANING_GAP_MS), buffer);
        }
    }
    if queue.count == 0 {
        return 0;
    }
    let slots = &queue.slots[..queue.count];
    // SAFETY: the mutable borrow above has ended; the synth only reads the clips.
    let clips: &'static [u8] = unsafe { &(&*BUFFER.0.get())[..queue.used] };
    audio.lock(|audio| {
        audio.synth.say(Some(Upsample::new(Clips::new(clips, slots), pack.factor)));
        audio.wake();
    });
    (played_samples(slots, pack.factor) as f32 * 1000.0 / SAMPLE_HZ) as u32
}
