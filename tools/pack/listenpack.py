#!/usr/bin/env python3
"""Builds a listening pack F000400.bin that compares ways to make clips, heard on the device.
Each test is made each way in WAYS, and each way is a pack word: the test's name, then the
way's letter.

    N  each letter on its own
    P  as N, ending in a period, so Kokoro says it as the end of a sentence and does not rush it
    S  as P, at 0.85 of Kokoro's speed

All at the device's 17,578 Hz. A way at a lower rate first goes through what the device does
at that rate: IMA ADPCM, then straight lines between samples up to 17,578 Hz.

Needs what tools/pack/pack.py needs for Kokoro.

    tools/pack/.venv/bin/python tools/pack/listenpack.py --out firmware/target/listen
"""

import argparse
import audioop
from pathlib import Path
from typing import NamedTuple

import numpy as np

from pack import HERE, build, read_list
from voiceclips import OUTPUT_HZ, Speaker, resample, shape, undo_upsampling, upsample

GAP_S = 0.12
# What each test speaks, a clip for each part. A part that is a character in chars.txt is
# spoken as chars.txt says it.
TESTS = {"H": ["H"], "A": ["A"], "K": ["K"], "J": ["J"], "TH": list("TH"), "8H": list("8H"), "AJ": list("AJ"), "BZ": list("BZ")}


class Way(NamedTuple):
    factor: int = 1
    boost: bool = False
    period: bool = False
    speed: float = 1.0


WAYS = {"N": Way(), "P": Way(period=True), "S": Way(period=True, speed=0.85)}


def through_device(parts: list[tuple[np.ndarray, int]], factor: int, boost: bool) -> bytes:
    """One clip at 17,578 Hz that sounds like `parts` played at the output rate divided by
    `factor`, each part a clip of its own and GAP_S after the one before."""
    hz = OUTPUT_HZ // factor
    pieces = []
    for samples, from_hz in parts:
        pcm = np.frombuffer(resample(shape(samples, from_hz), from_hz, hz), "<i2").astype(np.float64)
        if boost:
            pcm = undo_upsampling(pcm, factor)
        pcm = np.clip(pcm, -32768, 32767).astype("<i2").tobytes()
        if factor > 1:
            pcm = audioop.adpcm2lin(audioop.lin2adpcm(pcm, 2, None)[0], 2, None)[0]
        if pieces:
            pieces.append(np.zeros(int(GAP_S * hz), np.int16))
        pieces.append(np.frombuffer(pcm, "<i2"))
    output = upsample(np.concatenate(pieces), factor)
    return audioop.lin2adpcm(output.astype("<i2").tobytes(), 2, None)[0]


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--models", type=Path, default=HERE / "models", help="folder with the Kokoro model files")
    parser.add_argument("--out", type=Path, required=True, help="folder for F000400.bin")
    args = parser.parse_args()

    said = {text: spoken for text, spoken, _, _ in read_list("chars.txt", "char")}
    speaker = Speaker("kokoro", args.models)
    words = []
    for test, parts in TESTS.items():
        for name, way in WAYS.items():
            spoken = [speaker.speak(said.get(part, part) + "." * way.period, way.speed) for part in parts]
            clip = through_device(spoken, way.factor, way.boost)
            words.append((test + name, "common", clip, None))
            print(f"{test + name}: {len(clip) * 2 / OUTPUT_HZ:.2f} s")
    args.out.mkdir(parents=True, exist_ok=True)
    path = args.out / "F000400.bin"
    path.write_bytes(build(words, factor=1))
    print(f"{path}: {path.stat().st_size} bytes")


if __name__ == "__main__":
    main()
