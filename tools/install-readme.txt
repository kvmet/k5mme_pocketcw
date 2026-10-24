POCKETCW @VERSION@ FOR THE HAMCUBE CW POKEMON MINI
Install guide for macOS

This folder installs PocketCW, free replacement firmware, on a HamCube
"CW Pokemon Mini" Morse code trainer. It was tested on board version 2.64.
Other board versions may work, but nobody has tried them yet.

PocketCW is not made by HamCube, and is not affiliated with HamCube or
Nintendo.


WHAT IS IN THIS FOLDER

  A000014.bin     The firmware.
  flash.sh        The install script. It runs the other two programs for you.
  iapwrite.py     Writes the firmware file to the device.
  nomount         Stops macOS from opening the device's update drive while
                  the install runs.
  README.txt      This guide.
  LICENSE         The license of PocketCW (GNU GPL version 3 or later).
  B612-OFL.txt    The license of the B612 Mono font that the firmware uses.
  THIRD-PARTY.txt The licenses of other code that is built into the firmware.


WHAT YOU NEED

  - A Mac with an Intel or Apple chip, macOS 10.15 or later.
  - A USB cable that carries data. Some cables only charge; if the Mac never
    finds the device, try a different cable.
  - Your Mac login password. The install writes directly to the device, and
    macOS asks for your password before it allows that.
  - About 5 minutes.


IS IT SAFE?

The device has a small built-in program, the bootloader, that installs
firmware. This install never touches the bootloader; iapwrite.py refuses any
file that would. So if an install goes wrong, the bootloader still works, and
you can run the install again.

To go back to the original firmware, you need two update files from HamCube.
See "Going back to the original firmware" below.


HOW TO INSTALL

1. Unzip the download, if your Mac has not already done that. You get a folder
   named pocketcw-@VERSION@.

2. Turn the device off, and unplug its USB cable. To turn it off, hold the
   dial down for about 2 seconds.

3. Open Terminal. Press Command and Space together, type Terminal, and press
   Return.

4. In the Terminal window, type sh and then a space. Do not press Return yet.

5. Drag the file flash.sh from the folder into the Terminal window. Its full
   location appears after "sh ". Now press Return.

6. If a box appears that offers to install the "command line developer tools",
   click Install and wait until it finishes. This can take several minutes.
   Then go back to step 4. You only need to do this once.

7. The script now says "Waiting for the device". Put the device in update
   mode, then plug it in:
   a. Find F1 and F4. They are two of the four buttons beside the screen:
      F1 is the top one, and F4 is the bottom one.
   b. Hold F1 and F4 down.
   c. While you hold them, hold the dial down too.
   d. When the firmware update screen shows, let go of all three.
   e. Plug the USB cable into the device and the Mac.

   After a few seconds, the Terminal window says "Found the device".
   If the device already shows the firmware update screen, start at e.

8. Terminal asks for your password. Type your Mac login password and press
   Return. Nothing shows while you type; that is normal.

9. Wait about 10 seconds. When it works, you see:

      status now 'SUCCESS TXT'
      done: the device finished the update
      Finished. Unplug the device, then click the dial to turn it on.

   The device can also drop off USB for a moment and come back. Then you see
   "device dropped off USB" and then "done". That is a success too.

10. Unplug the device, then click the dial to turn it on. To check the
    version, press F4 (MENU), turn the dial to SYSTEM, and click the dial.
    The FIRMWARE row shows @VERSION@.


WHILE THE INSTALL RUNS

  - Do not unplug the device until the script says "Finished".
  - macOS may show a message about a disk it cannot read. Click Ignore. If you
    click Initialize by mistake, close the Disk Utility window that opens and
    change nothing.
  - To stop the script at any time, press Control and C together.


TROUBLESHOOTING

"the update drive is already mounted"
  The device was in update mode before the script started, so macOS opened its
  drive. Eject the drive in Finder (it is named POKEMON-2 or HAM MODULE),
  unplug the device, and start again at step 4.

"no update drive appeared in 180 seconds"
  The Mac did not find the device in update mode. Check that:
  - the device showed the firmware update screen before you plugged it in
    (step 7);
  - the cable carries data (try another cable or another USB port).
  Then start again at step 4.

"drive status is ..., not Ready"
  The device's update drive is not ready to take a file. Unplug the device,
  then start again at step 4.

"root directory has entries after the volume label; replug the device"
  macOS wrote something to the drive before the script could stop it. Unplug
  the device, then start again at step 4.

"status still Ready after 10 s; the device did not take the file"
  The write did not finish. Unplug the device, then start again at step 4.

"Python 3 is not installed"
  Do step 6. If no box appears, type this in Terminal and press Return:
    xcode-select --install
  Then start again at step 4.

"nomount" cannot be opened, or "the developer cannot be verified"
  flash.sh normally prevents this. If you see it, type this in Terminal, then
  a space, then drag the pocketcw-@VERSION@ folder into the window, and press
  Return:
    xattr -dr com.apple.quarantine
  Then start again at step 4.

"Sorry, try again" after your password
  The password was wrong. Type your Mac login password again.

The device shows the update drive when you only wanted to use it
  If the battery is full and the device is plugged into USB, it sometimes
  starts in update mode. Unplug USB and click the dial to turn it on.

The screen is turned on its side, or shows the picture in strips
  Older boards have two kinds of screen, and the firmware cannot always tell
  which one it has. To switch to the other kind:
  a. Hold the dial down for about 2 seconds. The device turns off, even
     though you cannot read the screen.
  b. Hold F2 down. F2 is the second button from the top.
  c. While you hold F2, click the dial to turn the device on.
  d. Let go of F2 when the screen shows a picture.
  The device remembers the choice. To switch back, do the same steps again.

The device does not start after an install
  Run the install again from step 2. The bootloader is never changed, so
  update mode always works.


GOING BACK TO THE ORIGINAL FIRMWARE

The original firmware needs two files from HamCube: F0000C0.bin (the screen
images) and A000014.bin (the firmware). Install F0000C0.bin first. With an
older F0000C0.bin, the original firmware shows a scrambled screen.

The original firmware does not fully support boards with an IPS screen (2.63
and later). Before you go back, read the user manual page:
https://github.com/kvmet/k5mme_pocketcw/blob/main/docs/user/stock-firmware.md

1. Download both files from the folder "CW Pokemon V5.03 Upgrade package" at
   https://github.com/admvip/CW-Pokemon-Infomation
2. Do steps 2 to 4 above. Then drag flash.sh into the window, type a space,
   drag F0000C0.bin into the window too, and press Return. Continue from
   step 7 to step 9. Then unplug the device, but do not turn it on yet.
3. Do step 2 again with A000014.bin in place of F0000C0.bin. Then unplug the
   device, and click the dial to turn it on.


QUESTIONS OR PROBLEMS

Open an issue at https://github.com/kvmet/k5mme_pocketcw/issues
Tell us your board version and what the Terminal window showed; you can copy
the text from it.
