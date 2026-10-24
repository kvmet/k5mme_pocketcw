#!/bin/sh
# Installs PocketCW, or the stock firmware V5.03, on a HamCube from macOS.
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

flash() {
    "$work/pocketcw-flash" "$1" </dev/tty
}

install_pocketcw() {
    mkdir "$work/pocketcw"
    get "$release/A000014.bin" "$work/pocketcw/A000014.bin"
    flash "$work/pocketcw/A000014.bin"
    echo "Then click the dial to turn it on."
}

install_stock() {
    echo "The stock firmware draws its screens from images stored on the device (F0000C0.bin)."
    echo "Older images scramble the stock screen. The images stay on the device, so they are"
    echo "needed only once, even if you install PocketCW in between."
    echo
    printf "Has this device had the V5.03 images installed before? [y/N] "
    read -r answer </dev/tty
    mkdir "$work/stock"
    case $answer in
    [yY]*) images=no ;;
    *) images=yes ;;
    esac
    if [ $images = yes ]; then
        get "$vendor/F0000C0.bin" "$work/stock/F0000C0.bin"
        check "$work/stock/F0000C0.bin" $vendor_f_sha256
    fi
    get "$vendor/A000014.bin" "$work/stock/A000014.bin"
    check "$work/stock/A000014.bin" $vendor_a_sha256
    if [ $images = yes ]; then
        echo
        echo "Step 1 of 2: the images."
        flash "$work/stock/F0000C0.bin"
        echo "Do not turn it on yet."
        echo
        echo "Step 2 of 2: the firmware."
    fi
    flash "$work/stock/A000014.bin"
    echo "Then click the dial to turn it on."
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
