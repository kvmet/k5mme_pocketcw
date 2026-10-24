# USB keyboard

The device can show up on a computer, phone or tablet as a USB keyboard. Set USB OUT on the KEYER tab of the [settings](settings.md#keyer-tab).

| USB OUT | What the computer gets |
| - | - |
| OFF | Nothing. The USB port only charges the battery. This is the default. |
| TEXT | Each character you key, typed as text. |
| VBAND | Your keying as the Ctrl keys, for [VBand](https://hamradio.solutions/vband/). |

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

## VBAND

VBAND sends your keying the way the VBand USB adapter does. In VBand, set the key type to match MODE on the KEYER tab.

| MODE | The device sends | Set VBand to |
| - | - | - |
| IAMBIC A, IAMBIC B | The dot lever holds Left Ctrl, and the dash lever holds Right Ctrl. PADDLES sets which lever is which. | An iambic paddle. VBand times the elements with its own keyer. |
| STRAIGHT | Left Ctrl, held while the device keys. This works for a straight key and a sideswiper. | A straight key. |

## Status message

`n USB KEYS LOST` means the device lost `n` characters or key changes on their way to the computer. See [Status messages](status-messages.md).
