# Go back to the stock firmware

This page puts the original HamCube firmware, version V5.03, back on the device. PocketCW does not include the HamCube files. You get them from a separate download.

## Before you start

On board 2.64, the stock firmware has no setup for the board's sound chip (an ES8388), so the speaker is silent under the stock firmware.

Boards from 2.63 have an IPS screen. Stock V5.03 has no setup for the IPS screen, so it shows that screen on its side, in strips. The beta version V5.03.13 sets up the IPS screen, but only on boards that read as version 2.63. No stock version fully supports board 2.64.

The SYSTEM screen of PocketCW shows your board version on the BOARD row.

Stock firmware keeps its settings in a different part of the memory chip than PocketCW. If you install PocketCW again later, it finds its settings again.

## Get the files

1. Open [github.com/admvip/CW-Pokemon-Infomation](https://github.com/admvip/CW-Pokemon-Infomation).
2. Open the folder "CW Pokemon V5.03 Upgrade package".
3. Download `F0000C0.bin` and `A000014.bin`.
4. Put both files in a folder of their own. Do not put them in the PocketCW folder: that folder has its own `A000014.bin`.

On a board 2.63, you can use the beta in place of the V5.03 `A000014.bin`: open the folder "CW Pokemon Beta Upgrade package", download `V5.03.13Beta.zip`, and unzip it. It holds only `A000014.bin`, so you still need `F0000C0.bin` from the V5.03 folder.

`F0000C0.bin` holds the images of the stock screens. The stock firmware draws its screens from the device's image memory, and an older V5.02 copy of this file scrambles the stock screen. HamCube says to install `F0000C0.bin` before `A000014.bin`. PocketCW never reads the image memory, so PocketCW works with any version of this file.

## Install

You install one file per install run. You need the macOS install tool from the PocketCW download. See [Install on macOS](install.md).

1. Do steps 2 to 4 of [Install on macOS](install.md).
2. Drag `flash.sh` into the Terminal window. Type a space. Drag the HamCube file `F0000C0.bin` into the window too. Press Return. The command looks like this:

    ```
    sh flash.sh F0000C0.bin
    ```

3. Continue from step 7 of the install, and wait until the script says "Finished".
4. Unplug the device.
5. Do steps 1 to 3 again with the HamCube file `A000014.bin`:

    ```
    sh flash.sh A000014.bin
    ```

6. Unplug the device, then click the dial to turn it on.

To install PocketCW again, follow [Install on macOS](install.md). PocketCW needs no `F` file.
