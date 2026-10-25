# Install

This page installs PocketCW on a HamCube. An install writes the firmware, and when it needs it, the [content pack](#the-content-pack) with the words and voice before it. On Windows and Linux, you copy them to the device. On macOS, an install script does the copy, because macOS writes its own files to the device, and those files break the install.

PocketCW was tested on a Mini with board version 2.64. Other board versions and the Pro may work, but nobody has tried them yet.

## What you need

- A computer with Windows, Linux, or macOS 10.15 or later on an Intel or Apple chip.
- A USB cable that carries data. Some cables only charge. If the computer never finds the device, try a different cable.
- About 10 minutes. The content pack is about 8 MB and takes several minutes to install.

## The content pack

The content pack holds the voice and the words for the drills. Install it:

- on the first PocketCW install on a device. Without an F file first, a first install stops on the update screen.
- when the device stays on the update screen after an install.
- when the content pack changed since you last installed it. The notes of each [release](https://github.com/kvmet/k5mme_pocketcw/releases) name the release that last changed it.
- when NINJA or HEAD shows `CONTENT PACK MISSING: INSTALL IT`.

Otherwise, install the firmware alone.

If you might go back to the [stock firmware](stock-firmware.md), know this first: the content pack can write over the stock firmware's contact log, and that log is then lost. Where the stock firmware keeps its log is not known yet. A first install needs the content pack all the same.

## Is it safe?

The device has a small built-in program, the bootloader, that installs firmware. This install never touches the bootloader: PocketCW's files install after it, and the macOS install tool refuses any file that would write over it. If an install goes wrong, the bootloader still works, and you can run the install again.

To go back to the original firmware, see [Go back to the stock firmware](stock-firmware.md). That page installs the published stock version, V5.03. Some devices come from the factory with a newer, unpublished version, and the install replaces it for good. On board 2.64, V5.03 has no sound.

## Update mode

The device takes new firmware in update mode. To start update mode:

1. Turn the device off, and unplug its USB cable. To turn it off, hold the dial down for about 2 seconds.
2. Find F1 and F4. They are two of the four buttons beside the screen: F1 is the top one, and F4 is the bottom one.
3. Hold F1 and F4 down.
4. While you hold them, hold the dial down too.
5. When the firmware update screen shows, let go of all three.

In update mode, the device shows up on the computer as a drive named POKEMON-2 or HAM MODULE. The drive holds one file, `READY.TXT`.

## Windows and Linux

Nobody has tested a Linux install yet. The bootloader needs the file's name to reach the drive before the file's data. Windows writes the name first. If a Linux install does not work, report it on the [issue tracker](https://github.com/kvmet/k5mme_pocketcw/issues).

1. Download `A000014.bin` from the newest [release](https://github.com/kvmet/k5mme_pocketcw/releases/latest). If you install the [content pack](#the-content-pack), download `F000400.bin` too.
2. Put the device in [update mode](#update-mode).
3. Plug the USB cable into the device and the computer. The drive opens.
4. To install the content pack, copy `F000400.bin` to the drive. Wait until the drive closes and opens again with only `READY.TXT`. This can take several minutes. Then hold F4 to reset the drive. Without the content pack, go to step 5.
5. Copy `A000014.bin` to the drive.
6. Wait about 10 seconds. The drive closes, then opens again with only `READY.TXT`.
7. Unplug the device, then click the dial to turn it on.

On Linux, run `sync` after each copy, so the file goes to the device at once.

If the drive holds a file other than `READY.TXT` before the copy, hold F4 to reset the drive.

## macOS

### Start the script

1. Turn the device off, and unplug its USB cable.
2. Open Terminal. Press Command and Space together, type Terminal, and press Return.
3. Copy this line, paste it into the Terminal window, and press Return:

```bash
curl -fsSL https://kvmet.github.io/k5mme_pocketcw/install.sh | sh
```

The script downloads the newest PocketCW release and the install tool. It asks:

```
Install the content pack? [y/N]
```

Type `y` and press Return to install the [content pack](#the-content-pack), then the firmware. Both install in one update mode session, so you put the device in update mode only once. Press Return alone to install only the firmware.

The script then says "Waiting for the device".

### Plug in the device

1. Put the device in [update mode](#update-mode). If it already shows the firmware update screen, go to the next step.
2. Plug the USB cable into the device and the Mac. After a few seconds, the script says "Found the device".
3. macOS asks for your password. Type your Mac login password, and click OK. The install writes directly to the device, and macOS asks for your password before it allows that. macOS can ask again for the second file.

### Finish

The content pack takes several minutes, and the firmware about 10 seconds. The script shows this when the install works:

```
Finished. Unplug the device.
Then click the dial to turn it on.
```

Unplug the device, then click the dial to turn it on.

### While the install runs

- Do not unplug the device until the script says "Finished".
- When the script finds a problem, it prints `Problem:` and the reason, then waits. Fix the cause, press Return, and follow the steps it prints again.
- macOS can show a message about a disk it cannot read. Click Ignore. If you click Initialize by mistake, close the Disk Utility window that opens, and change nothing.
- To stop the script at any time, press Control and C together.

## Check the version

On the KEYER screen, press F4 (MENU). Turn the dial to SYSTEM, and click the dial. The FIRMWARE row shows the version, such as `v0.3.0`.

If something goes wrong, see [Troubleshooting](troubleshooting.md).

## Release files

Each [release](https://github.com/kvmet/k5mme_pocketcw/releases) has these files:

| File | What it is |
| - | - |
| `A000014.bin` | The firmware. |
| `F000400.bin` | The content pack: the words and voice for the drills, for the device's memory chip. When an install needs it, it installs before the firmware. See [The content pack](#the-content-pack). |
| `pocketcw-flash` | The macOS install tool that the script runs. |
| `LICENSE` | The PocketCW license, GPL-3.0-or-later. |
| `B612-OFL.txt` | The license of the B612 Mono font. |
| `THIRD-PARTY.txt` | The licenses of the code libraries in the firmware, and the credits for the word list and voice in the content pack. |
