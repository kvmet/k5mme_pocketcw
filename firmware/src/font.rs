use embedded_graphics::{
    pixelcolor::{Rgb565, RgbColor},
    prelude::*,
    primitives::Rectangle,
};

/// Monospace glyph table made by `tools/fontgen`.
pub struct Font {
    pub width: u16,
    pub height: u16,
    /// Sorted by byte value.
    pub chars: &'static [u8],
    /// Coverage 0..=15, 4 bits per pixel, high nibble first, `width * height` pixels per glyph.
    pub pixels: &'static [u8],
}

impl Font {
    fn glyph(&self, c: u8) -> Option<&'static [u8]> {
        let index = self.chars.binary_search(&c).ok()?;
        let size = (self.width as usize * self.height as usize).div_ceil(2);
        self.pixels.get(index * size..(index + 1) * size)
    }

    pub fn size(&self) -> Size {
        Size::new(self.width as u32, self.height as u32)
    }

    /// Draws `text` with its top left corner at `origin`, blending from `bg` to `fg` by coverage.
    /// A character missing from the font draws as a solid `fg` cell, so the gap shows on screen.
    pub fn draw<D: DrawTarget<Color = Rgb565>>(
        &self,
        target: &mut D,
        origin: Point,
        text: &str,
        fg: Rgb565,
        bg: Rgb565,
    ) -> Result<(), D::Error> {
        let palette = blend_palette(fg, bg);
        let pixel_count = self.width as usize * self.height as usize;
        for (i, c) in text.bytes().enumerate() {
            let cell = Rectangle::new(origin + Point::new(i as i32 * self.width as i32, 0), self.size());
            match self.glyph(c) {
                Some(glyph) => target.fill_contiguous(
                    &cell,
                    (0..pixel_count).map(|p| {
                        let shift = if p % 2 == 0 { 4 } else { 0 };
                        palette[((glyph[p / 2] >> shift) & 0x0F) as usize]
                    }),
                )?,
                None => target.fill_solid(&cell, fg)?,
            }
        }
        Ok(())
    }
}

/// The 16 coverage levels from `bg` (0) to `fg` (15).
fn blend_palette(fg: Rgb565, bg: Rgb565) -> [Rgb565; 16] {
    core::array::from_fn(|level| {
        let level = level as u32;
        let mix = |f: u8, b: u8| ((b as u32 * (15 - level) + f as u32 * level + 7) / 15) as u8;
        Rgb565::new(mix(fg.r(), bg.r()), mix(fg.g(), bg.g()), mix(fg.b(), bg.b()))
    })
}
