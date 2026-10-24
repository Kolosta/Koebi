//! RGBA8 images and pixel comparison.

use std::fmt;

use koebi_core::Extent;
use koebi_gpu::TextureFormat;

use crate::png::{PngError, decode_png, encode_png};

/// An 8-bit RGBA image (straight bytes as stored in the render target).
#[derive(Clone, PartialEq, Eq)]
pub struct Image {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

/// Result of [`Image::diff`].
#[derive(Clone, Debug)]
pub struct ImageDiff {
    /// Largest absolute difference over every channel of every pixel.
    pub max_channel_delta: u8,
    /// Pixels where some channel differs by more than the tolerance passed to `diff`.
    pub differing_pixels: usize,
    /// Expected image dimmed to gray, with differing pixels in red.
    pub visualization: Image,
}

impl Image {
    /// A transparent black image.
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            rgba: vec![0; pixel_count(width, height) * 4],
        }
    }

    /// Wraps RGBA8 bytes.
    ///
    /// # Panics
    ///
    /// Panics if `rgba.len() != width * height * 4`.
    pub fn from_rgba8(width: u32, height: u32, rgba: Vec<u8>) -> Self {
        assert_eq!(
            rgba.len(),
            pixel_count(width, height) * 4,
            "RGBA buffer size mismatch"
        );
        Self {
            width,
            height,
            rgba,
        }
    }

    /// Converts tightly packed GPU readback bytes to RGBA8.
    pub(crate) fn from_gpu(
        extent: Extent,
        format: TextureFormat,
        mut bytes: Vec<u8>,
    ) -> Result<Self, String> {
        if format.bytes_per_pixel() != 4 {
            return Err(format!("readback of {format:?} is not supported"));
        }
        if format.is_bgra() {
            for pixel in bytes.as_chunks_mut::<4>().0 {
                pixel.swap(0, 2);
            }
        }
        Ok(Self::from_rgba8(extent.width, extent.height, bytes))
    }

    /// Decodes a PNG written by [`Image::to_png`].
    ///
    /// # Errors
    ///
    /// See [`decode_png`].
    pub fn from_png(bytes: &[u8]) -> Result<Self, PngError> {
        let (width, height, rgba) = decode_png(bytes)?;
        Ok(Self {
            width,
            height,
            rgba,
        })
    }

    /// Encodes as an uncompressed PNG.
    pub fn to_png(&self) -> Vec<u8> {
        encode_png(self.width, self.height, &self.rgba)
    }

    /// Width in pixels.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Height in pixels.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Raw RGBA8 bytes, row-major.
    pub fn as_rgba8(&self) -> &[u8] {
        &self.rgba
    }

    /// The pixel at `(x, y)`.
    ///
    /// # Panics
    ///
    /// Panics when out of bounds.
    pub fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        let i = self.index(x, y);
        [
            self.rgba[i],
            self.rgba[i + 1],
            self.rgba[i + 2],
            self.rgba[i + 3],
        ]
    }

    /// Sets the pixel at `(x, y)`.
    ///
    /// # Panics
    ///
    /// Panics when out of bounds.
    pub fn set_pixel(&mut self, x: u32, y: u32, rgba: [u8; 4]) {
        let i = self.index(x, y);
        self.rgba[i..i + 4].copy_from_slice(&rgba);
    }

    /// Compares with `expected`; channel differences up to `tolerance` are ignored.
    ///
    /// # Panics
    ///
    /// Panics if the dimensions differ.
    pub fn diff(&self, expected: &Self, tolerance: u8) -> ImageDiff {
        assert_eq!(
            (self.width, self.height),
            (expected.width, expected.height),
            "cannot diff images of different sizes"
        );
        let mut max_channel_delta = 0;
        let mut differing_pixels = 0;
        let mut visualization = Self::new(self.width, self.height);
        let (actual_pixels, _) = self.rgba.as_chunks::<4>();
        let (expected_pixels, _) = expected.rgba.as_chunks::<4>();
        let (out_pixels, _) = visualization.rgba.as_chunks_mut::<4>();
        for ((actual, wanted), out) in actual_pixels.iter().zip(expected_pixels).zip(out_pixels) {
            let delta = actual
                .iter()
                .zip(wanted)
                .map(|(a, b)| a.abs_diff(*b))
                .max()
                .unwrap_or(0);
            max_channel_delta = max_channel_delta.max(delta);
            if delta > tolerance {
                differing_pixels += 1;
                out.copy_from_slice(&[255, 0, 0, 255]);
            } else {
                let gray = u8::try_from(
                    (u16::from(wanted[0]) + u16::from(wanted[1]) + u16::from(wanted[2])) / 6,
                )
                .unwrap_or(u8::MAX);
                out.copy_from_slice(&[gray, gray, gray, 255]);
            }
        }
        ImageDiff {
            max_channel_delta,
            differing_pixels,
            visualization,
        }
    }

    fn index(&self, x: u32, y: u32) -> usize {
        assert!(
            x < self.width && y < self.height,
            "pixel ({x}, {y}) out of bounds"
        );
        (y as usize * self.width as usize + x as usize) * 4
    }
}

impl fmt::Debug for Image {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Image({}x{})", self.width, self.height)
    }
}

fn pixel_count(width: u32, height: u32) -> usize {
    width as usize * height as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pixels_round_trip() {
        let mut image = Image::new(3, 2);
        image.set_pixel(2, 1, [1, 2, 3, 4]);
        assert_eq!(image.pixel(2, 1), [1, 2, 3, 4]);
        assert_eq!(image.pixel(0, 0), [0, 0, 0, 0]);
    }

    #[test]
    fn bgra_readback_is_swizzled_to_rgba() {
        let image = Image::from_gpu(
            Extent::new(1, 1),
            TextureFormat::Bgra8Unorm,
            vec![3, 2, 1, 4],
        )
        .unwrap();
        assert_eq!(image.pixel(0, 0), [1, 2, 3, 4]);
    }

    #[test]
    fn diff_respects_tolerance() {
        let a = Image::from_rgba8(2, 1, vec![10, 10, 10, 255, 0, 0, 0, 255]);
        let b = Image::from_rgba8(2, 1, vec![12, 10, 10, 255, 0, 50, 0, 255]);
        let diff = a.diff(&b, 2);
        assert_eq!(diff.max_channel_delta, 50);
        assert_eq!(diff.differing_pixels, 1);
        assert_eq!(diff.visualization.pixel(1, 0), [255, 0, 0, 255]);
    }
}
