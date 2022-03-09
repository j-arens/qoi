use crate::error::Error;

pub const QOI_BYTES_END: [u8; 8] = [0, 0, 0, 0, 0, 0, 0, 1];
pub const QOI_BYTES_MAGIC: &[u8] = b"qoif";
pub const QOI_MAX_RUN: u8 = 62;
pub const QOI_HEADER_LEN: usize = 14;

/// Metadata describing an Image.
#[derive(Debug, PartialEq)]
pub struct ImageMeta {
  /// The number of color channels the image's pixels contain. For example,
  /// RGBA pixels have four channels, and RGB have three. Color channels are
  /// assumed to not be pre-multiplied with the alpha channel
  /// ("un-premultiplied alpha").
  pub channels: u8,
  /// The image's colorspace, see [Colorspace].
  pub colorspace: Colorspace,
  /// The image's height.
  pub height: u32,
  /// The image's width.
  pub width: u32,
}

impl ImageMeta {
  /// Returns the total number of pixels that make up the image.
  pub fn num_pixels(&self) -> usize {
    (self.width * self.height) as usize
  }
}

/// How an image's colors, or pixels, are organized. Supports only sRGB (RGBA)
/// and Linear (RGB) for now.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Colorspace {
  Linear = 1,
  Srgb = 0,
}

/// A `TryFrom` implemenation for converting any `u8` into a `Colorspace`.
/// `1` maps to `Colorspace::Linear`, and `0` maps to `Colorspace::Srgb`. All
/// other `u8` values are invalid.
impl TryFrom<u8> for Colorspace {
  type Error = Error;

  fn try_from(byte: u8) -> Result<Self, Self::Error> {
    let linear_u8 = Colorspace::Linear as u8;
    let srgb_u8 = Colorspace::Srgb as u8;

    match byte {
      _ if byte == linear_u8 => Ok(Colorspace::Linear),
      _ if byte == srgb_u8 => Ok(Colorspace::Srgb),
      _ => Err(Error::InvalidColorspace(byte)),
    }
  }
}
