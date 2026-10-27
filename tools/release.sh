#!/bin/sh
# Builds a release into dist/: the app image A000014.bin, the content pack F000400.bin, the
# macOS install tool pocketcw-flash that docs/install.sh downloads, and the licenses.
# Builds only from a clean tree whose firmware version is VERSION, so the files match the tag.
# The content pack needs the voice tools that tools/pack/setup.py sets up.
# usage: tools/release.sh VERSION   (for example v0.2.0)
set -eu
[ $# -eq 1 ] || { echo "usage: $0 VERSION" >&2; exit 1; }
version=$1
cd "$(dirname "$0")/.."
tools/pack/.venv/bin/python -c "import kokoro_onnx" 2>/dev/null || { echo "error: run python3.12 tools/pack/setup.py first" >&2; exit 1; }

crate=$(sed -n 's/^version = "\(.*\)"$/\1/p' firmware/Cargo.toml | head -n 1)
[ "v$crate" = "$version" ] || { echo "error: firmware/Cargo.toml has version $crate, not ${version#v}" >&2; exit 1; }
[ -z "$(git status --porcelain)" ] || { echo "error: uncommitted changes; commit them first" >&2; exit 1; }

sh tools/mkimage.sh
rm -rf dist
mkdir dist
cp firmware/target/A000014.bin LICENSE firmware/src/B612-OFL.txt dist/
# tools/panic.sh needs the matching ELF; it holds local paths, so it stays out of dist/.
mkdir -p symbols
cp firmware/target/thumbv7em-none-eabihf/release/pocketcw-firmware "symbols/A000014-$version.elf"
# Not run with -I: pack.py imports voiceclips from its own folder.
tools/pack/.venv/bin/python tools/pack/pack.py --out dist
clang -O2 -arch arm64 -arch x86_64 -mmacosx-version-min=10.15 -o dist/pocketcw-flash tools/pocketcw-flash.c \
    -framework DiskArbitration -framework CoreFoundation
{ python3 -I tools/notices.py firmware; cat tools/pack/pack-notice.txt; } >dist/THIRD-PARTY.txt
ls -l dist
shasum -a 256 dist/F000400.bin
echo
echo "Next: git tag $version && git push origin $version"
echo "Then: gh release create $version dist/*"
