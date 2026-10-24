# Install on macOS

This page installs PocketCW on a HamCube from a Mac. Only macOS has an install tool today.

PocketCW was tested on a Mini with board version 2.64. Other board versions and the Pro may work, but nobody has tried them yet.

## What you need

- A Mac with an Intel or Apple chip, macOS 10.15 or later, connected to the internet.
- A USB cable that carries data. Some cables only charge. If the Mac never finds the device, try a different cable.
- Your Mac login password. The install writes directly to the device, and macOS asks for your password before it allows that.
- About 5 minutes.

## Is it safe?

The device has a small built-in program, the bootloader, that installs firmware. This install never touches the bootloader: the install tool refuses any file that would write over it. If an install goes wrong, the bootloader still works, and you can run the install again.

To go back to the original firmware, see [Go back to the stock firmware](stock-firmware.md). That page installs the published stock version, V5.03. Some devices come from the factory with a newer, unpublished version, and the install replaces it for good. On board 2.64, V5.03 has no sound.

## Install

1. Turn the device off, and unplug its USB cable. To turn it off, hold the dial down for about 2 seconds.
2. Open Terminal. Press Command and Space together, type Terminal, and press Return.
3. Copy this line, paste it into the Terminal window, and press Return:

    ```bash
    curl -fsSL https://kvmet.github.io/k5mme_pocketcw/install.sh | sh
    ```

    The script downloads the newest PocketCW release and the install tool.
4. The script now says "Waiting for the device". Put the device in update mode, then plug it in:
    1. Find F1 and F4. They are two of the four buttons beside the screen: F1 is the top one, and F4 is the bottom one.
    2. Hold F1 and F4 down.
    3. While you hold them, hold the dial down too.
    4. When the firmware update screen shows, let go of all three.
    5. Plug the USB cable into the device and the Mac.

    After a few seconds, the Terminal window says "Found the device". If the device already shows the firmware update screen, start at step 4.5.
5. macOS asks for your password. Type your Mac login password, and click OK.
6. Wait about 10 seconds. When the install works, you see:

    ```
    Finished. Unplug the device.
    Then click the dial to turn it on.
    ```

7. Unplug the device, then click the dial to turn it on.

To check the version: on the KEYER screen, press F4 (MENU). Turn the dial to SYSTEM, and click the dial. The FIRMWARE row shows the version, such as `v0.2.0`.

## While the install runs

- Do not unplug the device until the script says "Finished".
- When the script finds a problem, it prints `Problem:` and the reason, then waits. Fix the cause, press Return, and follow the steps it prints again.
- macOS can show a message about a disk it cannot read. Click Ignore. If you click Initialize by mistake, close the Disk Utility window that opens, and change nothing.
- To stop the script at any time, press Control and C together.

If something goes wrong, see [Troubleshooting](troubleshooting.md).

## Install with your own tools

Each [release](https://github.com/kvmet/k5mme_pocketcw/releases) has these files:

| File | What it is |
| - | - |
| `A000014.bin` | The firmware. |
| `pocketcw-flash` | The macOS install tool that the script runs. |
| `LICENSE` | The PocketCW license, GPL-3.0-or-later. |
| `B612-OFL.txt` | The license of the B612 Mono font. |
| `THIRD-PARTY.txt` | The licenses of the code libraries in the firmware. |
