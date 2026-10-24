# Pins

`firmware/src/pins.rs` names every pin. The vendor firmware uses the same pins from board 2.60.

| Pin | Name in pins.rs | Function | Status |
| - | - | - | - |
| PA0 | `POWER_BUTTON` | Dial click, the power button. Active high. The vendor sets PA0 as the standby wakeup pin before power off. PC14 also reads high while the dial is pressed. | Confirmed on 2.64 |
| PA1 | `BATTERY` | ADC1 channel 1: the battery through a 2:1 divider, read against the 3.3 V supply. See [Battery](battery.md). | Confirmed |
| PA2 | `KEY_OUT_TIP` | Func jack tip, keyed output, high = closed. Keys a radio's or keyer's dit input, or a straight key input. | Confirmed on 2.64 |
| PA3 | `KEY_OUT_RING` | Func jack ring, keyed output, high = closed. With PA2 it passes paddle contacts to an external keyer: dit on PA2, dah on PA3. The vendor drives PA3 only on board 2.60. | Confirmed on 2.64 |
| PA4, PA5, PA6, PA7 | `F_KEYS` | F1, F2, F3, F4. Active high, internal pull-down. | Confirmed on 2.64 |
| PA8 | `SIDETONE` | Audio, TMR1 channel 1. See [Audio](audio.md). | Confirmed on 2.64 |
| PA9, PA10 | `DECODER_TX`, `DECODER_RX` | USART1 TX and RX to the decoder board, DMA on RX. | Confirmed |
| PA11, PA12 | none | USB device. | Confirmed (USB peripheral in use) |
| PA15 | `LCD_RESET` | LCD reset. | Confirmed |
| PB0 | `RING` | Key jack ring. Active low, internal pull-up. See [Key and func jacks](jacks.md). | Confirmed on 2.64 |
| PB1 | `TIP` | Key jack tip. Active low, internal pull-up. | Confirmed on 2.64 |
| PB2 | `LED` | Output LED, active high. | Confirmed on 2.64 |
| PB3, PB4, PB5 | `LCD_SCK`, `LCD_MISO`, `LCD_MOSI` | SPI1 SCK, MISO and MOSI, remapped. The LCD and the SPI flash share the bus. | Confirmed |
| PB6 | `LCD_CS` | LCD chip select. | Confirmed |
| PB7 | `LCD_DC` | LCD data or command, low = command. | Confirmed |
| PB8 | `BACKLIGHT` | LCD backlight, TMR4 channel 3 PWM at 20 kHz, duty 0 to 100. | Confirmed |
| PB9 | `FLASH_CS` | SPI flash chip select. Read command 0x03, 24 bit address. | Confirmed |
| PB10, PB11 | `I2C_SCL`, `I2C_SDA` | I2C2 SCL and SDA, to the EEPROM and the audio codec. PocketCW drives them in software at about 65 kHz. | Confirmed |
| PB12 | `SPEAKER_AMP` | Speaker amplifier enable. See [Audio](audio.md). | Confirmed |
| PB13 | `DECODER_POWER` | Decoder board power, high = on. See [Decoder board](decoder-board.md). | Confirmed in code |
| PB14 | `DECODER_TONE` | Decoder board tone detect, high while the board hears a tone. Input, pull-down. | Confirmed in code |
| PB15 | `PB15` | Output. The vendor drives it low at boot (0x08015598) and again after LCD init. PocketCW drives it low at boot. | Role unknown |
| PC13 | `POWER_HOLD` | Power hold, high = stay on. See [Power](power.md). | Confirmed on 2.64 |
| PC14, PC15 | `DIAL_LEFT`, `DIAL_RIGHT` | Dial left and dial right. Active high, internal pull-down. | Confirmed on 2.64 |

Without the pull-ups, the key jack inputs float and read at random.
