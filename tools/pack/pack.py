#!/usr/bin/env python3
"""Builds the content pack F000400.bin (format in `logic/src/pack.rs`): the characters in
tools/pack/words/chars.txt and the words in tools/pack/words/ham.txt and tools/pack/words/common.txt, each with
a clip of a voice in tools/pack/voices at 8789 Hz. A ham word with a meaning gets a second clip
that says the word, then its meaning. The least common words are left out until the pack fits
in one update file.

A voice's folder holds voice.toml (the engine, its settings and model files), fixes.txt (what
the voice says for a word, over the word lists) and review.txt (the marks tools/pack/audition.py
writes). The cache folder keeps what the voice says for each phoneme string, and each finished
clip under a key of how it is made. A rebuild speaks only new or changed words, and a change to
how clips are made redoes the clips in minutes without speaking again.

Needs the venv and models tools/pack/setup.py makes.

    tools/pack/.venv/bin/python tools/pack/pack.py --out firmware/target/pack
    python3 tools/pack/pack.py --no-voice --out firmware/target/pack
    python3 tools/pack/pack.py --fixture logic/fixtures/pack.bin
"""

import argparse
import hashlib
import importlib.metadata
import os
import re
import struct
import subprocess
import sys
import tomllib
import zlib
from pathlib import Path

MAGIC = b"PocketCW"
FORMAT = 2
WORD_TEXT = 18
NO_CLIP = 0xFFFFFFFF
FLAGS = {"char": 2, "ham": 1, "common": 0}
FACTOR = 2
# The speech buffer in firmware/src/speech.rs.
CLIP_MAX = 16384
# The largest update file: 1022 clusters of 8 KB (docs/dev/hardware/flash-layout.md).
FILE_MAX = 8_372_224
WORD_ENTRY = 28
CLIP_START = 2
# Change when how a voice speaks a phoneme string changes, so every text is spoken again.
SPEECH_RECIPE = 2
# Change when this file's making of clips from speech changes. A change to tools/pack/voiceclips.py
# makes clips again by itself.
CLIP_RECIPE = 4
HERE = Path(__file__).parent
WORDS = HERE / "words"
VOICES = HERE / "voices"
CACHE = HERE.parent.parent / "firmware" / "target" / "clip-cache"
VOICE = "kokoro-af_heart"
# A said of /phonemes/ in fixes.txt goes to the voice as it is, and so does the start of a
# clip's text in that form.
PHONEMES = re.compile(r"/([^/]+)/")
FIXED = re.compile(r"/([^/]+)/(.*)")
# A said with a lone capital letter spells a word.
SPELLED = re.compile(r"(?<!\S)[A-Z](?![^\s.,!?;:])")
ENDS = ".,!?;:"
# Phonemes with a sentence end inside them, after which the voice pauses.
SENTENCES = re.compile(r"[.!?] +\S")
# Between a word and its meaning in one clip: about the pause Kokoro makes after a sentence.
MEANING_PAUSE_S = 0.25
# What a cell of fixes.txt or review.txt may not hold: what ends a cell or a line.
BREAKS = re.compile("[\t\n\r\x0b\x0c\x1c-\x1e\x85\u2028\u2029]")
CLIPS = ("char", "word", "meaning")
REVIEW_HEADER = """# Clip marks from tools/pack/audition.py: text, clip (char, word or meaning), mark (good or
# bad), the speech key the mark was given for, and a note. A mark whose key differs from the clip's is stale.
"""
FIXES_HEADER = """# What this voice says for an entry, over the said column of tools/pack/words: text, entry
# (char or word), then a respelling, or misaki phonemes between slashes such as /bˈYzi/.
"""
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


def entry(kind: str) -> str:
    """Which entry a fix or a mark is for: A, I, K, R and W are both characters and words."""
    return "char" if kind == "char" else "word"


def read_words(fixes: dict[tuple[str, str], str] | None = None) -> list[tuple[str, str, str | None, str]]:
    """(text, said, meaning, kind) for each entry: characters, ham words, then common words
    that are not also ham words, with the said of `fixes` over the lists'."""
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
    entries = chars + ham + common
    fixes = fixes or {}
    unknown = set(fixes) - {(text, entry(kind)) for text, _, _, kind in entries}
    if unknown:
        sys.exit(f"fixes.txt has entries in no list: {' '.join(f'{text} {which}' for text, which in sorted(unknown))}")
    return [(text, fixes.get((text, entry(kind)), said), meaning, kind) for text, said, meaning, kind in entries]


def ended(text: str) -> str:
    """`text` ending in a period, unless it ends in a mark already; /phonemes/ too."""
    return text if text.rstrip("/")[-1:] in ENDS else f"{text}."


def phrases(said: str, meaning: str | None, kind: str) -> list[tuple[str, tuple[str, ...]]]:
    """(clip, parts) for each clip of an entry: what the voice says, each part spoken on its
    own. Kokoro rushes a lone letter and runs spelled letters together; as the end of a sentence
    it says each one in full, so a character and each part of a spelled word end in a period. A
    meaning clip is the word, then its meaning."""
    if kind == "char":
        return [("char", (ended(said),))]
    if not PHONEMES.fullmatch(said) and SPELLED.search(said):
        said = " ".join(ended(part) for part in said.split())
    word = [("word", (said,))]
    return word + [("meaning", (ended(said), ended(meaning)))] if meaning else word


def read_table(path: Path, columns: int) -> list[list[str]]:
    """The tab-separated rows of `path`, each padded to `columns`; none when it is missing."""
    if not path.exists():
        return []
    rows = []
    for number, line in enumerate(path.read_text().splitlines(), 1):
        if line and not line.startswith("#"):
            row = line.split("\t")
            if len(row) > columns:
                sys.exit(f"{path}:{number}: more than {columns} columns")
            rows.append(row + [""] * (columns - len(row)))
    return rows


def write_table(path: Path, header: str, rows: list[list[str]]):
    for row in rows:
        for cell in row:
            if BREAKS.search(cell):
                raise ValueError(f"{path.name}: a tab or line break in {cell!r}")
    part = path.with_suffix(".part")
    part.write_text(header + "".join("\t".join(row).rstrip("\t") + "\n" for row in rows))
    os.replace(part, path)


def read_fixes(folder: Path) -> dict[tuple[str, str], str]:
    """The said for each (text, entry)."""
    fixes = {}
    for text, which, said in read_table(folder / "fixes.txt", 3):
        if which not in ("char", "word") or not said or (text, which) in fixes:
            sys.exit(f"{folder / 'fixes.txt'}: {text} needs char or word, and one said")
        fixes[text, which] = said
    return fixes


def write_fixes(folder: Path, fixes: dict[tuple[str, str], str]):
    write_table(folder / "fixes.txt", FIXES_HEADER, [[*fix, said] for fix, said in fixes.items()])


def read_review(folder: Path) -> dict[tuple[str, str], tuple[str, str, str]]:
    """(mark, speech key, note) for each (text, clip)."""
    review = {}
    for text, clip, mark, speech_key, note in read_table(folder / "review.txt", 5):
        if clip not in CLIPS or mark not in ("good", "bad") or not speech_key:
            sys.exit(f"{folder / 'review.txt'}: bad row for {text}")
        review[text, clip] = (mark, speech_key, note)
    return review


def write_review(folder: Path, review: dict[tuple[str, str], tuple[str, str, str]]):
    write_table(folder / "review.txt", REVIEW_HEADER, [[*clip, *row] for clip, row in review.items()])


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
    """A voice in tools/pack/voices, speaking through Kokoro. `cache` keeps the speech in
    `speech/` under a key of the phonemes and what speaks them, and each clip under a key of the
    speech and what makes the clip."""

    def __init__(self, name: str, models: Path, cache: Path):
        from voiceclips import Speaker

        self.folder = VOICES / name
        settings = tomllib.loads((self.folder / "voice.toml").read_text())
        if settings["engine"] != "kokoro":
            sys.exit(f"{name}: pack.py speaks with kokoro only, not {settings['engine']}")
        for file, model in settings["models"].items():
            path = models / file
            if not path.exists() or sha256(path) != model["sha256"]:
                sys.exit(f"{path} is missing or not the file {name}/voice.toml names: run tools/pack/setup.py")
        self.speed = settings["speed"]
        self.speaker = Speaker(settings["engine"], models, settings["voice"])
        self.cache = cache
        version = lambda tool: subprocess.run([tool, "-version"], capture_output=True, text=True, check=True).stdout.splitlines()[0]
        models_sha = ",".join(model["sha256"] for model in settings["models"].values())
        self.speech_recipe = f"{SPEECH_RECIPE}|{importlib.metadata.version('kokoro-onnx')}|{importlib.metadata.version('onnxruntime')}|{settings['engine']}|{settings['voice']}|{self.speed}|{models_sha}"
        self.clip_recipe = f"{CLIP_RECIPE}|{FACTOR}|{sha256(HERE / 'voiceclips.py')}|{version('ffmpeg')}"
        (cache / "speech").mkdir(parents=True, exist_ok=True)

    def phonemes(self, text: str) -> str:
        """What the voice is given for one part of a clip."""
        fixed = FIXED.fullmatch(text)
        if fixed:
            return fixed[1] + (self.speaker.phonemes(fixed[2]) if fixed[2] else "")
        return self.speaker.phonemes(text)

    def speech_key(self, phonemes: tuple[str, ...]) -> str:
        """The key of what the voice says for a clip's parts, whatever is made of it after."""
        return key(f"{self.speech_recipe}|{' | '.join(phonemes)}")

    def speech(self, phonemes: str) -> tuple["np.ndarray", int, bool]:
        """What the voice says for one part, and whether it was spoken now."""
        import numpy as np
        import soundfile

        path = self.cache / "speech" / (self.speech_key((phonemes,)) + ".flac")
        spoken = not path.exists()
        if spoken:
            samples, hz = self.speaker.speak_phonemes(phonemes, self.speed)
            # FLAC holds integers up to full scale; `shape` sets the level again anyway.
            part = path.with_suffix(".part.flac")
            soundfile.write(part, samples * (0.9 / max(np.abs(samples).max(), 1e-9)), hz, subtype="PCM_24")
            os.replace(part, path)
        samples, hz = soundfile.read(path, dtype="float32")
        return samples, hz, spoken

    def clip(self, phonemes: tuple[str, ...]) -> tuple[bytes, str]:
        """The clip of the parts, MEANING_PAUSE_S apart, and how it came: "cached", "made" from
        kept speech, or "spoken" now. A part with sentences inside has its pauses shortened.
        Raises ValueError for phonemes the voice does not have."""
        path = self.cache / (key(f"{self.clip_recipe}|{self.speech_key(phonemes)}") + ".adpcm")
        if path.exists():
            return path.read_bytes(), "cached"
        import audioop

        import numpy as np
        from voiceclips import OUTPUT_HZ, close_gaps, level, resample, shape, trim, undo_upsampling

        parts = []
        how = "made"
        for part in phonemes:
            samples, hz, spoken = self.speech(part)
            how = "spoken" if spoken else how
            if SENTENCES.search(part):
                samples = close_gaps(samples, hz)
            if parts:
                parts.append(np.zeros(int(MEANING_PAUSE_S * hz), dtype=np.float32))
            parts.append(trim(samples))
        samples = np.concatenate(parts)
        pcm = np.frombuffer(resample(shape(samples, hz), hz, OUTPUT_HZ // FACTOR), "<i2") / 32768
        pcm = level(undo_upsampling(pcm, FACTOR), OUTPUT_HZ // FACTOR)
        clip = audioop.lin2adpcm(np.clip(pcm * 32768, -32768, 32767).astype("<i2").tobytes(), 2, None)[0]
        part = path.with_suffix(".part")
        part.write_bytes(clip)
        os.replace(part, path)
        return clip, how


def report_levels(clips: list[bytes]):
    """Prints the speech level of the clips next to the sidetone's."""
    import audioop
    import statistics

    import numpy as np
    from voiceclips import OUTPUT_HZ, SIDETONE_DB, active_level

    levels = [active_level(np.frombuffer(audioop.adpcm2lin(c, 2, None)[0], "<i2") / 32768, OUTPUT_HZ // FACTOR) - SIDETONE_DB for c in clips]
    low, *_, high = statistics.quantiles(levels, n=20)
    print(f"speech level against the sidetone: median {statistics.median(levels):+.1f} dB, 5% {low:+.1f}, 95% {high:+.1f}")


def report_review(voice: Voice, speech_keys: dict[tuple[str, str], str]):
    """Prints how many of the clips have a mark for the speech they have now."""
    review = read_review(voice.folder)
    current = [review[clip] for clip, speech_key in speech_keys.items() if clip in review and review[clip][1] == speech_key]
    good = sum(1 for mark, _, _ in current if mark == "good")
    stale = sum(1 for clip in speech_keys if clip in review) - len(current)
    gone = len(set(review) - set(speech_keys))
    print(
        f"review: {good} good, {len(current) - good} bad, {stale} stale and {len(speech_keys) - len(current) - stale} unmarked of {len(speech_keys)} clips"
        + (f"; {gone} marks for words in no list" if gone else "")
    )


def key(text: str) -> str:
    return hashlib.sha256(text.encode()).hexdigest()[:24]


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as f:
        while chunk := f.read(1 << 20):
            digest.update(chunk)
    return digest.hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--voice", default=VOICE, help=f"folder in tools/pack/voices (default {VOICE})")
    parser.add_argument("--models", type=Path, default=HERE / "models", help="folder with the model files")
    parser.add_argument("--out", type=Path, help="folder for F000400.bin")
    parser.add_argument("--cache", type=Path, default=CACHE, help="folder that keeps spoken clips")
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
    if args.no_voice:
        words = read_words()
        clips = [(None, None)] * len(words)
    else:
        voice = Voice(args.voice, args.models, args.cache)
        words = read_words(read_fixes(voice.folder))
        clips = []
        speech_keys = {}
        came = {"spoken": 0, "made": 0, "cached": 0}
        for i, (text, said, meaning, kind) in enumerate(words):
            pair = []
            for clip_name, parts in phrases(said, meaning, kind):
                phonemes = tuple(voice.phonemes(part) for part in parts)
                speech_keys[text, clip_name] = voice.speech_key(phonemes)
                try:
                    clip, how = voice.clip(phonemes)
                except ValueError as error:
                    sys.exit(f"{text}: {error}")
                pair.append(clip)
                came[how] += 1
                if how != "cached" and came[how] % 100 == 0:
                    print(f"{how} {came[how]}, at {i}/{len(words)} {text}", flush=True)
            clips.append(tuple(pair + [None] * (2 - len(pair))))
        print(f"spoke {came['spoken']} clips, made {came['made']} from kept speech, took {came['cached']} from the cache")
        report_review(voice, speech_keys)
    entries = [(text, kind, clip, with_meaning) for (text, _, _, kind), (clip, with_meaning) in zip(words, clips)]
    common = sum(1 for e in entries if e[1] == "common")
    pack = build(entries)
    over = len(pack) - FILE_MAX
    while over > 0 and entries[-1][1] == "common":
        over -= WORD_ENTRY + sum(CLIP_START + len(c) for c in entries.pop()[2:] if c)
    if over > 0:
        sys.exit(f"the pack is {over} bytes over {FILE_MAX} with no common words")
    pack = build(entries)
    if not args.no_voice:
        report_levels([c for e in entries for c in e[2:] if c])
    args.out.mkdir(parents=True, exist_ok=True)
    path = args.out / "F000400.bin"
    path.write_bytes(pack)
    counts = {kind: sum(1 for e in entries if e[1] == kind) for kind in FLAGS}
    print(f"{path}: {len(pack)} bytes, {counts['char']} characters, {counts['ham']} ham and {counts['common']} of {common} common words")


if __name__ == "__main__":
    main()
