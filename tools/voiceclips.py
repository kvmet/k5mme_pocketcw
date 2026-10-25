#!/usr/bin/env python3
"""Builds the voice probe's clips for one voice: speaks each word, trims and normalizes it,
resamples it to 8789 Hz and 5859 Hz (the 17,578 Hz sample rate divided by 2 and 3), and
encodes it as IMA ADPCM. Writes the clips and `clips.rs` for `firmware/build.rs`.

Needs Python 3.12 (for `audioop`), numpy, soundfile, ffmpeg, espeak-ng on the path, and for
the neural voices piper-tts with en_US-ljspeech-high, or kokoro-onnx with kokoro-v1.0.onnx
and voices-v1.0.bin.

    python tools/voiceclips.py kokoro --models DIR --out firmware/target/voice
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
    def __init__(self, voice: str, models: Path):
        self.voice = voice
        if voice == "kokoro":
            from kokoro_onnx import EspeakConfig, Kokoro

            data = espeak_data()
            library = next((data.parent.parent / "lib").glob("libespeak-ng.*[ds][yo]*"))
            config = EspeakConfig(lib_path=str(library), data_path=str(data))
            self.engine = Kokoro(str(models / "kokoro-v1.0.onnx"), str(models / "voices-v1.0.bin"), espeak_config=config)
            from phonemizer.backend import EspeakBackend

            self.espeak = EspeakBackend("en-us", preserve_punctuation=True, with_stress=True, tie="^")
            self.name = "KOKORO AF_HEART"
        elif voice == "piper":
            from piper import PiperVoice

            self.engine = PiperVoice.load(str(models / "en_US-ljspeech-high.onnx"), espeak_data_dir=espeak_data())
            self.name = "PIPER LJSPEECH HIGH"
        else:
            self.name = "ESPEAK-NG EN-US"

    def speak(self, word: str, speed: float = 1.0) -> tuple[np.ndarray, int]:
        """Mono float samples and their rate. Only Kokoro takes `speed`."""
        if self.voice == "kokoro":
            phonemes = misaki_form(self.espeak.phonemize([word])[0].strip())
            samples, hz = self.engine.create(phonemes, voice="af_heart", speed=speed, lang="en-us", is_phonemes=True)
            return np.asarray(samples, dtype=np.float32), hz
        if self.voice == "piper":
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
