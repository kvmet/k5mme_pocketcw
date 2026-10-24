# Install on macOS

This page installs PocketCW on a HamCube "CW Pokemon Mini" from a Mac. Only macOS has an install tool today.

PocketCW was tested on board version 2.64. Other board versions may work, but nobody has tried them yet.

## What you need

- A Mac with an Intel or Apple chip, macOS 10.15 or later.
- A USB cable that carries data. Some cables only charge. If the Mac never finds the device, try a different cable.
- Your Mac login password. The install writes directly to the device, and macOS asks for your password before it allows that.
- About 5 minutes.

## Is it safe?

The device has a small built-in program, the bootloader, that installs firmware. This install never touches the bootloader: the install tool refuses any file that would write over it. If an install goes wrong, the bootloader still works, and you can run the install again.

To go back to the original firmware, see [Go back to the stock firmware](stock-firmware.md).

## Get the files

1. Open [the releases page](https://github.com/kvmet/k5mme_pocketcw/releases).
2. Download `pocketcw-vX.Y.Z-mac.zip` from the newest release. X.Y.Z is the version number.

The release also has `A000014.bin` as its own download. You need it only if you install with your own tools.

The zip file unzips to a folder named `pocketcw-vX.Y.Z`. The folder holds these files:

| File | What it is |
| - | - |
| `A000014.bin` | The firmware. |
| `flash.sh` | The install script. It runs the next two programs for you. |
| `iapwrite.py` | Writes the firmware file to the device. |
| `nomount` | Stops macOS from opening the device's update drive while the install runs. |
| `README.txt` | The install guide. |
| `LICENSE` | The PocketCW license, GPL-3.0-or-later. |
| `B612-OFL.txt` | The license of the B612 Mono font. |
| `THIRD-PARTY.txt` | The licenses of the code libraries in the firmware. |

## Install

1. Unzip the download, if your Mac has not done that already.
2. Turn the device off, and unplug its USB cable. To turn it off, hold the dial down for about 2 seconds.
3. Open Terminal. Press Command and Space together, type Terminal, and press Return.
4. In the Terminal window, type `sh` and then a space. Do not press Return yet.
5. Drag the file `flash.sh` from the folder into the Terminal window. Its full location appears after `sh `. Now press Return.
6. A box can appear that offers to install the "command line developer tools". If it does, click Install and wait until it finishes. This can take several minutes. Then go back to step 4. You do this only once.
7. The script now says "Waiting for the device". Put the device in update mode, then plug it in:
    1. Find F1 and F4. They are two of the four buttons beside the screen: F1 is the top one, and F4 is the bottom one.
    2. Hold F1 and F4 down.
    3. While you hold them, hold the dial down too.
    4. When the firmware update screen shows, let go of all three.
    5. Plug the USB cable into the device and the Mac.

    After a few seconds, the Terminal window says "Found the device". If the device already shows the firmware update screen, start at step 7.5.
8. Terminal asks for your password. Type your Mac login password and press Return. Nothing shows while you type. That is normal.
9. Wait about 10 seconds. When the install works, you see:

    ```
    status now 'SUCCESS TXT'
    done: the device finished the update
    Finished. Unplug the device, then click the dial to turn it on.
    ```

    The device can also drop off USB for a moment and come back. Then you see "device dropped off USB" and then "done". That is a success too.
10. Unplug the device, then click the dial to turn it on.

To check the version: on the KEYER screen, press F4 (MENU). Turn the dial to SYSTEM, and click the dial. The FIRMWARE row shows the version, such as `v0.2.0`.

## While the install runs

- Do not unplug the device until the script says "Finished".
- macOS can show a message about a disk it cannot read. Click Ignore. If you click Initialize by mistake, close the Disk Utility window that opens, and change nothing.
- To stop the script at any time, press Control and C together.

If something goes wrong, see [Troubleshooting](troubleshooting.md).
