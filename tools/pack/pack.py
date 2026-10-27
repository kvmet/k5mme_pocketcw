#!/usr/bin/env python3
"""Builds the content pack F000400.bin (format in `logic/src/pack.rs`): the characters in
tools/pack/words/chars.txt and the words in tools/pack/words/ham.txt and tools/pack/words/common.txt, each with
a clip of a voice in tools/pack/voices at 8789 Hz. A word said letter by letter, such as QTH,
has no clip: the device spells it from the characters' clips. A ham word with a meaning gets a
clip of the meaning, which the device says after the word. The least common words are left out until the pack fits
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
FORMAT = 4
WORD_TEXT = 18
NO_CLIP = 0xFFFFFFFF
FLAGS = {"char": 2, "ham": 1, "common": 0}
FACTOR = 2
# The speech buffer in firmware/src/speech.rs, which holds a word's clip, or its letters' clips,
# and its meaning's.
CLIP_MAX = 16384
# The largest update file: 1022 clusters of 8 KB (docs/dev/hardware/flash-layout.md).
FILE_MAX = 8_372_224
WORD_ENTRY = 28
CLIP_START = 2
# Change when how a voice speaks a phoneme string changes, so every text is spoken again.
SPEECH_RECIPE = 2
# Change when this file's making of clips from speech changes. A change to tools/pack/voiceclips.py
# makes clips again by itself.
CLIP_RECIPE = 5
HERE = Path(__file__).parent
WORDS = HERE / "words"
VOICES = HERE / "voices"
CACHE = HERE.parent.parent / "firmware" / "target" / "clip-cache"
VOICE = "kokoro-af_heart"
# A said of /phonemes/ in fixes.txt goes to the voice as it is, and so does the start of a
# clip's text in that form.
PHONEMES = re.compile(r"/([^/]+)/")
FIXED = re.compile(r"/([^/]+)/(.*)")
# A said with a lone letter or digit, such as "Q T H" or "5 N N", spells a word.
CHARACTER = re.compile("[A-Z0-9]")
ENDS = ".,!?;:"
# Phonemes with a sentence end inside them, after which the voice pauses.
SENTENCES = re.compile(r"[.!?] +\S")
# What a cell of fixes.txt or review.txt may not hold: what ends a cell or a line.
BREAKS = re.compile("[\t\n\r\x0b\x0c\x1c-\x1e\x85\u2028\u2029]")
CLIPS = ("char", "word", "meaning")
REVIEW_HEADER = """# Clip marks from tools/pack/audition.py: text, clip (char, word or meaning), mark (good or
# bad), the speech key the mark was given for, and a note. A mark whose key differs from the clip's is stale.
"""
FIXES_HEADER = """# What this voice says for a clip, over the said or meaning column of tools/pack/words: text,
# clip (char, word or meaning), then a respelling, or misaki phonemes between slashes such as /bˈYzi/.
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


def clips_of(said: str | None, meaning: str | None, kind: str) -> list[tuple[str, str]]:
    """(clip, said) for each clip of an entry: its char or word clip, then its meaning's. A, I,
    K, R and W are both characters and words, so a fix or a mark names the clip."""
    first = [("char" if kind == "char" else "word", said)] if said else []
    return first + [("meaning", meaning)] if meaning else first


def read_words(fixes: dict[tuple[str, str], str] | None = None) -> list[tuple[str, str, str | None, str]]:
    """(text, said, meaning, kind) for each entry: characters, ham words, then common words
    that are not also ham words, with the said and meaning of `fixes` over the lists'. Said is
    None for a word the device spells from the characters' clips."""
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
    for text, said, _, _ in ham + common:
        if all_letters(said) and said.replace(" ", "") != text.strip("<>"):
            sys.exit(f"{text}: the device spells the text, so {said!r} must spell it too")
    entries = chars + [(text, None if all_letters(said) else said, meaning, kind) for text, said, meaning, kind in ham + common]
    fixes = fixes or {}
    unknown = set(fixes) - {(text, clip) for text, said, meaning, kind in entries for clip, _ in clips_of(said, meaning, kind)}
    if unknown:
        sys.exit(f"fixes.txt has clips in no list: {' '.join(f'{text} {clip}' for text, clip in sorted(unknown))}")
    fixed = []
    for text, said, meaning, kind in entries:
        first = "char" if kind == "char" else "word"
        fixed.append((text, said and fixes.get((text, first), said), meaning and fixes.get((text, "meaning"), meaning), kind))
    return fixed


def ended(text: str) -> str:
    """`text` ending in a period, unless it ends in a mark already; /phonemes/ too."""
    return text if text.rstrip("/")[-1:] in ENDS else f"{text}."


def spoken(clip: str, said: str) -> str:
    """What the voice says for a clip of `said`. Kokoro rushes a lone letter and runs spelled
    letters together; as the end of a sentence it says each one in full, so a character, a
    meaning and each part of a spelled word end in a period. Kokoro barely pauses at a comma,
    so a meaning's commas become ellipses."""
    if clip == "meaning" and not PHONEMES.fullmatch(said):
        return ended(said.replace(",", "…"))
    if clip != "word":
        return ended(said)
    if spelled(said):
        return " ".join(ended(part) for part in said.split())
    return said


def spelled(said: str) -> bool:
    return not PHONEMES.fullmatch(said) and any(CHARACTER.fullmatch(part.rstrip(ENDS)) for part in said.split())


def all_letters(said: str) -> bool:
    """A said of only letters and digits, such as "Q T H", which the device spells from the
    characters' clips."""
    return not PHONEMES.fullmatch(said) and all(CHARACTER.fullmatch(part.rstrip(ENDS)) for part in said.split())


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
    """The said for each (text, clip)."""
    fixes = {}
    for text, clip, said in read_table(folder / "fixes.txt", 3):
        if clip not in CLIPS or not said or (text, clip) in fixes:
            sys.exit(f"{folder / 'fixes.txt'}: {text} needs char, word or meaning, and one said")
        fixes[text, clip] = said
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
    """The pack for (text, kind, clip, meaning clip) entries, with clips at the output rate
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

    chars = {text: clip for text, kind, clip, _ in words if kind == "char"}
    for text, kind, clip, meaning in words:
        # The device spells a word with no clip from the characters' clips it finds.
        played = clip or b"".join(chars.get(c) or b"" for c in text)
        if meaning and len(played) + len(meaning) > CLIP_MAX:
            sys.exit(f"the clips for {text} and its meaning are {len(played) + len(meaning)} bytes, more than {CLIP_MAX}")
        encoded = text.encode()
        entries += struct.pack("<BB", len(encoded), FLAGS[kind]) + encoded.ljust(WORD_TEXT, b"\0")
        entries += struct.pack("<II", add(text, clip), add(text, meaning))
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
        """What the voice is given for a clip's text."""
        fixed = FIXED.fullmatch(text)
        if fixed:
            return fixed[1] + (self.speaker.phonemes(fixed[2]) if fixed[2] else "")
        return self.speaker.phonemes(text)

    def speech_key(self, phonemes: str) -> str:
        """The key of what the voice says for `phonemes`, whatever is made of it after."""
        return key(f"{self.speech_recipe}|{phonemes}")

    def clip(self, phonemes: str) -> tuple[bytes, str]:
        """The clip, and how it came: "cached", "made" from kept speech, or "spoken" now.
        Phonemes with sentences inside have their pauses shortened. Raises ValueError for
        phonemes the voice does not have."""
        speech_key = self.speech_key(phonemes)
        path = self.cache / (key(f"{self.clip_recipe}|{speech_key}") + ".adpcm")
        if path.exists():
            return path.read_bytes(), "cached"
        import audioop

        import numpy as np
        import soundfile
        from voiceclips import OUTPUT_HZ, close_gaps, cut_stray_vowel, level, resample, shape, undo_upsampling

        speech = self.cache / "speech" / (speech_key + ".flac")
        how = "made" if speech.exists() else "spoken"
        if how == "spoken":
            samples, hz = self.speaker.speak_phonemes(phonemes, self.speed)
            # FLAC holds integers up to full scale; `shape` sets the level again anyway.
            part = speech.with_suffix(".part.flac")
            soundfile.write(part, samples * (0.9 / max(np.abs(samples).max(), 1e-9)), hz, subtype="PCM_24")
            os.replace(part, speech)
        samples, hz = soundfile.read(speech, dtype="float32")
        # Kokoro often says a short vowel before a first s or f: "ehsports", "vast". A z or v starts voiced.
        if phonemes.lstrip("ˈˌ")[:1] in ("s", "f"):
            samples = cut_stray_vowel(samples, hz)
        if SENTENCES.search(phonemes):
            samples = close_gaps(samples, hz)
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
            ("<BK>", "ham", None, bytes([6])),
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
            made = {}
            for clip_name, clip_said in clips_of(said, meaning, kind):
                phonemes = voice.phonemes(spoken(clip_name, clip_said))
                speech_keys[text, clip_name] = voice.speech_key(phonemes)
                try:
                    clip, how = voice.clip(phonemes)
                except ValueError as error:
                    sys.exit(f"{text}: {error}")
                made[clip_name] = clip
                came[how] += 1
                if how != "cached" and came[how] % 100 == 0:
                    print(f"{how} {came[how]}, at {i}/{len(words)} {text}", flush=True)
            clips.append((made.get("char") or made.get("word"), made.get("meaning")))
        print(f"spoke {came['spoken']} clips, made {came['made']} from kept speech, took {came['cached']} from the cache")
        report_review(voice, speech_keys)
    entries = [(text, kind, clip, meaning) for (text, _, _, kind), (clip, meaning) in zip(words, clips)]
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
