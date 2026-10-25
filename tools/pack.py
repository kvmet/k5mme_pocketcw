#!/usr/bin/env python3
"""Builds the content pack F000400.bin (format in `logic/src/pack.rs`): the characters in
tools/words/chars.txt and the words in tools/words/ham.txt and tools/words/common.txt, each with
a clip of the Kokoro voice af_heart at 5859 Hz.

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

MAGIC = b"PocketCW"
FORMAT = 2
WORD_TEXT = 18
NO_CLIP = 0xFFFFFFFF
FLAGS = {"char": 2, "ham": 1, "common": 0}
VOICE = "af_heart"
FACTOR = 3
CLIP_MAX = 16384
# Change when the shaping, resampling or encoding of clips changes, so the cache is rebuilt.
CLIP_RECIPE = 1
WORDS = Path(__file__).parent / "words"
# What Morse can send: letters, digits, and prosigns of two letters in angle brackets.
SENDABLE = re.compile(r"(?:[A-Z0-9]|<[A-Z]{2}>)+")
CHARS = set("ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789/?.,=:'-()\"@+")


def read_list(name: str) -> list[tuple[str, str]]:
    """(text, spoken) for each line of a tab-separated list; spoken is the text in lower case
    when the line has no tab."""
    pairs = []
    for line in (WORDS / name).read_text().splitlines():
        if line and not line.startswith("#"):
            text, _, spoken = line.partition("\t")
            pairs.append((text, spoken or text.lower()))
    return pairs


def read_words() -> list[tuple[str, str, str]]:
    """(text, spoken, kind) for each entry: characters, ham words, then common words that are
    not also ham words."""
    chars = [(text, spoken, "char") for text, spoken in read_list("chars.txt")]
    ham = [(text, spoken, "ham") for text, spoken in read_list("ham.txt")]
    ham_texts = {text for text, _, _ in ham}
    common = [(text, spoken, "common") for text, spoken in read_list("common.txt") if text not in ham_texts]
    for text, _, _ in chars:
        if text not in CHARS:
            sys.exit(f"cannot send the character {text!r}")
    if {text for text, _, _ in chars} != CHARS:
        sys.exit(f"chars.txt misses {''.join(sorted(CHARS - {t for t, _, _ in chars}))}")
    for text, _, _ in ham + common:
        if not SENDABLE.fullmatch(text) or len(text) > WORD_TEXT:
            sys.exit(f"cannot send {text!r} in a word entry")
    for group in (chars, ham + common):
        if len({text for text, _, _ in group}) != len(group):
            sys.exit("an entry is listed twice")
    return chars + ham + common


def build(words: list[tuple[str, str, bytes | None]]) -> bytes:
    """The pack for (text, kind, clip) entries."""
    entries = bytearray(struct.pack("<HH", len(words), 0))
    clips = bytearray(struct.pack("<B3x", FACTOR))
    for text, kind, clip in words:
        offset = NO_CLIP
        if clip is not None:
            if len(clip) > CLIP_MAX:
                sys.exit(f"the clip for {text} is {len(clip)} bytes, more than {CLIP_MAX}")
            offset = len(clips)
            clips += struct.pack("<H", len(clip)) + clip
        encoded = text.encode()
        entries += struct.pack("<BB", len(encoded), FLAGS[kind]) + encoded.ljust(WORD_TEXT, b"\0")
        entries += struct.pack("<I", offset)
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

        from voiceclips import OUTPUT_HZ, Speaker, resample, shape

        if self.speaker is None:
            self.speaker = Speaker("kokoro", self.models)
        samples, hz = self.speaker.speak(spoken)
        clip = audioop.lin2adpcm(resample(shape(samples, hz), hz, OUTPUT_HZ // FACTOR), 2, None)[0]
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
        fixture = [("K", "char", bytes([5])), ("CQ", "ham", bytes([1, 2, 3])), ("<BK>", "ham", None), ("THE", "common", bytes([9, 8]))]
        args.fixture.write_bytes(build(fixture))
        return
    if not args.out:
        parser.error("--out is required")
    words = read_words()
    clips = [None] * len(words)
    if not args.no_voice:
        voice = Voice(args.models, args.cache)
        spoken_now = 0
        for i, (text, spoken, _) in enumerate(words):
            clips[i], new = voice.clip(spoken)
            spoken_now += new
            if new and spoken_now % 100 == 0:
                print(f"spoken {spoken_now}, at {i}/{len(words)} {text}", flush=True)
        print(f"spoke {spoken_now} clips, took {len(words) - spoken_now} from the cache")
    pack = build([(text, kind, clip) for (text, _, kind), clip in zip(words, clips)])
    args.out.mkdir(parents=True, exist_ok=True)
    path = args.out / "F000400.bin"
    path.write_bytes(pack)
    counts = {kind: sum(1 for _, _, k in words if k == kind) for kind in FLAGS}
    print(f"{path}: {len(pack)} bytes, {counts['char']} characters, {counts['ham']} ham and {counts['common']} common words")


if __name__ == "__main__":
    main()
