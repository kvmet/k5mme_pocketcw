//! Volume levels: 0 mutes, then 1.5 dB steps from -60 dB at level 1 up to +4.5 dB.

pub const MAX_LEVEL: u8 = 44;
/// The ES8388 output stage alone reaches down to -45 dB, 10 steps above level 1.
const OUTPUT_FLOOR_STEPS: u8 = 10;
pub const MAX_MIXER_CUT: u8 = 5;

/// How the ES8388 makes one level.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CodecGain {
    /// Input mixer cut in 3 dB steps, 0..=`MAX_MIXER_CUT`.
    pub mixer_cut: u8,
    /// Output volume register: 0 is -45 dB, then 1.5 dB per step.
    pub output: u8,
}

/// None mutes. The output stage cuts first; the mixer cuts only below its range, since a cut
/// before the output stage raises the hiss relative to the tone.
pub fn codec_gain(level: u8) -> Option<CodecGain> {
    if level == 0 {
        return None;
    }
    let steps = level.min(MAX_LEVEL) - 1;
    let mixer_cut = OUTPUT_FLOOR_STEPS.saturating_sub(steps).div_ceil(2);
    Some(CodecGain { mixer_cut, output: steps + 2 * mixer_cut - OUTPUT_FLOOR_STEPS })
}

/// The WM8978 speaker volume register for `level`, in 1 dB steps with 57 at 0 dB; None mutes.
/// Levels keep their ES8388 dB values, so `MAX_LEVEL` gives +4 dB, under the vendor's +6 dB top.
pub fn wm8978_speaker(level: u8) -> Option<u8> {
    const ZERO_DB: i32 = 57;
    const TOP: i32 = 63;
    if level == 0 {
        return None;
    }
    let half_db = -120 + 3 * (level.min(MAX_LEVEL) as i32 - 1);
    Some((ZERO_DB + half_db.div_euclid(2)).clamp(0, TOP) as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Total gain in half dB.
    fn half_db(g: CodecGain) -> i32 {
        -90 + 3 * g.output as i32 - 6 * g.mixer_cut as i32
    }

    #[test]
    fn levels_step_by_1_5_db_from_minus_60() {
        assert_eq!(codec_gain(0), None);
        for level in 1..=MAX_LEVEL {
            let g = codec_gain(level).unwrap();
            assert_eq!(half_db(g), -120 + 3 * (level as i32 - 1), "level {level}");
            assert!(g.mixer_cut <= MAX_MIXER_CUT && g.output <= 0x21, "level {level}");
        }
        assert_eq!(codec_gain(MAX_LEVEL), Some(CodecGain { mixer_cut: 0, output: 0x21 }));
    }

    #[test]
    fn wm8978_follows_the_same_db_and_stays_under_the_vendor_top() {
        assert_eq!(wm8978_speaker(0), None);
        assert_eq!(wm8978_speaker(1), Some(0));
        assert_eq!(wm8978_speaker(MAX_LEVEL - 3), Some(57));
        assert_eq!(wm8978_speaker(MAX_LEVEL), Some(61));
        assert_eq!(wm8978_speaker(u8::MAX), Some(61));
    }

    #[test]
    fn mixer_cuts_only_below_the_output_range() {
        assert_eq!(codec_gain(11), Some(CodecGain { mixer_cut: 0, output: 0 }));
        assert_eq!(codec_gain(10).unwrap().mixer_cut, 1);
    }
}
