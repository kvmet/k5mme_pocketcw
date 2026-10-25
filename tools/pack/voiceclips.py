#!/usr/bin/env python3
"""Builds the voice probe's clips for one voice: speaks each word, trims and normalizes it,
resamples it to 8789 Hz and 5859 Hz (the 17,578 Hz sample rate divided by 2 and 3), and
encodes it as IMA ADPCM. Writes the clips and `clips.rs` for `firmware/build.rs`.

Needs the voice tools tools/pack/setup.py sets up. Piper also needs piper-tts and
en_US-ljspeech-high.onnx, which setup.py does not install.

    tools/pack/.venv/bin/python tools/pack/voiceclips.py kokoro --models tools/pack/models --out firmware/target/voice
    VOICE_CLIPS=firmware/target/voice sh tools/mkimage.sh voiceprobe
"""

import argparse
import audioop
import random
import re
import subprocess
import tempfile
from pathlib import Path

import numpy as np
import soundfile

OUTPUT_HZ = 17578
FACTORS = (2, 3)
WORDS_PER_RATE = 20
VOICES = ("kokoro", "piper", "espeak")
# The engines speak at their own levels; every clip is scaled to this peak.
PEAK = 0.9
# Speech below this fraction of the peak counts as silence at the ends.
TRIM = 0.02
LEAD_S = 0.01
TAIL_S = 0.04

# Common words, so a miss means the voice is unclear, not the word unknown. Each voice and rate
# gets its own 20, so a word heard once is not easier the next time.
WORDS = """
apple bank bird black boat bread bring brown chair cheap child clean clock cloud coast cold count
corn cream dark dog door duck earth egg dream drink drive fast field fire fish flag floor food fork friend
frog game girl glass gold grass green hand happy heavy hill horse hot house jump key king kite
lake lamp large light lion long lunch map milk money moon mouse music name night north nurse
ocean orange paint paper park pencil phone pink plant plate queen quick rain red river road
rock salt sand ship shoe short sing small smile snake snow soft south spoon star stone storm
street sugar summer table tiger train tree truck under water whale wheel white window winter
""".split()

# Kokoro was trained on phonemes in misaki's form. This is misaki's espeak fallback for US
# English and espeak-ng 1.52 or later (hexgrad/misaki, misaki/espeak.py): each pair in order,
# the longest first, then the US pairs.
MISAKI = sorted(
    [
        ("ʔˌn\u0329", "ʔn"), ("ʔn\u0329", "ʔn"), ("a^ɪ", "I"), ("a^ʊ", "W"), ("d^ʒ", "ʤ"), ("e^ɪ", "A"),
        ("t^ʃ", "ʧ"), ("ɔ^ɪ", "Y"), ("ə^l", "ᵊl"), ("ʲo", "jo"), ("ʲə", "jə"), ("e", "A"), ("ʲ", ""),
        ("ɚ", "əɹ"), ("r", "ɹ"), ("x", "k"), ("ç", "k"), ("ɐ", "ə"), ("ɬ", "l"), ("\u0303", ""),
    ],
    key=lambda pair: -len(pair[0]),
)
MISAKI_US = [("o^ʊ", "O"), ("ɜːɹ", "ɜɹ"), ("ɜː", "ɜɹ"), ("ɪə", "iə"), ("ː", "")]


def misaki_form(phonemes: str) -> str:
    """espeak-ng phonemes, with ^ tying the parts of a sound, as misaki writes them."""
    for espeak, misaki in MISAKI:
        phonemes = phonemes.replace(espeak, misaki)
    phonemes = re.sub("(\\S)\u0329", "ᵊ\\1", phonemes).replace("\u0329", "")
    for espeak, misaki in MISAKI_US:
        phonemes = phonemes.replace(espeak, misaki)
    return phonemes.replace("ɾ", "T").replace("ʔ", "t").replace("^", "")


def words_for(voice: str) -> list[tuple[int, str]]:
    """(factor, word) pairs for `voice`: a fixed shuffle split into one slice per voice and rate."""
    words = WORDS[:]
    random.Random(388).shuffle(words)
    per_voice = WORDS_PER_RATE * len(FACTORS)
    mine = words[VOICES.index(voice) * per_voice:][:per_voice]
    return [(factor, word) for i, factor in enumerate(FACTORS) for word in mine[i * WORDS_PER_RATE:][:WORDS_PER_RATE]]


def espeak_data() -> Path:
    """The installed espeak-ng's data folder. Kokoro and Piper phonemize with it: the copies
    their wheels bundle fail to load on macOS."""
    version = subprocess.run(["espeak-ng", "--version"], capture_output=True, text=True, check=True).stdout
    return Path(version.split("Data at:")[1].strip())


class Speaker:
    def __init__(self, engine: str, models: Path, voice: str = "af_heart"):
        """`engine` is one of VOICES; `voice` picks among Kokoro's voices."""
        self.kind = engine
        self.voice = voice
        if engine == "kokoro":
            from kokoro_onnx import EspeakConfig, Kokoro

            data = espeak_data()
            library = next((data.parent.parent / "lib").glob("libespeak-ng.*[ds][yo]*"))
            config = EspeakConfig(lib_path=str(library), data_path=str(data))
            self.engine = Kokoro(str(models / "kokoro-v1.0.onnx"), str(models / "voices-v1.0.bin"), espeak_config=config)
            from phonemizer.backend import EspeakBackend

            self.espeak = EspeakBackend("en-us", preserve_punctuation=True, with_stress=True, tie="^")
            self.name = f"KOKORO {voice.upper()}"
        elif engine == "piper":
            from piper import PiperVoice

            self.engine = PiperVoice.load(str(models / "en_US-ljspeech-high.onnx"), espeak_data_dir=espeak_data())
            self.name = "PIPER LJSPEECH HIGH"
        else:
            self.name = "ESPEAK-NG EN-US"

    def phonemes(self, text: str) -> str:
        """What Kokoro is given for `text`. espeak gives nothing for blank text."""
        phonemized = self.espeak.phonemize([text])
        return misaki_form(phonemized[0].strip()) if phonemized else ""

    def speak_phonemes(self, phonemes: str, speed: float = 1.0) -> tuple[np.ndarray, int]:
        """Kokoro's mono float samples and their rate. Kokoro drops what is not in its
        vocabulary without a word, so that is an error here."""
        unknown = sorted(set(phonemes) - set(self.engine.tokenizer.vocab))
        if unknown:
            raise ValueError(f"not Kokoro phonemes: {' '.join(unknown)}")
        samples, hz = self.engine.create(phonemes, voice=self.voice, speed=speed, lang="en-us", is_phonemes=True)
        return np.asarray(samples, dtype=np.float32), hz

    def speak(self, word: str, speed: float = 1.0) -> tuple[np.ndarray, int]:
        """Mono float samples and their rate. Only Kokoro takes `speed`."""
        if self.kind == "kokoro":
            return self.speak_phonemes(self.phonemes(word), speed)
        if self.kind == "piper":
            chunks = list(self.engine.synthesize(word))
            return np.concatenate([c.audio_float_array for c in chunks]).astype(np.float32), chunks[0].sample_rate
        with tempfile.NamedTemporaryFile(suffix=".wav") as wav:
            subprocess.run(["espeak-ng", "-v", "en-us", "-w", wav.name, word], check=True)
            samples, hz = soundfile.read(wav.name, dtype="float32")
        return samples, hz


def shape(samples: np.ndarray, hz: int) -> np.ndarray:
    """Trims silence at both ends, scales to PEAK, and pads a short lead and tail."""
    if samples.ndim > 1:
        samples = samples.mean(axis=1)
    loud = np.flatnonzero(np.abs(samples) >= TRIM * np.abs(samples).max())
    samples = samples[loud[0]:loud[-1] + 1]
    samples = samples * (PEAK / np.abs(samples).max())
    return np.concatenate([np.zeros(int(LEAD_S * hz)), samples, np.zeros(int(TAIL_S * hz))]).astype(np.float32)


def resample(samples: np.ndarray, hz: int, to_hz: int) -> bytes:
    """16-bit little endian samples at `to_hz`."""
    result = subprocess.run(
        ["ffmpeg", "-v", "error", "-f", "f32le", "-ar", str(hz), "-ac", "1", "-i", "-", "-ar", str(to_hz), "-f", "s16le", "-"],
        input=samples.tobytes(),
        capture_output=True,
        check=True,
    )
    return result.stdout


def undo_upsampling(samples: np.ndarray, factor: int) -> np.ndarray:
    """Divides each frequency by the gain of straight-line upsampling by `factor`, which is
    -7 dB at the top of the band for 3 and -6 dB for 2, and keeps the peak."""
    f = np.fft.rfftfreq(len(samples))
    gain = np.ones_like(f)
    nonzero = f > 0
    gain[nonzero] = (np.sin(np.pi * f[nonzero]) / (factor * np.sin(np.pi * f[nonzero] / factor))) ** 2
    boosted = np.fft.irfft(np.fft.rfft(samples) / gain, len(samples))
    return boosted * (np.abs(samples).max() / np.abs(boosted).max())


def upsample(samples: np.ndarray, factor: int) -> np.ndarray:
    """What `Upsample` in logic/src/adpcm.rs gives: a line from each sample to the next, one
    sample late."""
    to = samples.astype(np.int64)
    start = np.concatenate([[0], to[:-1]])
    phase = np.arange(factor)
    return (start[:, None] + np.trunc((to - start)[:, None] * phase / factor)).astype(np.int16).ravel()


# A spelled word's pauses between its letters are cut to this. Each letter ends in a period,
# and Kokoro paused up to 225 ms after one in the word lists; a stop such as T keeps its silent
# build up.
GAP_S = 0.05
GAP_FRAME_S = 0.005
# Under this, in dB of the peak, a frame is part of a pause.
GAP_DB = -40.0


def close_gaps(samples: np.ndarray, hz: int) -> np.ndarray:
    """`samples` with each pause inside them longer than GAP_S cut to GAP_S, from its middle."""
    n = int(GAP_FRAME_S * hz)
    frames = len(samples) // n
    power = (samples[: frames * n].reshape(frames, n) ** 2).mean(axis=1)
    quiet = power < np.abs(samples).max() ** 2 * 10 ** (GAP_DB / 10)
    longest = int(GAP_S / GAP_FRAME_S)
    keep = np.ones(len(samples), dtype=bool)
    start = None
    for frame, is_quiet in enumerate([*quiet, False]):
        if is_quiet and start is None:
            start = frame
        elif not is_quiet and start is not None:
            # A pause at either end is the trim's to cut, not this.
            if start > 0 and frame < frames and frame - start > longest:
                keep[(start + longest // 2) * n : (frame - (longest - longest // 2)) * n] = False
            start = None
    return samples[keep]


STRAY_FRAME_S = 0.01
VOICED_HZ = (100, 1500)
HISS_HZ = 3500
# A frame is silence when its voiced and hiss power is under this, in dB of the peak squared times
# the frame's samples: about 58 dB under a sine at the peak, for 24 kHz speech.
STRAY_SILENCE_DB = -45.0
# Kokoro's stray vowel before an s or f ran 20 to 90 ms; FIST, with no hiss at its f, voiced 260.
STRAY_MIN_S = 0.02
STRAY_MAX_S = 0.1
STRAY_FADE_S = 0.005


def cut_stray_vowel(samples: np.ndarray, hz: int) -> np.ndarray:
    """`samples` from their first hiss on, faded in, when STRAY_MIN_S to STRAY_MAX_S of voiced
    sound comes before it. A hiss frame has more power above HISS_HZ than in VOICED_HZ."""
    n = int(STRAY_FRAME_S * hz)
    frames = len(samples) // n
    power = np.abs(np.fft.rfft(samples[: frames * n].reshape(frames, n) * np.hanning(n), axis=1)) ** 2
    freqs = np.fft.rfftfreq(n, 1 / hz)
    voiced = power[:, (freqs > VOICED_HZ[0]) & (freqs < VOICED_HZ[1])].sum(axis=1)
    hiss = power[:, freqs > HISS_HZ].sum(axis=1)
    loud = voiced + hiss >= np.abs(samples).max() ** 2 * n * 10 ** (STRAY_SILENCE_DB / 10)
    hissing = np.flatnonzero(loud & (hiss > voiced))
    if not len(hissing):
        return samples
    vowel = hissing[0] - np.flatnonzero(loud)[0]
    if not round(STRAY_MIN_S / STRAY_FRAME_S) <= vowel <= round(STRAY_MAX_S / STRAY_FRAME_S):
        return samples
    kept = samples[hissing[0] * n :].copy()
    fade = min(len(kept), int(STRAY_FADE_S * hz))
    kept[:fade] *= np.linspace(0, 1, fade, endpoint=False)
    return kept


FRAME_S = 0.02
# 20 ms frames more than this far under the loudest one are pauses, not speech.
ACTIVE_RANGE_DB = 30.0
# The limiter's ceiling, under `PEAK`: ADPCM overshot dense peaks by up to 16% in a test of the
# old clips, and with a ceiling of `PEAK` one leveled clip in 15 reached full scale. The synth
# plays this ceiling at the sidetone's peak, so `CLIP_CEILING` in logic/src/synth.rs matches it.
LIMIT_PEAK = 0.8
# The sine sidetone's RMS in dB of a clip's full scale. A square one is 0.9 dB louder.
SIDETONE_DB = 20 * np.log10(LIMIT_PEAK / np.sqrt(2))
# Speech level in dB of a clip's full scale.
SPEECH_DB = SIDETONE_DB - 7.0
# The level is set within this. 8 passes left one clip in five short of it; 40 left none.
LEVEL_TOLERANCE_DB = 0.25
LEVEL_PASSES = 40
LIMIT_BLOCK_S = 0.001
# dB a block that the limiter's gain falls before a peak, and comes back after it. At 8789 Hz a
# block is 0.91 ms, so 12 dB takes 3.6 ms and 55 ms. The release is slow next to a 5 ms cycle of
# the voice, so the gain does not follow the wave between peaks.
LIMIT_ATTACK_DB = 3.0
LIMIT_RELEASE_DB = 0.2


def active_level(samples: np.ndarray, hz: int) -> float:
    """The level of the speech in `samples`, in dB of full scale 1.0: the power of its 20 ms
    frames, leaving out pauses."""
    n = int(FRAME_S * hz)
    power = (samples[: len(samples) // n * n].reshape(-1, n) ** 2).mean(axis=1)
    return 10 * np.log10(power[power >= power.max() * 10 ** (-ACTIVE_RANGE_DB / 10)].mean())


def limit(samples: np.ndarray, hz: int, ceiling: float) -> np.ndarray:
    """Turns `samples` down where they pass `ceiling`, with the gain in dB set at block starts
    about 1 ms apart and a straight line between."""
    n = max(1, int(LIMIT_BLOCK_S * hz))
    blocks = -(-len(samples) // n)
    peaks = np.abs(np.concatenate([samples, np.zeros(blocks * n - len(samples))])).reshape(blocks, n).max(axis=1)
    need = 20 * np.log10(np.minimum(1.0, ceiling / np.maximum(peaks, 1e-9)))
    # A block lies between its own start and the next, so both starts take what it needs.
    gain = np.minimum(np.concatenate([need, [0.0]]), np.concatenate([[0.0], need]))
    for b in range(blocks - 1, -1, -1):
        gain[b] = min(gain[b], gain[b + 1] + LIMIT_ATTACK_DB)
    for b in range(1, blocks + 1):
        gain[b] = min(gain[b], gain[b - 1] + LIMIT_RELEASE_DB)
    return samples * 10 ** (np.interp(np.arange(len(samples)) / n, np.arange(blocks + 1), gain) / 20)


def level(samples: np.ndarray, hz: int) -> np.ndarray:
    """Sets the speech to `SPEECH_DB` with its peaks under `LIMIT_PEAK`: speech has short peaks
    far above its level, so a clip scaled by its peak alone sounds much quieter than the
    sidetone. Each pass of the limiter lowers the level, and the next pass makes up for it."""
    for _ in range(LEVEL_PASSES):
        short = SPEECH_DB - active_level(samples, hz)
        if abs(short) < LEVEL_TOLERANCE_DB:
            break
        samples = limit(samples * 10 ** (short / 20), hz, LIMIT_PEAK)
    return limit(samples, hz, LIMIT_PEAK)


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("voice", choices=VOICES)
    parser.add_argument("--models", type=Path, default=Path("."), help="folder with the kokoro or piper model files")
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()

    speaker = Speaker(args.voice, args.models)
    args.out.mkdir(parents=True, exist_ok=True)
    lines = [f'const VOICE: &str = "{speaker.name}";', "const CLIPS: &[Clip] = &["]
    total = 0
    for factor, word in words_for(args.voice):
        samples, hz = speaker.speak(word)
        pcm = resample(shape(samples, hz), hz, OUTPUT_HZ // factor)
        encoded, _ = audioop.lin2adpcm(pcm, 2, None)
        path = (args.out / f"{word}-{factor}.adpcm").resolve()
        path.write_bytes(encoded)
        total += len(encoded)
        lines.append(f'    Clip {{ word: "{word.upper()}", factor: {factor}, data: include_bytes!("{path}") }},')
    lines.append("];")
    (args.out / "clips.rs").write_text("\n".join(lines) + "\n")
    print(f"{speaker.name}: {len(lines) - 3} clips, {total} bytes in {args.out}")


if __name__ == "__main__":
    main()
