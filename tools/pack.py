#!/usr/bin/env python3
"""Builds the content pack F000400.bin (format in `logic/src/pack.rs`): the characters in
tools/words/chars.txt and the words in tools/words/ham.txt and tools/words/common.txt, each with
a clip of the Kokoro voice af_heart at 8789 Hz. A ham word with a meaning gets a second clip
that says the word, then its meaning. The least common words are left out until the pack fits
in one update file.

Clips are kept in a cache folder by what they say and how they are made, so a rebuild only
speaks new or changed words.

Needs what tools/voiceclips.py needs for Kokoro: Python 3.12, numpy, soundfile, ffmpeg,
espeak-ng, kokoro-onnx, kokoro-v1.0.onnx and voices-v1.0.bin.

    python tools/pack.py --models MODELS --out firmware/target/pack
    python tools/pack.py --no-voice --out firmware/target/pack
    python tools/pack.py --fixture logic/fixtures/pack.bin
"""

import argparse
import hashlib
import importlib.metadata
import re
import struct
import sys
import zlib
from pathlib import Path

import numpy as np

MAGIC = b"PocketCW"
FORMAT = 2
WORD_TEXT = 18
NO_CLIP = 0xFFFFFFFF
FLAGS = {"char": 2, "ham": 1, "common": 0}
VOICE = "af_heart"
FACTOR = 2
# The speech buffer in firmware/src/speech.rs.
CLIP_MAX = 16384
# The largest update file: 1022 clusters of 8 KB (docs/dev/hardware/flash-layout.md).
FILE_MAX = 8_372_224
WORD_ENTRY = 28
CLIP_START = 2
# Change when the shaping, resampling or encoding of clips changes, so the cache is rebuilt.
CLIP_RECIPE = 2
WORDS = Path(__file__).parent / "words"
# What Morse can send: letters, digits, and prosigns of two letters in angle brackets.
SENDABLE = re.compile(r"(?:[A-Z0-9]|<[A-Z]{2}>)+")
CHARS = set("ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789/?.,=:'-()\"@+")


def read_list(name: str, kind: str) -> list[tuple[str, str, str | None, str]]:
    """(text, said, meaning, kind) for each line of a tab-separated list. Said is the text in
    lower case when the line has no tab; meaning is None when it has no second tab."""
    entries = []
    for line in (WORDS / name).read_text().splitlines():
        if line and not line.startswith("#"):
            text, said, meaning = (line.split("\t") + [None, None])[:3]
            entries.append((text, said or text.lower(), meaning, kind))
    return entries


def read_words() -> list[tuple[str, str, str | None, str]]:
    """(text, said, meaning, kind) for each entry: characters, ham words, then common words
    that are not also ham words."""
    chars = read_list("chars.txt", "char")
    ham = read_list("ham.txt", "ham")
    ham_texts = {e[0] for e in ham}
    common = [e for e in read_list("common.txt", "common") if e[0] not in ham_texts]
    texts = lambda entries: {e[0] for e in entries}
    if texts(chars) != CHARS or len(chars) != len(CHARS):
        sys.exit(f"chars.txt must hold each of {''.join(sorted(CHARS))} once")
    for text, _, _, _ in ham + common:
        if not SENDABLE.fullmatch(text) or len(text) > WORD_TEXT:
            sys.exit(f"cannot send {text!r} in a word entry")
    if len(texts(ham + common)) != len(ham + common):
        sys.exit("a word is listed twice")
    return chars + ham + common


def build(words: list[tuple[str, str, bytes | None, bytes | None]], factor: int = FACTOR) -> bytes:
    """The pack for (text, kind, clip, clip with meaning) entries, with clips at the output rate
    divided by `factor`."""
    entries = bytearray(struct.pack("<HH", len(words), 0))
    clips = bytearray(struct.pack("<B3x", factor))

    def add(text: str, clip: bytes | None) -> int:
        if clip is None:
            return NO_CLIP
        if len(clip) > CLIP_MAX:
            sys.exit(f"a clip for {text} is {len(clip)} bytes, more than {CLIP_MAX}")
        offset = len(clips)
        clips.extend(struct.pack("<H", len(clip)) + clip)
        return offset

    for text, kind, clip, with_meaning in words:
        encoded = text.encode()
        entries += struct.pack("<BB", len(encoded), FLAGS[kind]) + encoded.ljust(WORD_TEXT, b"\0")
        entries += struct.pack("<II", add(text, clip), add(text, with_meaning))
    sections = [(b"WORD", bytes(entries)), (b"CLIP", bytes(clips))]
    header_len = 12 + 16 * len(sections) + 4
    table = bytearray(MAGIC + struct.pack("<BBH", FORMAT, len(sections), 0))
    offset = header_len
    for kind, data in sections:
        table += kind + struct.pack("<III", offset, len(data), zlib.crc32(data))
        offset += len(data)
    return bytes(table) + struct.pack("<I", zlib.crc32(table)) + b"".join(data for _, data in sections)


class Voice:
    """Speaks clips through Kokoro, keeping each in `cache` under a key of everything that
    shapes it."""

    def __init__(self, models: Path, cache: Path):
        self.models = models
        self.cache = cache
        self.speaker = None
        model = models / "kokoro-v1.0.onnx"
        self.recipe = f"{CLIP_RECIPE}|{importlib.metadata.version('kokoro-onnx')}|{model.stat().st_size}|{VOICE}|{FACTOR}"
        cache.mkdir(parents=True, exist_ok=True)

    def clip(self, spoken: str) -> tuple[bytes, bool]:
        """The clip, and whether it was spoken now rather than taken from the cache."""
        path = self.cache / (hashlib.sha256(f"{self.recipe}|{spoken}".encode()).hexdigest()[:24] + ".adpcm")
        if path.exists():
            return path.read_bytes(), False
        import audioop

        from voiceclips import OUTPUT_HZ, Speaker, resample, shape, undo_upsampling

        if self.speaker is None:
            self.speaker = Speaker("kokoro", self.models)
        samples, hz = self.speaker.speak(spoken)
        pcm = np.frombuffer(resample(shape(samples, hz), hz, OUTPUT_HZ // FACTOR), "<i2").astype(np.float64)
        pcm = np.clip(undo_upsampling(pcm, FACTOR), -32768, 32767).astype("<i2").tobytes()
        clip = audioop.lin2adpcm(pcm, 2, None)[0]
        path.write_bytes(clip)
        return clip, True


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--models", type=Path, default=Path("."), help="folder with the Kokoro model files")
    parser.add_argument("--out", type=Path, help="folder for F000400.bin")
    parser.add_argument("--cache", type=Path, default=Path("firmware/target/clip-cache"), help="folder that keeps spoken clips")
    parser.add_argument("--no-voice", action="store_true", help="words only, with no clips")
    parser.add_argument("--fixture", type=Path, help="write the logic crate's test pack here instead")
    args = parser.parse_args()

    if args.fixture:
        args.fixture.parent.mkdir(parents=True, exist_ok=True)
        fixture = [
            ("K", "char", bytes([5]), None),
            ("CQ", "ham", bytes([1, 2, 3]), bytes([4, 4])),
            ("<BK>", "ham", None, None),
            ("THE", "common", bytes([9, 8]), None),
        ]
        args.fixture.write_bytes(build(fixture))
        return
    if not args.out:
        parser.error("--out is required")
    words = read_words()
    clips = [(None, None)] * len(words)
    if not args.no_voice:
        voice = Voice(args.models, args.cache)
        spoken_now = taken = 0
        for i, (text, said, meaning, kind) in enumerate(words):
            # Kokoro rushes a lone letter; a period makes it the end of a sentence.
            phrases = [f"{said}." if kind == "char" else said, f"{said}. {meaning}" if meaning else None]
            pair = []
            for phrase in phrases:
                clip, new = voice.clip(phrase) if phrase else (None, False)
                pair.append(clip)
                spoken_now += new
                taken += bool(phrase) and not new
                if new and spoken_now % 100 == 0:
                    print(f"spoken {spoken_now}, at {i}/{len(words)} {text}", flush=True)
            clips[i] = tuple(pair)
        print(f"spoke {spoken_now} clips, took {taken} from the cache")
    entries = [(text, kind, clip, with_meaning) for (text, _, _, kind), (clip, with_meaning) in zip(words, clips)]
    common = sum(1 for e in entries if e[1] == "common")
    pack = build(entries)
    over = len(pack) - FILE_MAX
    while over > 0 and entries[-1][1] == "common":
        over -= WORD_ENTRY + sum(CLIP_START + len(c) for c in entries.pop()[2:] if c)
    if over > 0:
        sys.exit(f"the pack is {over} bytes over {FILE_MAX} with no common words")
    pack = build(entries)
    args.out.mkdir(parents=True, exist_ok=True)
    path = args.out / "F000400.bin"
    path.write_bytes(pack)
    counts = {kind: sum(1 for e in entries if e[1] == kind) for kind in FLAGS}
    print(f"{path}: {len(pack)} bytes, {counts['char']} characters, {counts['ham']} ham and {counts['common']} of {common} common words")


if __name__ == "__main__":
    main()
