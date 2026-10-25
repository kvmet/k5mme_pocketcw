# Go back to the stock firmware

This page puts the original HamCube firmware, version V5.03, back on the device. PocketCW does not include the HamCube files. The install script downloads them from a separate source.

## Before you start

Board 2.64 has a different sound chip (an ES8388). The published stock versions, V5.03 and the beta V5.03.13, have no setup for that chip, so the speaker is silent under them. At least one board 2.64 came from the factory with V5.03.15, a version HamCube has not published, and its sound worked. If your device came with a stock version newer than V5.03.13, you cannot get that version back after you install PocketCW.

Boards from 2.63 have an IPS screen. Stock V5.03 has no setup for the IPS screen, so it shows that screen on its side, in strips. The beta version V5.03.13 sets up the IPS screen, but only on boards that read as version 2.63. No published stock version fully supports board 2.64.

The SYSTEM screen of PocketCW shows your board version on the BOARD row.

Stock firmware keeps its settings in a different part of the memory chip than PocketCW. If you install PocketCW again later, it finds its settings again.

## The two stock files

The stock firmware is two files from HamCube's V5.03 upgrade package:

- `F0000C0.bin` holds the images of the stock screens. The stock firmware draws its screens from the device's image memory, and an older V5.02 copy of this file scrambles the stock screen. PocketCW never reads the image memory, so PocketCW works with any version of this file.
- `A000014.bin` is the stock firmware.

Install both files, `F0000C0.bin` first, as HamCube says. The bootloader can fail an `A000014.bin` install that comes with no `F` file before it, and leave the device on the update screen.

PocketCW does not include these files. The install script downloads them from [github.com/admvip/CW-Pokemon-Infomation](https://github.com/admvip/CW-Pokemon-Infomation), and checks that each one matches the V5.03 file.

## Install

### Windows and Linux

1. Download `F0000C0.bin` and `A000014.bin` from the "CW Pokemon V5.03 Upgrade package" folder of [the HamCube files](https://github.com/admvip/CW-Pokemon-Infomation).
2. Put the device in [update mode](install.md#update-mode), and plug it in. The drive opens.
3. Copy `F0000C0.bin` to the drive. Wait until the drive closes and opens again with only `READY.TXT`. Then hold F4 to reset the drive.
4. Copy `A000014.bin` to the drive.
5. Wait about 10 seconds. The drive closes, then opens again with only `READY.TXT`.
6. Unplug the device, then click the dial to turn it on.

On Linux, run `sync` after each copy, so the file goes to the device at once.

### macOS

Start the script as in [Start the script](install.md#start-the-script), with this line in place of the PocketCW line:

```bash
curl -fsSL https://kvmet.github.io/k5mme_pocketcw/install.sh | sh -s stock
```

Then:

1. Follow the steps the script prints. The script installs both files in one update mode session, so you put the device in update mode only once. macOS can ask for your password once for each file.
2. When the script says "Finished", unplug the device, then click the dial to turn it on.

To install PocketCW again, follow [Install](install.md).

## The beta for board 2.63

On a board 2.63, you can use the beta V5.03.13 in place of the V5.03 `A000014.bin`. The install script does not download the beta.

1. Install the stock firmware with the steps above, so the device has the V5.03 images.
2. In [the HamCube files](https://github.com/admvip/CW-Pokemon-Infomation), open the folder "CW Pokemon Beta Upgrade package", download `V5.03.13Beta.zip`, and unzip it. It holds only `A000014.bin`.
3. Copy that `A000014.bin` to the update drive, as in [Install](install.md#windows-and-linux). On a Mac, use the install tool below in place of a copy.

On a Mac, download the PocketCW install tool in Terminal, and make it a program you can run:

```bash
curl -fsSLo ~/Downloads/pocketcw-flash https://github.com/kvmet/k5mme_pocketcw/releases/latest/download/pocketcw-flash
```

```bash
chmod +x ~/Downloads/pocketcw-flash
```

Then run the tool on the beta file, and follow the steps it prints. Replace the path with the place of your file:

```bash
~/Downloads/pocketcw-flash ~/Downloads/V5.03.13Beta/A000014.bin
```
