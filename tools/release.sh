#!/bin/sh
# Builds a release into dist/: the app image A000014.bin, the content pack F000400.bin, the
# macOS install tool pocketcw-flash that docs/install.sh downloads, and the licenses.
# Builds only from a clean tree whose firmware version is VERSION, so the files match the tag.
# VOICE_PYTHON is a Python 3.12 with the voice tools that tools/pack.py needs, and VOICE_MODELS
# the folder with the Kokoro model files.
# usage: VOICE_PYTHON=PYTHON VOICE_MODELS=DIR tools/release.sh VERSION   (for example v0.2.0)
set -eu
[ $# -eq 1 ] || { echo "usage: VOICE_PYTHON=PYTHON VOICE_MODELS=DIR $0 VERSION" >&2; exit 1; }
version=$1
[ -n "${VOICE_PYTHON:-}" ] || { echo "error: set VOICE_PYTHON to the Python that runs tools/pack.py" >&2; exit 1; }
[ -n "${VOICE_MODELS:-}" ] || { echo "error: set VOICE_MODELS to the folder with the Kokoro model files" >&2; exit 1; }
# Both may be relative to the folder this runs from, so they are made absolute before the cd.
case $VOICE_PYTHON in */*) VOICE_PYTHON=$(cd "$(dirname "$VOICE_PYTHON")" && pwd)/$(basename "$VOICE_PYTHON") ;; esac
VOICE_MODELS=$(cd "$VOICE_MODELS" && pwd)
cd "$(dirname "$0")/.."

crate=$(sed -n 's/^version = "\(.*\)"$/\1/p' firmware/Cargo.toml | head -n 1)
[ "v$crate" = "$version" ] || { echo "error: firmware/Cargo.toml has version $crate, not ${version#v}" >&2; exit 1; }
[ -z "$(git status --porcelain)" ] || { echo "error: uncommitted changes; commit them first" >&2; exit 1; }

sh tools/mkimage.sh
rm -rf dist
mkdir dist
cp firmware/target/A000014.bin LICENSE firmware/src/B612-OFL.txt dist/
# Not run with -I: pack.py imports voiceclips from its own folder when a clip is not cached.
"$VOICE_PYTHON" tools/pack.py --models "$VOICE_MODELS" --out dist
clang -O2 -arch arm64 -arch x86_64 -mmacosx-version-min=10.15 -o dist/pocketcw-flash tools/pocketcw-flash.c \
    -framework DiskArbitration -framework CoreFoundation
{ python3 -I tools/notices.py firmware; cat tools/pack-notice.txt; } >dist/THIRD-PARTY.txt
ls -l dist
shasum -a 256 dist/F000400.bin
echo
echo "Next: git tag $version && git push origin $version"
echo "Then: gh release create $version dist/*"
