# Hardware reference

These pages describe the HamCube CW Pokemon Mini as PocketCW uses it. The facts about the hardware and the vendor firmware were found by static analysis and emulation of vendor firmware V5.03. Addresses such as 0x080185E0 point into the vendor app image, which starts at 0x08005000.

PocketCW was tested on board version 2.64 only. No Pro model with the decoder board has been checked.

## Markers

| Marker | Meaning |
| - | - |
| Confirmed on 2.64 | Checked on a board 2.64 unit. |
| Confirmed | The vendor code shows it directly. |
| Confirmed in code | PocketCW does it, and the code shows it. Not checked on a board. |
| Likely | Inferred. Not checked on a board. |
| Role unknown | The firmware drives the pin, but what the pin does is not known. |

## Pages

- [MCU and clock](mcu.md): the chip, option bytes, clock setup and debug access.
- [Flash layout and bootloader](flash-layout.md): what lives where, update files, and how the bootloader takes them.
- [Pins](pins.md): every pin and its use.
- [Buttons](buttons.md): the F buttons, the dial, and their timing.
- [Key and func jacks](jacks.md): the paddle inputs, the keyed outputs, and the LED.
- [Display](display.md): the two screen types and how PocketCW picks one.
- [Audio](audio.md): the PA8 audio output, the two codecs, and the speaker amplifier.
- [Storage](storage.md): the EEPROM layout, the record format, and the SPI flash.
- [Battery](battery.md): the battery reading and the gauge.
- [Power](power.md): power hold, standby, and the fault screen.
- [Decoder board](decoder-board.md): the Pro model's decoder board and its serial protocol.
- [Board variants](board-variants.md): what differs between board versions.
- [Vendor firmware](vendor-firmware.md): resource formats, slow buttons, and the vendor's features.
