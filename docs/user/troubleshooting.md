# Troubleshooting

This page lists install problems first, then problems on the device. Step numbers refer to [Install on macOS](install.md).

## Install messages

### "could not download"

The script could not reach GitHub. Check that the Mac is connected to the internet, and run the line from step 3 again.

### "the update drive is already mounted"

The device was in update mode before the script started, so macOS opened its drive. Eject the drive in Finder. Its name is POKEMON-2 or HAM MODULE. Unplug the device, and start again at step 3.

### "no update drive appeared in 180 seconds"

The Mac did not find the device in update mode. Check these two things:

- The device showed the firmware update screen before you plugged it in (step 4).
- The cable carries data. Try another cable or another USB port.

Then start again at step 3.

### "macOS did not allow writing to the device"

The password box was closed with Cancel. Start again at step 3, and type your Mac login password when macOS asks. If the box shakes, the password was wrong: type it again.

### "drive status is ..., not Ready"

The update drive is not ready to take a file. The message tells you to hold F4, which resets the drive. If the message comes back, unplug the device and start again at step 3.

### "root directory has entries after the volume label; replug the device"

macOS wrote something to the drive before the install tool could stop it. Unplug the device, and start again at step 3.

### "status still Ready after 10 s; the device did not take the file"

The write did not finish. Unplug the device, and start again at step 3.

### "the device did not return within 15 s"

The device dropped off USB after the write and did not come back in time. Unplug the device and turn it on. If the SYSTEM screen does not show the new version, run the install again from step 1.

### "is not the V5.03 file this script expects"

The script restores the stock firmware only from the exact V5.03 files. The downloaded file is different, so the script stopped before it changed anything. Report it on the [issue tracker](https://github.com/kvmet/k5mme_pocketcw/issues).

## Device problems

### The device shows the update screen when you only want to use it

If the battery is full and the device is plugged into USB, it sometimes starts in update mode. Unplug USB and click the dial to turn it on.

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
