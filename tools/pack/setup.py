#!/usr/bin/env python3
"""Sets up the voice tools: a Python 3.12 venv in tools/pack/.venv with the packages in
requirements.txt, and the model files every voice in tools/pack/voices needs, in
tools/pack/models. Checks that espeak-ng and ffmpeg are on the path. Run it again after a
change to requirements.txt or a voice.toml; it skips what is already in place.

    python3.12 tools/pack/setup.py
"""

import hashlib
import os
import re
import shutil
import subprocess
import sys
import tomllib
import urllib.request
import venv
from pathlib import Path

HERE = Path(__file__).parent
VENV = HERE / ".venv"
MODELS = HERE / "models"
ESPEAK_NG = (1, 52)


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as f:
        while chunk := f.read(1 << 20):
            digest.update(chunk)
    return digest.hexdigest()


def download(name: str, url: str, expected: str):
    path = MODELS / name
    if path.exists() and sha256(path) == expected:
        print(f"{name}: in place")
        return
    part = path.with_suffix(path.suffix + ".part")
    print(f"{name}: downloading {url}")
    with urllib.request.urlopen(url) as response, part.open("wb") as f:
        shutil.copyfileobj(response, f, 1 << 20)
    got = sha256(part)
    if got != expected:
        part.unlink()
        sys.exit(f"{name}: sha256 is {got}, not {expected} as voice.toml says")
    os.replace(part, path)


def main():
    if sys.version_info[:2] != (3, 12):
        sys.exit(f"run this with Python 3.12, not {sys.version.split()[0]}: the tools encode with audioop, which 3.13 removed")
    missing = [tool for tool in ("espeak-ng", "ffmpeg") if shutil.which(tool) is None]
    if missing:
        sys.exit(f"install {' and '.join(missing)} first, for example with Homebrew")
    version = subprocess.run(["espeak-ng", "--version"], capture_output=True, text=True, check=True).stdout
    found = tuple(int(n) for n in re.search(r"(\d+)\.(\d+)", version).groups())
    if found < ESPEAK_NG:
        sys.exit(f"espeak-ng is {found[0]}.{found[1]}; the phoneme table needs {ESPEAK_NG[0]}.{ESPEAK_NG[1]} or later")

    if not VENV.exists():
        venv.create(VENV, with_pip=True)
    python = VENV / "bin" / "python"
    subprocess.run([python, "-m", "pip", "install", "-q", "-r", HERE / "requirements.txt"], check=True)

    MODELS.mkdir(exist_ok=True)
    for settings in sorted((HERE / "voices").glob("*/voice.toml")):
        for name, model in tomllib.loads(settings.read_text())["models"].items():
            download(name, model["url"], model["sha256"])
    print(f"Ready. Run the tools with {python.relative_to(Path.cwd()) if python.is_relative_to(Path.cwd()) else python}")


if __name__ == "__main__":
    main()
