use color_eyre::eyre::Result;
use image::imageops::FilterType;
use image::{DynamicImage, ImageResult};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;
use std::io::Read;
use std::time::Duration;

pub(crate) struct CoverPixels {
    pub(crate) width: u16,
    /// In pixels, not terminal rows: each row of `▀` cells shows 2 pixels, so this is 2 × rows.
    pub(crate) height: u16,
    /// Row by row, left to right: pixel (x, y) is at `y * width + x`.
    pub(crate) pixels: Vec<[u8; 3]>,
}

impl CoverPixels {
    pub fn from_image(image: &DynamicImage, width: u16, height: u16) -> Self {
        let resize = image.resize_exact(width.into(), height.into(), FilterType::Triangle);

        let rgb_image = resize.to_rgb8();

        let pixels = rgb_image.pixels().map(|pixel| pixel.0).collect();

        Self {
            width,
            height,
            pixels,
        }
    }

    /// Decodes image bytes (JPEG only, per Cargo features) and resizes them.
    /// `Cover::download` decodes once for two sizes instead, so only tests use this for now.
    #[allow(dead_code)]
    pub fn from_bytes(bytes: &[u8], width: u16, height: u16) -> ImageResult<Self> {
        let image = image::load_from_memory(bytes)?;
        Ok(Self::from_image(&image, width, height))
    }

    /// Colour of pixel (x, y). Converts to `usize` before multiplying so large images can't overflow.
    fn color_at(&self, x: u16, y: u16) -> Color {
        let index = y as usize * self.width as usize + x as usize;
        let [r, g, b] = self.pixels[index];
        Color::Rgb(r, g, b)
    }
}

/// Now-playing cover: 16 columns × 8 rows of cells.
pub(crate) const LARGE_COLS: u16 = 16;
pub(crate) const LARGE_ROWS: u16 = 8;
/// Queue glyph: 4 columns × 2 rows of cells (a square 4×4 pixel thumbnail).
pub(crate) const SMALL_COLS: u16 = 4;
pub(crate) const SMALL_ROWS: u16 = 2;

/// Largest download accepted; Spotify's smallest covers are a few KB.
const MAX_DOWNLOAD_BYTES: u64 = 1_000_000;
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(5);

/// One album's cover at both sizes the music tab draws, built from a single download.
pub(crate) struct Cover {
    pub(crate) large: CoverPixels,
    pub(crate) small: CoverPixels,
}

impl Cover {
    /// Downloads and converts a cover. Blocking: call it from a background thread.
    pub(crate) fn download(url: &str) -> Result<Self> {
        let response = ureq::get(url).timeout(DOWNLOAD_TIMEOUT).call()?;

        let mut bytes = Vec::new();
        response
            .into_reader()
            .take(MAX_DOWNLOAD_BYTES)
            .read_to_end(&mut bytes)?;

        // Decode once, resize twice.
        let image = image::load_from_memory(&bytes)?;
        Ok(Self {
            large: CoverPixels::from_image(&image, LARGE_COLS, LARGE_ROWS * 2),
            small: CoverPixels::from_image(&image, SMALL_COLS, SMALL_ROWS * 2),
        })
    }
}

/// Draws a `CoverPixels` with half-blocks: each cell is `▀` with the top pixel as the
/// foreground and the bottom pixel as the background. Same trick as the big moon.
pub(crate) struct CoverImage<'a> {
    pixels: &'a CoverPixels,
}

impl<'a> CoverImage<'a> {
    pub(crate) fn new(pixels: &'a CoverPixels) -> Self {
        Self { pixels }
    }
}

impl Widget for CoverImage<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Clip to whichever is smaller: the space we were given or the image itself.
        let cols = area.width.min(self.pixels.width);
        let rows = area.height.min(self.pixels.height / 2);

        for row in 0..rows {
            for col in 0..cols {
                let top = self.pixels.color_at(col, row * 2);
                let bottom = self.pixels.color_at(col, row * 2 + 1);

                // col/row count from the corner of our area; the buffer covers the whole screen.
                if let Some(cell) = buf.cell_mut((area.x + col, area.y + row)) {
                    cell.set_char('▀').set_fg(top).set_bg(bottom);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgb, RgbImage};

    const RED: Rgb<u8> = Rgb([255, 0, 0]);
    const BLUE: Rgb<u8> = Rgb([0, 0, 255]);

    /// 8×8 image: left half red, right half blue.
    fn red_blue_image() -> DynamicImage {
        let image = RgbImage::from_fn(8, 8, |x, _y| if x < 4 { RED } else { BLUE });
        DynamicImage::ImageRgb8(image)
    }

    #[test]
    fn keeps_left_red_and_right_blue() {
        let cover = CoverPixels::from_image(&red_blue_image(), 4, 4);

        assert_eq!(cover.width, 4);
        assert_eq!(cover.height, 4);
        assert_eq!(cover.pixels.len(), 16);

        // Smoothing blends the colours a little, so compare channels rather than exact values.
        let top_left = cover.pixels[0];
        let top_right = cover.pixels[3];
        assert!(
            top_left[0] > top_left[2],
            "top-left should be mostly red: {top_left:?}"
        );
        assert!(
            top_right[2] > top_right[0],
            "top-right should be mostly blue: {top_right:?}"
        );
    }

    #[test]
    fn rejects_bytes_that_are_not_an_image() {
        assert!(CoverPixels::from_bytes(b"definitely not a jpeg", 4, 4).is_err());
    }

    #[test]
    fn widget_draws_half_blocks_in_the_right_colours() {
        // 4×4 pixels = 4 columns × 2 rows of cells.
        let cover = CoverPixels::from_image(&red_blue_image(), 4, 4);
        let area = Rect::new(0, 0, 4, 2);
        let mut buf = Buffer::empty(area);

        CoverImage::new(&cover).render(area, &mut buf);

        let left = &buf[(0, 0)];
        let right = &buf[(3, 1)];
        assert_eq!(left.symbol(), "▀");

        let Color::Rgb(r, _, b) = left.fg else {
            panic!("expected an RGB colour, got {:?}", left.fg);
        };
        assert!(r > b, "left cell should be red");

        let Color::Rgb(r, _, b) = right.bg else {
            panic!("expected an RGB colour, got {:?}", right.bg);
        };
        assert!(b > r, "right cell should be blue");
    }

    #[test]
    fn widget_clips_to_a_smaller_area_without_panicking() {
        let cover = CoverPixels::from_image(&red_blue_image(), 4, 4);
        let area = Rect::new(0, 0, 2, 1);
        let mut buf = Buffer::empty(area);

        CoverImage::new(&cover).render(area, &mut buf);

        assert_eq!(buf[(1, 0)].symbol(), "▀");
    }

    #[test]
    fn widget_draws_at_the_area_offset_not_the_screen_corner() {
        let cover = CoverPixels::from_image(&red_blue_image(), 4, 4);
        let screen = Rect::new(0, 0, 10, 5);
        let mut buf = Buffer::empty(screen);

        CoverImage::new(&cover).render(Rect::new(5, 2, 4, 2), &mut buf);

        assert_eq!(buf[(0, 0)].symbol(), " ", "screen corner should be untouched");
        assert_eq!(buf[(5, 2)].symbol(), "▀");
    }
}
