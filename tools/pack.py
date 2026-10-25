#!/usr/bin/env python3
"""Builds the content pack F000400.bin (format in `logic/src/pack.rs`): the words in
tools/words/ham.txt and tools/words/common.txt, each with a clip of the Kokoro voice af_heart
at 5859 Hz.

Needs what tools/voiceclips.py needs for Kokoro: Python 3.12, numpy, soundfile, ffmpeg,
espeak-ng, kokoro-onnx, kokoro-v1.0.onnx and voices-v1.0.bin.

    python tools/pack.py --models MODELS --out firmware/target/pack
    python tools/pack.py --no-voice --out firmware/target/pack
    python tools/pack.py --fixture logic/fixtures/pack.bin
"""

import argparse
import re
import struct
import sys
import zlib
from pathlib import Path

MAGIC = b"PocketCW"
FORMAT = 2
WORD_TEXT = 18
NO_CLIP = 0xFFFFFFFF
FLAG_HAM = 1
FACTOR = 3
CLIP_MAX = 16384
WORDS = Path(__file__).parent / "words"
# What Morse can send: letters, digits, and prosigns of two letters in angle brackets.
SENDABLE = re.compile(r"(?:[A-Z0-9]|<[A-Z]{2}>)+")


def read_words() -> list[tuple[str, str, bool]]:
    """(text, spoken, ham) for each word: ham words, then common words that are not also ham."""
    words = []
    for line in (WORDS / "ham.txt").read_text().splitlines():
        if line and not line.startswith("#"):
            text, spoken = line.split("\t")
            words.append((text, spoken, True))
    ham = {text for text, _, _ in words}
    for line in (WORDS / "common.txt").read_text().splitlines():
        if line and not line.startswith("#") and line not in ham:
            words.append((line, line.lower(), False))
    for text, _, _ in words:
        if not SENDABLE.fullmatch(text) or len(text) > WORD_TEXT:
            sys.exit(f"cannot send {text!r} in a word entry")
    if len({text for text, _, _ in words}) != len(words):
        sys.exit("a word is listed twice")
    return words


def build(words: list[tuple[str, bool, bytes | None]]) -> bytes:
    """The pack for (text, ham, clip) entries."""
    entries = bytearray(struct.pack("<HH", len(words), 0))
    clips = bytearray(struct.pack("<B3x", FACTOR))
    for text, ham, clip in words:
        offset = NO_CLIP
        if clip is not None:
            if len(clip) > CLIP_MAX:
                sys.exit(f"the clip for {text} is {len(clip)} bytes, more than {CLIP_MAX}")
            offset = len(clips)
            clips += struct.pack("<H", len(clip)) + clip
        encoded = text.encode()
        entries += struct.pack("<BB", len(encoded), FLAG_HAM if ham else 0) + encoded.ljust(WORD_TEXT, b"\0")
        entries += struct.pack("<I", offset)
    sections = [(b"WORD", bytes(entries)), (b"CLIP", bytes(clips))]
    header_len = 12 + 16 * len(sections) + 4
    table = bytearray(MAGIC + struct.pack("<BBH", FORMAT, len(sections), 0))
    offset = header_len
    for kind, data in sections:
        table += kind + struct.pack("<III", offset, len(data), zlib.crc32(data))
        offset += len(data)
    return bytes(table) + struct.pack("<I", zlib.crc32(table)) + b"".join(data for _, data in sections)


def speak_all(words: list[tuple[str, str, bool]], models: Path) -> list[bytes]:
    import audioop

    from voiceclips import OUTPUT_HZ, Speaker, resample, shape

    speaker = Speaker("kokoro", models)
    clips = []
    for i, (text, spoken, _) in enumerate(words):
        samples, hz = speaker.speak(spoken)
        pcm = resample(shape(samples, hz), hz, OUTPUT_HZ // FACTOR)
        clips.append(audioop.lin2adpcm(pcm, 2, None)[0])
        if i % 100 == 0:
            print(f"{i}/{len(words)} {text}", flush=True)
    return clips


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--models", type=Path, default=Path("."), help="folder with the Kokoro model files")
    parser.add_argument("--out", type=Path, help="folder for F000400.bin")
    parser.add_argument("--no-voice", action="store_true", help="words only, with no clips")
    parser.add_argument("--fixture", type=Path, help="write the logic crate's test pack here instead")
    args = parser.parse_args()

    if args.fixture:
        args.fixture.parent.mkdir(parents=True, exist_ok=True)
        args.fixture.write_bytes(build([("CQ", True, bytes([1, 2, 3])), ("<BK>", True, None), ("THE", False, bytes([9, 8]))]))
        return
    if not args.out:
        parser.error("--out is required")
    words = read_words()
    clips = [None] * len(words) if args.no_voice else speak_all(words, args.models)
    pack = build([(text, ham, clip) for (text, _, ham), clip in zip(words, clips)])
    args.out.mkdir(parents=True, exist_ok=True)
    path = args.out / "F000400.bin"
    path.write_bytes(pack)
    ham = sum(1 for _, _, h in words if h)
    print(f"{path}: {len(pack)} bytes, {ham} ham and {len(words) - ham} common words")


if __name__ == "__main__":
    main()
