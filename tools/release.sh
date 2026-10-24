#!/bin/sh
# Builds a release into dist/: the app image A000014.bin, the macOS install tool pocketcw-flash
# that docs/install.sh downloads, and the licenses.
# Builds only from a clean tree whose firmware version is VERSION, so the files match the tag.
# usage: tools/release.sh VERSION   (for example v0.2.0)
set -eu
[ $# -eq 1 ] || { echo "usage: $0 VERSION" >&2; exit 1; }
version=$1
cd "$(dirname "$0")/.."

crate=$(sed -n 's/^version = "\(.*\)"$/\1/p' firmware/Cargo.toml | head -n 1)
[ "v$crate" = "$version" ] || { echo "error: firmware/Cargo.toml has version $crate, not ${version#v}" >&2; exit 1; }
[ -z "$(git status --porcelain)" ] || { echo "error: uncommitted changes; commit them first" >&2; exit 1; }

sh tools/mkimage.sh
rm -rf dist
mkdir dist
cp firmware/target/A000014.bin LICENSE firmware/src/B612-OFL.txt dist/
clang -O2 -arch arm64 -arch x86_64 -mmacosx-version-min=10.15 -o dist/pocketcw-flash tools/pocketcw-flash.c \
    -framework DiskArbitration -framework CoreFoundation
python3 -I tools/notices.py firmware >dist/THIRD-PARTY.txt
ls -l dist
echo
echo "Next: git tag $version && git push origin $version"
echo "Then: gh release create $version dist/*"
