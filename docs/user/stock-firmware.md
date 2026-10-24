# Go back to the stock firmware

This page puts the original HamCube firmware, version V5.03, back on the device. PocketCW does not include the HamCube files. The install script downloads them from a separate source.

## Before you start

Board 2.64 has a different sound chip (an ES8388). The published stock versions, V5.03 and the beta V5.03.13, have no setup for that chip, so the speaker is silent under them. At least one board 2.64 came from the factory with V5.03.15, a version HamCube has not published, and its sound worked. If your device came with a stock version newer than V5.03.13, you cannot get that version back after you install PocketCW.

Boards from 2.63 have an IPS screen. Stock V5.03 has no setup for the IPS screen, so it shows that screen on its side, in strips. The beta version V5.03.13 sets up the IPS screen, but only on boards that read as version 2.63. No published stock version fully supports board 2.64.

The SYSTEM screen of PocketCW shows your board version on the BOARD row.

Stock firmware keeps its settings in a different part of the memory chip than PocketCW. If you install PocketCW again later, it finds its settings again.

## The two stock files

The stock firmware is two files from HamCube's V5.03 upgrade package:

- `F0000C0.bin` holds the images of the stock screens. The stock firmware draws its screens from the device's image memory, and an older V5.02 copy of this file scrambles the stock screen. The images stay in the image memory, so a device needs this file only once. PocketCW never reads the image memory, so PocketCW works with any version of this file.
- `A000014.bin` is the stock firmware.

HamCube says to install `F0000C0.bin` before `A000014.bin`.

PocketCW does not include these files. The install script downloads them from [github.com/admvip/CW-Pokemon-Infomation](https://github.com/admvip/CW-Pokemon-Infomation), and checks that each one matches the V5.03 file.

## Install

1. Do steps 1 and 2 of [Install on macOS](install.md).
2. Copy this line, paste it into the Terminal window, and press Return:

    ```bash
    curl -fsSL https://kvmet.github.io/k5mme_pocketcw/install.sh | sh -s stock
    ```

3. The script asks whether the device has had the V5.03 images installed before. If you are not sure, press Return: the script then installs the images first.
4. Follow the steps the script prints. With the images, the script installs two files, and you put the device in update mode once for each file.
5. When the script says "Finished", unplug the device, then click the dial to turn it on.

To install PocketCW again, follow [Install on macOS](install.md). PocketCW needs no `F` file.

## The beta for board 2.63

On a board 2.63, you can use the beta V5.03.13 in place of the V5.03 `A000014.bin`. The install script does not download the beta.

1. Install the stock firmware with the steps above, so the device has the V5.03 images.
2. In [the HamCube files](https://github.com/admvip/CW-Pokemon-Infomation), open the folder "CW Pokemon Beta Upgrade package", download `V5.03.13Beta.zip`, and unzip it. It holds only `A000014.bin`.
3. In Terminal, download the PocketCW install tool, and make it a program you can run:

    ```bash
    curl -fsSLo ~/Downloads/pocketcw-flash https://github.com/kvmet/k5mme_pocketcw/releases/latest/download/pocketcw-flash
    ```

    ```bash
    chmod +x ~/Downloads/pocketcw-flash
    ```

4. Run the tool on the beta file. Replace the path with the place of your file:

    ```bash
    ~/Downloads/pocketcw-flash ~/Downloads/V5.03.13Beta/A000014.bin
    ```

5. Follow the steps the tool prints.
