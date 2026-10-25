#!/bin/sh
# Installs PocketCW, or the stock firmware V5.03, on a HamCube from macOS.
# Each install writes an F file to SPI flash before the A file: PocketCW's content pack must match
# its firmware, and the bootloader can fail an A write that no F write came before.
# usage: curl -fsSL https://kvmet.github.io/k5mme_pocketcw/install.sh | sh
#        curl -fsSL https://kvmet.github.io/k5mme_pocketcw/install.sh | sh -s stock
# The shell reads this script from its stdin, so prompts and pocketcw-flash read the terminal.
set -eu

release=https://github.com/kvmet/k5mme_pocketcw/releases/latest/download
vendor='https://raw.githubusercontent.com/admvip/CW-Pokemon-Infomation/master/CW%20Pokemon%20V5.03%20Upgrade%20package'
# These match the MD5 sums in HamCube's V5.03 upgrade instructions.
vendor_a_sha256=94e5ee35e091e3ce3306e80b22499d9a2261796e74c3d2b9478d32324246d0fa
vendor_f_sha256=60f75d9e4c69d0176f15de47dcc37f26aeee22d7f636ee3936b2829b9f144622

fail() {
    echo "error: $*" >&2
    exit 1
}

# get URL FILE
get() {
    curl -fsSL -o "$2" "$1" || fail "could not download $1"
}

# check FILE SHA256
check() {
    [ "$(shasum -a 256 "$1" | cut -d ' ' -f 1)" = "$2" ] || fail "$(basename "$1") is not the V5.03 file this script expects"
}

# flash FILE...
flash() {
    "$work/pocketcw-flash" "$@" </dev/tty
    echo "Then click the dial to turn it on."
}

install_pocketcw() {
    mkdir "$work/pocketcw"
    echo "Downloading the content pack, about 8 MB, and the firmware."
    get "$release/F000400.bin" "$work/pocketcw/F000400.bin"
    get "$release/A000014.bin" "$work/pocketcw/A000014.bin"
    echo "The install writes the content pack, which takes a few minutes, then the firmware."
    flash "$work/pocketcw/F000400.bin" "$work/pocketcw/A000014.bin"
}

install_stock() {
    mkdir "$work/stock"
    get "$vendor/F0000C0.bin" "$work/stock/F0000C0.bin"
    check "$work/stock/F0000C0.bin" $vendor_f_sha256
    get "$vendor/A000014.bin" "$work/stock/A000014.bin"
    check "$work/stock/A000014.bin" $vendor_a_sha256
    flash "$work/stock/F0000C0.bin" "$work/stock/A000014.bin"
}

main() {
    [ "$(uname -s)" = Darwin ] || fail "this script runs only on macOS"
    work=$(mktemp -d -t pocketcw)
    trap 'rm -rf "$work"' EXIT
    get "$release/pocketcw-flash" "$work/pocketcw-flash"
    chmod +x "$work/pocketcw-flash"
    case ${1:-} in
    "") install_pocketcw ;;
    stock) install_stock ;;
    *) fail "unknown option '$1'; use no option for PocketCW, or 'stock' for the stock firmware" ;;
    esac
}

# Called only once the whole script has arrived, so a cut-off download runs nothing.
main "$@"
