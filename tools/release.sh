#!/bin/sh
# Builds a release into dist/: the app image A000014.bin, the SPI flash file F000400.bin, the
# macOS install tool pocketcw-flash that docs/install.sh downloads, and the licenses.
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
# 4 KB at SPI flash 1 MB: magic, format version 1, then erased flash. A first install writes it
# before A000014.bin, because the bootloader can fail a first A write that no F write came before.
{ printf 'PocketCW\001'; head -c 4087 /dev/zero | LC_ALL=C tr '\0' '\377'; } >dist/F000400.bin
clang -O2 -arch arm64 -arch x86_64 -mmacosx-version-min=10.15 -o dist/pocketcw-flash tools/pocketcw-flash.c \
    -framework DiskArbitration -framework CoreFoundation
python3 -I tools/notices.py firmware >dist/THIRD-PARTY.txt
ls -l dist
echo
echo "Next: git tag $version && git push origin $version"
echo "Then: gh release create $version dist/*"
