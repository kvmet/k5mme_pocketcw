//! Speaking a word from the content pack. `ui` reads the clip into one buffer while the synth
//! holds no clip, then lends the buffer to the synth until the clip ends. The clip is read
//! outside the audio lock, so the sample interrupt never waits on the SPI flash.

use core::cell::UnsafeCell;

use pocketcw_logic::adpcm::{Clip, Upsample};
use pocketcw_logic::pack::Word;
use rtic::Mutex;

use crate::audio::Audio;
use crate::lcd::Lcd;
use crate::pack::Pack;

/// The longest clip: about 5.6 s at 5859 Hz. `tools/pack.py` refuses longer ones.
const CLIP_MAX: usize = 16 * 1024;

struct Buffer(UnsafeCell<[u8; CLIP_MAX]>);

// SAFETY: `say` is the only writer, and it writes only while the synth holds no clip.
unsafe impl Sync for Buffer {}

static BUFFER: Buffer = Buffer(UnsafeCell::new([0; CLIP_MAX]));

/// Stops any clip and starts `word`'s. False when the word has no clip that fits.
pub fn say(lcd: &mut Lcd, pack: &Pack, word: &Word, audio: &mut impl Mutex<T = Audio>) -> bool {
    audio.lock(|audio| audio.synth.say(None));
    // SAFETY: the synth holds no clip now, and only this function gives it one, so nothing else
    // reads the buffer while it fills.
    let buffer = unsafe { &mut *BUFFER.0.get() };
    let Some(len) = pack.load_clip(lcd, word, buffer) else { return false };
    // SAFETY: the mutable borrow above has ended; the synth only reads the clip.
    let clip: &'static [u8] = unsafe { &(&*BUFFER.0.get())[..len] };
    audio.lock(|audio| {
        audio.synth.say(Some(Upsample::new(Clip::new(clip), pack.factor)));
        audio.wake();
    });
    true
}
