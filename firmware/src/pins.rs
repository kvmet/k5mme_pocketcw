//! Pin numbers, found on board 2.64; the vendor firmware uses the same pins from 2.60. See
//! HARDWARE.md for how each was found.

// Port A
pub const POWER_BUTTON: u32 = 0; // dial click, active high, standby wakeup pin
pub const BATTERY: u32 = 1; // ADC1 channel 1, battery through a 2:1 divider
pub const KEY_OUT_TIP: u32 = 2; // "func" jack tip, high = closed
pub const KEY_OUT_RING: u32 = 3; // "func" jack ring, high = closed
pub const F_KEYS: [u32; 4] = [4, 5, 6, 7]; // F1..F4, active high
pub const SIDETONE: u32 = 8; // TMR1 channel 1
pub const DECODER_TX: u32 = 9; // USART1 to the Pro decoder board
pub const DECODER_RX: u32 = 10;
pub const LCD_RESET: u32 = 15;

// Port B
pub const RING: u32 = 0; // "key" jack ring, active low
pub const TIP: u32 = 1; // "key" jack tip, active low
pub const LED: u32 = 2; // output LED, active high
pub const LCD_SCK: u32 = 3;
pub const LCD_MISO: u32 = 4; // SPI1 MISO, shared with the SPI flash; the panel may answer here
pub const LCD_MOSI: u32 = 5;
pub const LCD_CS: u32 = 6;
pub const LCD_DC: u32 = 7; // low = command
pub const BACKLIGHT: u32 = 8;
pub const FLASH_CS: u32 = 9; // SPI flash shares the LCD bus
pub const I2C_SCL: u32 = 10;
pub const I2C_SDA: u32 = 11;
pub const SPEAKER_AMP: u32 = 12;
pub const DECODER_POWER: u32 = 13; // Pro decoder board power, high = on
pub const DECODER_TONE: u32 = 14; // Pro decoder board, high while it hears a tone
pub const PB15: u32 = 15; // role unknown; the vendor holds it low from before LCD init

// Port C
pub const POWER_HOLD: u32 = 13; // high keeps the board powered after the dial click
pub const DIAL_LEFT: u32 = 14; // active high; also closes with the dial click
pub const DIAL_RIGHT: u32 = 15; // active high
