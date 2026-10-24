use at32f4xx_pac::at32f403a as pac;
use embedded_graphics::{
    pixelcolor::{Rgb565, raw::RawU16},
    prelude::*,
    primitives::Rectangle,
};

use pocketcw_logic::panel::{IdRead, Panel};

use crate::clock::{delay_ms, delay_us};
use crate::gpio::{self, port_a, port_b};
use crate::pins::{FLASH_CS, LCD_CS as CS, LCD_DC as DC, LCD_MISO as MISO, LCD_MOSI as MOSI, LCD_RESET as RESET, LCD_SCK as SCK};

pub const WIDTH: u32 = 320;
pub const HEIGHT: u32 = 240;

const SPI_DIV_2: u8 = 0b000;
const RDDID: u8 = 0x04;
/// One dummy clock, 24 ID bits, and 7 more so a wrong dummy count shows in the raw answer.
const ID_BITS: u32 = 32;

fn init_sequence(panel: Panel) -> &'static [(u8, &'static [u8], u32)] {
    match panel {
        Panel::Ips => IPS_INIT,
        Panel::Ili9341 => ILI9341_INIT,
    }
}

/// IPS panel init from vendor V5.03.13: (command, parameters, delay after in ms).
const IPS_INIT: &[(u8, &[u8], u32)] = &[
    (0x11, &[], 120), // sleep out
    (0x36, &[0xA0], 0), // MADCTL: landscape
    (0x3A, &[0x55], 0), // COLMOD: RGB565
    (0xB2, &[0x0C, 0x0C, 0x00, 0x33, 0x33], 0),
    (0xB7, &[0x46], 0),
    (0xBB, &[0x1B], 0),
    (0xC0, &[0x2C], 0),
    (0xC2, &[0x01], 0),
    (0xC3, &[0x0F], 0),
    (0xC4, &[0x20], 0),
    (0xC6, &[0x0F], 0),
    (0xD0, &[0xA4, 0xA1], 0),
    (0xD6, &[0xA1], 0),
    (0xE0, &[0xF0, 0x00, 0x06, 0x04, 0x05, 0x05, 0x31, 0x44, 0x48, 0x36, 0x12, 0x12, 0x2B, 0x34], 0),
    (0xE1, &[0xF0, 0x0B, 0x0F, 0x0F, 0x0D, 0x26, 0x31, 0x43, 0x47, 0x38, 0x14, 0x14, 0x2C, 0x32], 0),
    (0x21, &[], 0), // inversion on
    (0x29, &[], 0), // display on
];

/// ILI9341 panel init from vendor V5.03.13, in its order. Same layout as `IPS_INIT`.
const ILI9341_INIT: &[(u8, &[u8], u32)] = &[
    (0xCB, &[0x39, 0x2C, 0x00, 0x34, 0x02], 0),
    (0xCF, &[0x00, 0xC1, 0x30], 0),
    (0xE8, &[0x85, 0x00, 0x78], 0),
    (0xEA, &[0x00, 0x00], 0),
    (0xED, &[0x64, 0x03, 0x12, 0x81], 0),
    (0xF7, &[0x20], 0),
    (0xC0, &[0x23], 0),
    (0xC1, &[0x10], 0),
    (0xC5, &[0x3E, 0x28], 0),
    (0xC7, &[0x86], 0),
    (0x36, &[0xE8], 0), // MADCTL: landscape, BGR
    (0x2A, &[0x00, 0x00, 0x01, 0x3F], 0),
    (0x2B, &[0x00, 0x00, 0x00, 0xEF], 0),
    (0x2C, &[], 0),
    (0x3A, &[0x55], 0), // COLMOD: RGB565
    (0xB1, &[0x00, 0x18], 0),
    (0xB6, &[0x08, 0x82, 0x27], 0),
    (0xF2, &[0x00], 0),
    (0x26, &[0x01], 0),
    (0xE0, &[0x0F, 0x31, 0x2B, 0x0C, 0x0E, 0x08, 0x4E, 0xF1, 0x37, 0x07, 0x10, 0x03, 0x0E, 0x09, 0x00], 0),
    (0xE1, &[0x00, 0x0E, 0x14, 0x03, 0x11, 0x07, 0x31, 0xC1, 0x48, 0x08, 0x0F, 0x0C, 0x31, 0x36, 0x0F], 0),
    (0x11, &[], 120), // sleep out
    (0x29, &[], 0), // display on
];

pub struct Lcd {
    spi: pac::SPI1,
}

impl Lcd {
    /// Needs SPI1 remapped to PB3/PB4/PB5 and JTAG off, so PA15, PB3 and PB4 are free.
    pub fn new(spi: pac::SPI1) -> Self {
        let (gpioa, gpiob) = (port_a(), port_b());
        // Drive strengths as the vendor sets them.
        for pin in [CS, FLASH_CS] {
            gpio::set_high(gpiob, pin);
            gpio::configure(gpiob, pin, gpio::OUTPUT_LARGE);
        }
        gpio::configure(gpiob, DC, gpio::OUTPUT_LARGE);
        gpio::configure(gpioa, RESET, gpio::OUTPUT_LARGE);
        gpio::configure(gpiob, SCK, gpio::PERIPHERAL_OUTPUT_MAXIMUM);
        gpio::configure(gpiob, MOSI, gpio::PERIPHERAL_OUTPUT_MAXIMUM);
        // Mode 0, MSB first, APB2 / 2 = 18 MHz, the vendor speed for both panels. Software CS held high keeps the master from faulting.
        spi.ctrl1().write(|w| unsafe {
            w.msten().set_bit().swcsen().set_bit().swcsil().set_bit().mdiv2_0().bits(SPI_DIV_2)
        });
        spi.ctrl1().modify(|_, w| w.spien().set_bit());
        Lcd { spi }
    }

    pub fn reset(&mut self) {
        gpio::set_low(port_a(), RESET);
        delay_ms(100);
        gpio::set_high(port_a(), RESET);
        delay_ms(220);
    }

    /// The panel's answer to RDDID on PB4 and on PB5, in that order: a panel answers on its
    /// SDO pin when it has one wired, else on its bidirectional SDA. Bit-banged, since the SPI
    /// block cannot clock the single dummy bit. Call between `reset` and `init`.
    pub fn read_id(&mut self) -> [IdRead; 2] {
        let b = port_b();
        gpio::set_low(b, SCK);
        gpio::configure(b, SCK, gpio::OUTPUT_LARGE);
        let up = read_rddid(true);
        let down = read_rddid(false);
        gpio::configure(b, SCK, gpio::PERIPHERAL_OUTPUT_MAXIMUM);
        gpio::configure(b, MOSI, gpio::PERIPHERAL_OUTPUT_MAXIMUM);
        [0, 1].map(|pin| IdRead { pulled_up: up[pin], pulled_down: down[pin] })
    }

    pub fn init(&mut self, panel: Panel) {
        for &(command, params, wait) in init_sequence(panel) {
            self.command(command, params);
            delay_ms(wait);
        }
    }

    /// Display off, then sleep in. The panel keeps its RAM.
    pub fn sleep(&mut self) {
        self.command(0x28, &[]);
        self.command(0x10, &[]);
        delay_ms(5);
    }

    pub fn wake(&mut self) {
        self.command(0x11, &[]);
        delay_ms(120);
        self.command(0x29, &[]);
    }

    fn send(&mut self, byte: u8) {
        while self.spi.sts().read().tdbe().bit_is_clear() {}
        self.spi.dt().write(|w| unsafe { w.bits(byte as u16) });
    }

    fn wait_idle(&mut self) {
        while self.spi.sts().read().tdbe().bit_is_clear() {}
        while self.spi.sts().read().bf().bit_is_set() {}
    }

    /// CS rises after every byte, as in the vendor driver: a stray clock edge then spoils one
    /// byte, not the rest of the command.
    fn command(&mut self, command: u8, params: &[u8]) {
        gpio::set_low(port_b(), DC);
        self.byte(command);
        gpio::set_high(port_b(), DC);
        for &p in params {
            self.byte(p);
        }
    }

    fn byte(&mut self, byte: u8) {
        gpio::set_low(port_b(), CS);
        self.send(byte);
        self.wait_idle();
        gpio::set_high(port_b(), CS);
    }

    fn write_pixels(&mut self, area: &Rectangle, colors: impl IntoIterator<Item = Rgb565>) {
        let (x0, y0) = (area.top_left.x as u16, area.top_left.y as u16);
        let (x1, y1) = (x0 + area.size.width as u16 - 1, y0 + area.size.height as u16 - 1);
        self.command(0x2A, &[(x0 >> 8) as u8, x0 as u8, (x1 >> 8) as u8, x1 as u8]);
        self.command(0x2B, &[(y0 >> 8) as u8, y0 as u8, (y1 >> 8) as u8, y1 as u8]);
        gpio::set_low(port_b(), CS);
        gpio::set_low(port_b(), DC);
        self.send(0x2C);
        self.wait_idle();
        gpio::set_high(port_b(), DC);
        for color in colors {
            let raw = RawU16::from(color).into_inner();
            self.send((raw >> 8) as u8);
            self.send(raw as u8);
        }
        self.wait_idle();
        gpio::set_high(port_b(), CS);
    }
}

/// One RDDID read with PB4 and PB5 pulled up or down while the panel answers: the bits from PB4
/// and from PB5. Both controllers allow a read clock of 150 ns; a 2 us clock is far inside it.
fn read_rddid(pull_up: bool) -> [u32; 2] {
    let b = port_b();
    gpio::configure(b, MISO, gpio::INPUT_PULL);
    gpio::set(b, MISO, pull_up);
    gpio::configure(b, MOSI, gpio::OUTPUT_LARGE);
    gpio::set_low(b, DC);
    gpio::set_low(b, CS);
    for bit in (0..8).rev() {
        gpio::set(b, MOSI, RDDID >> bit & 1 != 0);
        delay_us(1);
        gpio::set_high(b, SCK);
        delay_us(1);
        if bit == 0 {
            // The panel starts driving at the next falling edge, so release SDA before it.
            gpio::configure(b, MOSI, gpio::INPUT_PULL);
            gpio::set(b, MOSI, pull_up);
        }
        gpio::set_low(b, SCK);
    }
    gpio::set_high(b, DC);
    let mut bits = [0u32; 2];
    for _ in 0..ID_BITS {
        // The panel shifts out on the falling edge; sample just before the rising edge.
        delay_us(1);
        let inputs = gpio::inputs(b);
        for (pin, bits) in [MISO, MOSI].into_iter().zip(&mut bits) {
            *bits = *bits << 1 | (inputs >> pin & 1) as u32;
        }
        gpio::set_high(b, SCK);
        delay_us(1);
        gpio::set_low(b, SCK);
    }
    gpio::set_high(b, CS);
    bits
}

impl OriginDimensions for Lcd {
    fn size(&self) -> Size {
        Size::new(WIDTH, HEIGHT)
    }
}

impl DrawTarget for Lcd {
    type Color = Rgb565;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        let bounds = self.bounding_box();
        for Pixel(point, color) in pixels {
            if bounds.contains(point) {
                self.write_pixels(&Rectangle::new(point, Size::new(1, 1)), [color]);
            }
        }
        Ok(())
    }

    fn fill_contiguous<I>(&mut self, area: &Rectangle, colors: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Self::Color>,
    {
        if area.is_zero_sized() {
            return Ok(());
        }
        if area.intersection(&self.bounding_box()) == *area {
            self.write_pixels(area, colors);
            Ok(())
        } else {
            self.draw_iter(area.points().zip(colors).map(|(p, c)| Pixel(p, c)))
        }
    }

    fn fill_solid(&mut self, area: &Rectangle, color: Self::Color) -> Result<(), Self::Error> {
        let visible = area.intersection(&self.bounding_box());
        if !visible.is_zero_sized() {
            let count = visible.size.width * visible.size.height;
            self.write_pixels(&visible, core::iter::repeat_n(color, count as usize));
        }
        Ok(())
    }
}
