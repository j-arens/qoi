use std::io;

use crate::error::Error;
use crate::meta::{ImageMeta, QOI_BYTES_END, QOI_BYTES_MAGIC, QOI_MAX_RUN};
use crate::op::Op;
use crate::pixel::{Pixel, PixelDiff};
use crate::state::State;

/// Encodes an image's raw pixel data and `ImageMeta` data into a QOI encoded
/// image.
/// 
/// This function supports reading and writing to in-memory structures or IO
/// streams by accepting a generic trait bound of `std::io::Read` for the
/// image's pixel data, and `std::io::Write` for the encoded image's
/// destination.
/// 
/// Note that this function performs frequent reads and writes, so it's
/// recommended to provide a buffered IO implementation such as
/// `std::io::BufReader` and `std::io::BufWriter` for streaming applications.
pub fn encode_image<R: io::Read, W: io::Write>(
  mut reader: R,
  mut writer: W,
  meta: &ImageMeta,
) -> Result<(), Error> {
  encode_header(meta, &mut writer)?;

  let mut state = State::new();
  let mut pixel_buf = vec![0; meta.channels as usize];

  for _ in 0..meta.num_pixels() {
    reader.read_exact(&mut pixel_buf)?;

    let pixel = Pixel {
      r: pixel_buf[0],
      g: pixel_buf[1],
      b: pixel_buf[2],
      a: pixel_buf.get(3).copied().unwrap_or(state.prev_pixel.a),
    };

    encode_pixel(&mut state, pixel, &mut writer)?;
    state.prev_pixel = pixel;
  }

  if state.run_count > 0 {
    Op::Run(state.run_count).into_bytes(&mut writer)?;
  }

  writer.write_all(&QOI_BYTES_END)?;
  writer.flush()?;

  Ok(())
}

// Attempts to encode the image's header and write the encoded bytes to the
// image's destination.
fn encode_header<W: io::Write>(meta: &ImageMeta, mut writer: W) -> Result<(), Error> {
  writer.write_all(QOI_BYTES_MAGIC)?;
  writer.write_all(&meta.width.to_be_bytes())?;
  writer.write_all(&meta.height.to_be_bytes())?;
  writer.write_all(&[meta.channels, meta.colorspace as u8])?;
  Ok(())
}

// Attempts to encode and write the provided pixel using the QOI OP encoding
// scheme and provided `state`.
fn encode_pixel<W: io::Write>(
  state: &mut State,
  pixel: Pixel,
  mut writer: W,
) -> Result<(), Error> {
  if pixel == state.prev_pixel {
    state.run_count += 1;

    if state.run_count == QOI_MAX_RUN {
      Op::Run(QOI_MAX_RUN).into_bytes(&mut writer)?;
      state.run_count = 0;
    }

    return Ok(());
  }

  if state.run_count > 0 {
    Op::Run(state.run_count).into_bytes(&mut writer)?;
    state.run_count = 0;
  }

  if let Some(index) = state.cache_match_or_replace(pixel) {
    Op::Index(index as u8).into_bytes(&mut writer)?;
    return Ok(());
  }

  if let Some(diff) = pixel.diff(&state.prev_pixel) {
    match diff {
      PixelDiff::Color(diff_r, diff_g, diff_b) => {
        Op::Color(diff_r, diff_g, diff_b).into_bytes(&mut writer)?;
      }
      PixelDiff::Luma(luma_g, luma_rg, luma_bg) => {
        Op::Luma(luma_g, luma_rg, luma_bg).into_bytes(&mut writer)?;
      }
    }

    return Ok(());
  }

  if pixel.a == state.prev_pixel.a {
    Op::Rgb(pixel.r, pixel.g, pixel.b).into_bytes(&mut writer)?;
    return Ok(());
  }

  Op::Rgba(pixel.r, pixel.g, pixel.b, pixel.a).into_bytes(&mut writer)?;
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::meta::{Colorspace, QOI_HEADER_LEN};

  #[test]
  fn test_encoding_rgb_op() {
    let mut dest = Vec::new();
    let mut state = State::new();
    let pixel = Pixel { r: 101, g: 102, b: 103, a: 255 };

    encode_pixel(&mut state, pixel, &mut dest).expect("Failed to encode pixel");
    assert_eq!(
      dest,
      vec![
        // Op::Rgb(101, 102, 103)
        0xfe, 101, 102, 103,
      ]
    );
  }

  #[test]
  fn test_encoding_rgba_op() {
    let mut dest = Vec::new();
    let mut state = State::new();
    let pixel = Pixel { r: 101, g: 102, b: 103, a: 104 };

    encode_pixel(&mut state, pixel, &mut dest).expect("Failed to encode pixel");
    assert_eq!(
      dest,
      vec![
        // Op::Rgba(101, 102, 103, 104)
        0xff, 101, 102, 103, 104,
      ]
    );
  }

  #[test]
  fn test_encoding_run_op() {
    let mut dest = Vec::new();
    let mut state = State::new();
    let mut pixel = Pixel { r: 101, g: 102, b: 103, a: 104 };

    state.prev_pixel = pixel;
    encode_pixel(&mut state, pixel, &mut dest).expect("Failed to encode pixel");
    assert_eq!(dest.len(), 0);

    pixel.a = 0;
    encode_pixel(&mut state, pixel, &mut dest).expect("Failed to encode pixel");
    assert_eq!(dest[0], 0xc0);
  }

  #[test]
  fn test_encoding_trailing_run_op() {
    let source = [101, 102, 103, 101, 102, 103];
    let mut dest = Vec::new();

    assert_eq!(
      encode_image(
        source.as_slice(),
        &mut dest,
        &ImageMeta { width: 2, height: 1, channels: 3, colorspace: Colorspace::Srgb },
      ),
      Ok(())
    );

    let range_start = QOI_HEADER_LEN + 4; // Header length + Op::Rgb(101, 102, 103)
    let range_end = range_start + 1; // Op::Run(1)
    assert_eq!(&dest[range_start..range_end], &[0xc0]); // Op::Run(1)
  }

  #[test]
  fn test_encoding_max_run_ops() {
    let mut dest = Vec::new();
    let mut state = State::new();
    let pixel = Pixel { r: 101, g: 102, b: 103, a: 104 };

    state.prev_pixel = pixel;
    state.run_count = 61;
    encode_pixel(&mut state, pixel, &mut dest).expect("Failed to encode pixel");
    assert_eq!(dest, vec![0xc0 | 61]); // Op::Run(61)

    encode_pixel(&mut state, pixel, &mut dest).expect("Failed to encode pixel");
    assert_eq!(dest.len(), 1);
  }

  #[test]
  fn test_encoding_index_op() {
    let mut dest = Vec::new();
    let mut state = State::new();
    let pixel = Pixel { r: 101, g: 102, b: 103, a: 104 };

    state.cache_insert(pixel);
    encode_pixel(&mut state, pixel, &mut dest).expect("Failed to encode pixel");
    assert_eq!(dest, vec![54]); // Op::Index(pixel.qoi_hash() % 64 = 54)
  }

  #[test]
  fn test_encoding_color_op() {
    let mut dest = Vec::new();
    let mut state = State::new();
    let pixel_a = Pixel { r: 100, g: 100, b: 100, a: 255 };
    let pixel_b = Pixel { r: 101, g: 101, b: 101, a: 255 };
    let pixel_c = Pixel { r: 99, g: 99, b: 99, a: 255 };

    state.prev_pixel = pixel_a;
    encode_pixel(&mut state, pixel_b, &mut dest).expect("Failed to encode pixel");
    assert_eq!(dest, vec![0x40 | 3 << 4 | 3 << 2 | 3]); // (101 - 100) + 2 = 3 = Op::Color(3, 3, 3)

    state.prev_pixel = pixel_b;
    encode_pixel(&mut state, pixel_c, &mut dest).expect("Failed to encode pixel");
    assert_eq!(dest[1], 0x40); // (99 - 101) + 2 = 0 = Op::Color(0, 0, 0)
  }

  #[test]
  fn test_encoding_luma_op() {
    let mut dest = Vec::new();
    let mut state = State::new();
    let pixel_a = Pixel { r: 100, g: 100, b: 100, a: 255 };
    let pixel_b = Pixel { r: 100, g: 108, b: 100, a: 255 };
    let pixel_c = Pixel { r: 99, g: 100, b: 99, a: 255 };

    state.prev_pixel = pixel_a;
    encode_pixel(&mut state, pixel_b, &mut dest).expect("Failed to encode pixel");
    assert_eq!(dest, vec![0x80 | 40, 0]); // Op::Luma(40, 0, 0)

    state.prev_pixel = pixel_b;
    encode_pixel(&mut state, pixel_c, &mut dest).expect("Failed to encode pixel");
    assert_eq!(dest[2..], [0x80 | 24, 15 << 4 | 15]); // Op::Luma(24, 15, 15)
  }
}
