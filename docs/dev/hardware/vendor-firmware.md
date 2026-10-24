# Vendor firmware

PocketCW does not include or need any vendor file. This page records what is known about the vendor firmware, for comparison and for going back to stock. See [Go back to the stock firmware](../../user/stock-firmware.md).

## Resource formats

- Images: an Image2Lcd header of 8 bytes (`10 10`, width as big endian 16 bits, height as big endian 16 bits, `01 1B`), then RGB565 big endian, row major. This is the byte order the ILI9341 expects. Code refers to an image as `0xC0000000 | spi_flash_address`.
- Fonts (`F003800.bin`): starts with the text `This is an image as font!`, then a table of (width, height) byte pairs. Not decoded further. PocketCW uses its own font format. See [Fonts](../fonts.md).

## Why the vendor buttons feel slow

1. Buttons use the `multi_button` library with only single click and long press handlers. The tick runs every 11 ms. A click is reported about 110 ms after release. A second press inside that window becomes a double click, which has no handler, so the press is lost.
2. The UI loops clear all key flags before and after each redraw, so a press during a redraw is lost.
3. Images are read from SPI flash one byte at a time, without DMA, on the bus the LCD also uses. This makes redraws slow.

Paddle keying runs in the 1 ms SysTick interrupt, so the UI never delays it.

PocketCW sends a button event at the press, and reads buttons in its 1 ms tick. See [Buttons](buttons.md).

## Vendor modes and settings

- Training: CW Memory, Code Listen, Koch Listen, Code Send, Followme, Chinese Code, Send Game, Listen Game.
- Settings: F1 to F8 hotkey text, callsign, backlight, volume, decode lock and tolerance, auto shutdown, language, factory reset, date and time, key type (Auto, Hand, KeyB, Bug), Iambic A or B, dit and dah swap, tone, long and short code, decoder board power, Chinese decode, repeat group and interval, contact log (entry, view, ADIF export, init).

The vendor settings live at EEPROM 0x000 to 0x367. PocketCW never writes there, so the stock settings are still there after you go back. See [Storage](storage.md).
