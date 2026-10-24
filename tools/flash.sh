#!/bin/sh
# Installs a HamCube update file through the device's update drive, from macOS.
# usage: sh flash.sh [FILE]   (default: A000014.bin next to this script)
set -eu

here=$(cd "$(dirname "$0")" && pwd)
file=${1:-$here/A000014.bin}
nomount=$here/nomount
wait_s=180

fail() {
    echo
    echo "error: $*" >&2
    exit 1
}

[ -f "$file" ] || fail "no update file at $file"
[ -x "$nomount" ] || fail "no nomount program at $nomount; build it with the command at the top of nomount.c"

# A fresh Mac has only a stand-in python3 that offers to install the Command Line Tools.
if ! python3 -c '' >/dev/null 2>&1; then
    fail "Python 3 is not installed. If macOS offers to install the command line developer tools, click Install, wait for it to finish, then run this again. Otherwise run: xcode-select --install"
fi

# macOS refuses to run a downloaded program until its quarantine mark is removed.
if xattr -p com.apple.quarantine "$nomount" >/dev/null 2>&1; then
    xattr -d com.apple.quarantine "$nomount"
fi

# A mounted drive has already had macOS files written to it, which the update can't share.
if ls /Volumes | grep -qix -e 'POKEMON-2' -e 'HAM MODULE'; then
    fail "the update drive is already mounted. Eject it in Finder, unplug the device, then run this again."
fi

log=$(mktemp -t pocketcw-nomount)
"$nomount" >"$log" 2>&1 &
nomount_pid=$!
stop_nomount() {
    rm -f "$log"
    # Already stopped when the kill fails.
    kill "$nomount_pid" 2>/dev/null || return 0
    # Reaping it keeps the shell from printing a "Terminated" notice; its status is the kill's.
    wait "$nomount_pid" 2>/dev/null || true
}
trap stop_nomount EXIT
sleep 1
kill -0 "$nomount_pid" 2>/dev/null || fail "nomount stopped: $(cat "$log")"

echo "Installing $(basename "$file") ($(wc -c <"$file" | tr -d ' ') bytes)."
echo
echo "1. Unplug the device, and turn it off."
echo "2. Hold F1 and F4. Keep holding them, and hold the dial down too."
echo "3. When the firmware update screen shows, let go of all three."
echo "4. Plug the device into this Mac."
echo
echo "Waiting for the device (up to $wait_s seconds; Ctrl-C to stop)..."

disk=
waited=0
while [ -z "$disk" ]; do
    disk=$(sed -n 's|^blocked mount of \(/dev/disk[0-9s]*\)$|\1|p' "$log" | head -n 1)
    [ -n "$disk" ] && break
    kill -0 "$nomount_pid" 2>/dev/null || fail "nomount stopped: $(cat "$log")"
    [ "$waited" -lt "$wait_s" ] || fail "no update drive appeared in $wait_s seconds. See Troubleshooting in README.txt."
    sleep 1
    waited=$((waited + 1))
done

echo "Found the device at $disk."
echo
echo "Writing needs your Mac login password. Type it and press Return; it does not show as you type."
sudo python3 "$here/iapwrite.py" "$disk" "$file"

echo
echo "Finished. Unplug the device, then click the dial to turn it on."
