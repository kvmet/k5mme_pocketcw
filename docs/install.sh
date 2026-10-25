#!/bin/sh
# Installs PocketCW, or the stock firmware V5.03, on a HamCube from macOS.
# A first install writes an F file to SPI flash before the A file: the bootloader can fail an A
# write that no F write came before. A PocketCW install asks whether to write the content pack,
# since most updates leave it unchanged.
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
    get "$release/A000014.bin" "$work/pocketcw/A000014.bin"
    echo
    echo "The content pack holds the voice and the words for the drills. Install it:"
    echo "  - on the first PocketCW install on this device, or when the device stays on the"
    echo "    update screen after an install."
    echo "  - when the content pack changed since you last installed it. The release notes"
    echo "    name the release that last changed it."
    echo "  - when NINJA or HEAD shows CONTENT PACK MISSING."
    echo "Otherwise the firmware alone is enough. The pack is about 8 MB and takes a few"
    echo "minutes to install."
    echo
    echo "If you might go back to the stock firmware: the content pack can write over the"
    echo "stock firmware's contact log, and that log is then lost. A first install needs"
    echo "the content pack all the same."
    echo
    printf "Install the content pack? [y/N] "
    read -r answer </dev/tty
    case $answer in
    [yY]*)
        get "$release/F000400.bin" "$work/pocketcw/F000400.bin"
        flash "$work/pocketcw/F000400.bin" "$work/pocketcw/A000014.bin"
        ;;
    *)
        flash "$work/pocketcw/A000014.bin"
        ;;
    esac
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
