#!/bin/sh
# Names the function and line for each address on a PocketCW panic screen, with lldb (Xcode
# command line tools). The first frame outside core:: is the code that panicked; the line can be
# off where the compiler merged identical panic paths.
# usage: tools/panic.sh [-e ELF] ADDR...   (the ELF defaults to the last app build)
set -eu
cd "$(dirname "$0")/.."
elf=firmware/target/thumbv7em-none-eabihf/release/pocketcw-firmware
if [ "${1:-}" = -e ]; then
    elf=$2
    shift 2
fi
[ $# -gt 0 ] || { echo "usage: $0 [-e ELF] ADDR..." >&2; exit 1; }
for ret in "$@"; do
    case $ret in
    [Ff][Ff][Ff][Ff][Ff][Ff]*) echo "$ret: exception entry"; continue ;;
    esac
    # A return address points past its call, often into the next function when the callee
    # never returns. The 4-byte bl starts 4 bytes before it; minus 2 lands inside the bl.
    call=$(printf '0x%08X' $(( (0x$ret & ~1) - 2 )))
    echo "$ret:"
    lldb --batch -o "image lookup -a $call" "$elf" | sed -n '/Summary:/,$p' | sed 's/^ *\(Summary: \)\{0,1\}[^`]*`/    /; s/::h[0-9a-f]\{16\}//g'
done
