//! USB MIDI: the keyed line as Note On and Note Off, and the messages Vail's page sends.

/// One USB MIDI event: the cable and code index, then the MIDI message.
pub type Packet = [u8; 4];

/// Vail reads note 0 as a straight key in every one of its keyer modes, so every key type works.
pub const VAIL_NOTE: u8 = 0;

pub fn note_on(note: u8) -> Packet {
    [0x09, 0x90, note, 0x7F]
}

/// A real Note Off: Vail reads any Note On, velocity 0 included, as a press.
pub fn note_off(note: u8) -> Packet {
    [0x08, 0x80, note, 0x00]
}

/// Vail's page sends Control Change 0 = 0 on channel 1 to every MIDI output when it loads.
pub fn is_vail_hello(packet: &Packet) -> bool {
    packet[1..] == [0xB0, 0x00, 0x00]
}

/// The MIDI note nearest `hz`, from 0 (8.18 Hz) to 127.
pub fn nearest_note(hz: u16) -> u8 {
    const SEMITONE: f32 = 1.059_463_1;
    const QUARTER_TONE: f32 = 1.029_302_2;
    let mut note_hz = 8.175_799;
    for note in 0..127 {
        if (hz as f32) < note_hz * QUARTER_TONE {
            return note;
        }
        note_hz *= SEMITONE;
    }
    127
}

/// The note the computer last took, so each mark gets one Note On and one Note Off.
pub struct Out {
    sounding: Option<u8>,
}

impl Out {
    pub const fn new() -> Out {
        Out { sounding: None }
    }

    /// The packet that moves the computer toward `note` sounding while `keyed`, if any. A note
    /// change while sounding ends the old note first.
    pub fn next(&self, keyed: bool, note: u8) -> Option<Packet> {
        match self.sounding {
            Some(n) if !keyed || n != note => Some(note_off(n)),
            None if keyed => Some(note_on(note)),
            _ => None,
        }
    }

    /// The computer took `packet`, one that `next` gave.
    pub fn sent(&mut self, packet: Packet) {
        self.sounding = (packet[1] == 0x90).then_some(packet[2]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nearest_note_rounds_to_the_closer_semitone() {
        for (hz, note) in [(440, 69), (600, 74), (700, 77), (880, 81), (8, 0), (20_000, 127), (452, 69), (453, 70)] {
            assert_eq!(nearest_note(hz), note, "{hz}");
        }
    }

    #[test]
    fn a_mark_is_one_note_on_and_one_note_off() {
        let mut out = Out::new();
        assert_eq!(out.next(false, 74), None);
        let on = out.next(true, 74).unwrap();
        assert_eq!(on, [0x09, 0x90, 74, 0x7F]);
        assert_eq!(out.next(true, 74), Some(on), "unsent until the computer takes it");
        out.sent(on);
        assert_eq!(out.next(true, 74), None);
        let off = out.next(false, 74).unwrap();
        assert_eq!(off, [0x08, 0x80, 74, 0x00]);
        out.sent(off);
        assert_eq!(out.next(false, 74), None);
    }

    #[test]
    fn a_note_change_ends_the_old_note_first() {
        let mut out = Out::new();
        out.sent(out.next(true, 74).unwrap());
        let off = out.next(true, VAIL_NOTE).unwrap();
        assert_eq!(off, note_off(74));
        out.sent(off);
        assert_eq!(out.next(true, VAIL_NOTE), Some(note_on(VAIL_NOTE)));
    }

    #[test]
    fn vail_hello_is_cc0_zero_on_channel_1() {
        assert!(is_vail_hello(&[0x0B, 0xB0, 0, 0]));
        assert!(!is_vail_hello(&[0x0B, 0xB1, 0, 0]));
        assert!(!is_vail_hello(&[0x0B, 0xB0, 0, 1]));
    }
}
