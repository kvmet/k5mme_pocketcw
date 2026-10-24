# Fonts

The screen text uses B612 Mono Regular, under the SIL Open Font License 1.1. The font file is `tools/fontgen/B612Mono-Regular.ttf`, and the license is `firmware/src/B612-OFL.txt`.

The firmware does not read the font file. `tools/fontgen` turns it into glyph tables in `firmware/src/fonts.rs`. Do not edit `fonts.rs` by hand: the next run of the tool replaces it.

## The two fonts

| Font | Cell | Size | Use |
| - | - | - | - |
| `SMALL` | 10 x 16 pixels | 13 px | Main text. 32 columns by 15 rows fill the 320 x 240 screen. |
| `TINY` | 7 x 12 pixels | 10 px | Labels, units and chart axes. |

Each font holds the printable ASCII characters, from space to `~`.

## Glyph format

`firmware/src/font.rs` defines `Font`:

- `width`, `height`: the cell size. Every glyph has the same cell.
- `chars`: the characters, sorted by byte value. `Font::draw` finds a glyph by binary search.
- `pixels`: 4 bits of coverage per pixel, 0 to 15, high nibble first, `width * height` pixels per glyph.

`Font::draw` blends from the background color to the text color by coverage. A character that is not in the font draws as a solid cell, so the gap shows on the screen.

A prosign draws as its two letters under a bar. `ui/mod.rs` does this in `draw_char`, so the fonts need no prosign glyphs.

## Make the fonts again

```bash
cd tools/fontgen
cargo build -r
target/release/fontgen B612Mono-Regular.ttf ../../firmware/src/fonts.rs
```

`fontgen` is its own crate, with `fontdue` as its only dependency. It builds for your computer.

The sizes and cells are in `SPECS` in `tools/fontgen/src/main.rs`. When a glyph has visible pixels outside its cell, the tool prints how many and writes nothing. When you change a cell size, change `CHAR_W`, `CHAR_H`, `TINY_W` and `TINY_H` in `firmware/src/gfx.rs` too. Asserts there stop the build when the two do not match.

The hardware probe uses the `FONT_6X10` font of `embedded-graphics`, not these fonts.
