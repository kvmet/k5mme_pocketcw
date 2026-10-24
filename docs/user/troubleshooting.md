# Troubleshooting

This page lists install problems first, then problems on the device.

## Install messages

These messages come from the macOS install tool. When it finds a problem, it prints `Problem:` and one of these messages, then waits. Do what the message says, press Return, and follow the steps the tool prints again.

### "could not download"

The script could not reach GitHub. Check that the Mac is connected to the internet, and run the line from [Start the script](install.md#start-the-script) again.

### "the update drive is mounted"

The device was in update mode before the script started, so macOS opened its drive. Eject the drive in Finder. Its name is POKEMON-2 or HAM MODULE. Unplug the device, and press Return.

### "no update drive appeared in 180 seconds"

The Mac did not find the device in update mode. Check these two things:

- The device showed the firmware update screen before you plugged it in. See [Update mode](install.md#update-mode).
- The cable carries data. Try another cable or another USB port.

Then press Return.

### "macOS did not allow writing to the device"

The password box was closed with Cancel. Press Return, and type your Mac login password when macOS asks. If the box shakes, the password was wrong: type it again.

### "the device status is ..., not Ready"

The update drive is not ready to take a file. Hold F4, which resets the drive, then press Return. If the message comes back, unplug the device and press Return.

### "the drive already holds a file"

macOS wrote something to the drive before the install tool could stop it. Hold F4 to reset the drive, then press Return.

### "the status stayed Ready for 10 s; the device did not take the file"

The write did not finish. Press Return to install again.

### "the device dropped off USB before showing Success"

The tool cannot tell whether the write finished. Press Return to install again.

### "the device status is ..., not Success"

The device reported a problem with the file. Press Return to install again. If the message comes back, report it on the [issue tracker](https://github.com/kvmet/k5mme_pocketcw/issues) with the status the message shows.

### "is not the V5.03 file this script expects"

The script restores the stock firmware only from the exact V5.03 files. The downloaded file is different, so the script stopped before it changed anything. Report it on the [issue tracker](https://github.com/kvmet/k5mme_pocketcw/issues).

## Device problems

### The device shows the update screen when you only want to use it

If the battery is full and the device is plugged into USB, it sometimes starts in update mode. Unplug USB and click the dial to turn it on.

If the device starts in update mode every time, even unplugged, the last install did not finish. Install again, with the [first install](install.md#windows-and-linux) steps.

### The screen is turned on its side, or shows the picture in strips

Boards use one of two kinds of screen, and the firmware cannot always tell which one it has. To switch to the other kind:

1. Hold the dial down for about 2 seconds. The device turns off, even though you cannot read the screen.
2. Hold F2 down. F2 is the second button from the top.
3. While you hold F2, click the dial to turn the device on.
4. Let go of F2 when the screen shows a picture.

The device remembers the choice. To switch back, do the same steps again.

### The device does not start after an install

Run the install again from step 1. The install never changes the bootloader, so update mode always works.

### A red screen with an error

The firmware stopped on an error it cannot recover from. Click the dial to turn the device off. Then turn it on again. If the error comes back, copy the text on the screen into a report on the [issue tracker](https://github.com/kvmet/k5mme_pocketcw/issues).

### No sound

Check the volume: the VOL value on the KEYER screen, or VOLUME on the SOUND tab of the settings. At 0 the sound is off. A plug in the headphone jack turns the speaker off.

### A message on the bottom line of the screen

See [Status messages](status-messages.md).
