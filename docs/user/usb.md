# USB keyboard

The device can show up on a computer, phone or tablet as a USB keyboard and a serial port. Set USB OUT on the KEYER tab of the [settings](settings.md#keyer-tab).

| USB OUT | What the computer gets |
| - | - |
| OFF | Nothing. The USB port only charges the battery. This is the default. |
| TEXT | Each character you key, typed as text. |
| VBAND | Your keying as the Ctrl keys, for [VBand](https://hamradio.solutions/vband/). |
| SERIAL | No typing. A logging program uses the serial port as a [WinKeyer](#winkeyer). |
| RADIO | Each character the decoder board of the Pro model hears, typed as text. |

## Connect

1. Turn the device on.
2. Plug the USB cable into the device and the computer.

The computer finds a keyboard named PocketCW. It needs no driver. The first time, macOS can open its Keyboard Setup Assistant. Close it.

If the device is plugged into USB when you turn it on, it can start in update mode. See [Troubleshooting](troubleshooting.md#the-device-shows-the-update-screen-when-you-only-want-to-use-it).

## TEXT

The device types each character that the decoder reads from your key. Use it with any program that takes typed text, such as a copy trainer or a text editor.

- A character types after the gap that ends it, as it shows on the screen.
- A word gap types one space.
- Letters type in lowercase.
- A pattern that is not a character types `*`. A prosign types between angle brackets, such as `<AR>`, as on the screen.
- It works on every screen, with every key type.

### Keyboard layout

The device sends the keys of a US keyboard, and the computer turns each key into a character with its own layout setting.

- With a US layout, every character types correctly.
- With another layout, letters and digits are mostly correct, but punctuation such as `/`, `?`, `@` and `"` can type other characters. Some layouts also exchange Y and Z.

For exact text, set the computer's keyboard layout to US.

### Phones and tablets

While a USB keyboard is connected, an iPhone or iPad hides its on-screen keyboard. To charge the device from a phone or tablet, set USB OUT to OFF.

## RADIO

The device types each character that it decodes from the [decoder board](radio.md). The text types the same way as TEXT, and your keying types nothing.

- USB OUT offers RADIO only while DECODER BOARD is ON. If you set DECODER BOARD to OFF, USB OUT changes to OFF.
- The device decodes the board on every screen, not only on the RADIO screen.

## VBAND

VBAND sends your keying the way the VBand USB adapter does. In VBand, set the key type to match MODE on the KEYER tab.

| MODE | The device sends | Set VBand to |
| - | - | - |
| IAMBIC A, IAMBIC B | The dot lever holds Left Ctrl, and the dash lever holds Right Ctrl. PADDLES sets which lever is which. | An iambic paddle. VBand times the elements with its own keyer. |
| STRAIGHT | Left Ctrl, held while the device keys. This works for a straight key and a sideswiper. | A straight key. |

## VBand CW Hotline

VBand's page can also take your keying over the serial port, as it does from a CW Hotline. VBand does not document this, so a change to VBand can stop it.

1. Plug the device into a computer. USB OUT can be any setting but OFF.
2. Open VBand in Chrome, Edge, or Firefox 151 or later. Safari cannot open a serial port.
3. Click Connect to CW Hotline, and choose the PocketCW port.

The screen shows `HOTLINE ON` when VBand connects, and `HOTLINE OFF` when it disconnects or the browser closes the port.

- The device sends what keys the radio: your keying, `SEND` text from the serial port, and memories.
- While VBand has the port, the device types nothing and sends no Ctrl keys, so VBand never gets your keying twice.
- VBand sends nothing back. You hear the other stations from VBand, not from the device.
- VBand passes your keying to a channel only while another station is in it. The Practice Channel and the Lobby do not pass it on.
- A space longer than 3 seconds goes to VBand as 3 seconds. VBand takes that as the start of a new over.
- If `HOTLINE ON` never shows, the browser has no serial port support, or VBand changed how it connects.

## WinKeyer

A logging program can key through the device as if it were a K1EL WinKeyer. The program sends text, and the device keys it on the func jack and the sidetone. Every setting but OFF has the serial port, but SERIAL types nothing, so it is the one to use.

1. Set USB OUT to SERIAL.
2. Plug the device into the computer.
3. In the logging program, choose a WinKeyer on the device's serial port. It is `/dev/cu.usbmodem` and a number on macOS, `/dev/ttyACM` and a number on Linux, and a COM port on Windows. The baud rate does not matter.

The screen shows `WINKEYER OPEN` when the program takes the port, and `WINKEYER CLOSED` when it lets go.

- The text keys at the program's speed. When the program sets a speed, SPEED on the KEYER tab changes to it and is saved.
- The text keys the func jack only when KEY OUT is ON and OUT TYPE is STRAIGHT.
- Touch your key to stop the program's text. Your keying goes to the radio as usual, and the device tells the program you broke in.
- The device sends back each character it keys, and each character you key, when the program asks for that.
- Tune from the program holds the key down.
- When you change SPEED on the device, the program sees the new speed, as from a WinKeyer's speed knob.
- If the program asks for WinKeyer pushbuttons, F1 to F4 are the WinKeyer's four buttons, which the program can map to its macros. Until the program lets go, the screen does not see F1 to F4; the dial still works.
- Your MODE, PADDLES, TONE and VOLUME settings stay in force. The device ignores the program's settings for these, and for weight, PTT, lead-in, Farnsworth and the WinKeyer's own message memories.

## Status message

`n USB KEYS LOST` means the device lost `n` characters or key changes on their way to the computer. See [Status messages](status-messages.md).
