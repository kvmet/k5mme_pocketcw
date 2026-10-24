#!/bin/sh
# Builds a release into dist/: the app image A000014.bin and the macOS install bundle
# pocketcw-VERSION-mac.zip, holding the image, the install tools, README.txt and the licenses.
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
cp firmware/target/A000014.bin dist/

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
stage=$work/pocketcw-$version
mkdir "$stage"
clang -O2 -arch arm64 -arch x86_64 -mmacosx-version-min=10.15 -o "$stage/nomount" tools/nomount.c \
    -framework DiskArbitration -framework CoreFoundation
cp tools/flash.sh tools/iapwrite.py dist/A000014.bin LICENSE firmware/src/B612-OFL.txt "$stage/"
python3 -I tools/notices.py firmware >"$stage/THIRD-PARTY.txt"
sed "s/@VERSION@/$version/g" tools/install-readme.txt >"$stage/README.txt"

zip=dist/pocketcw-$version-mac.zip
# ditto keeps the execute bits and leaves out macOS metadata files.
ditto -c -k --norsrc --keepParent "$stage" "$zip"
unzip -l "$zip"
echo
echo "Next: git tag $version && git push origin $version"
echo "Then make a GitHub release for $version and upload dist/A000014.bin and $zip."
