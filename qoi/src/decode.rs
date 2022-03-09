use std::io;

use crate::error::Error;
use crate::meta::{Colorspace, ImageMeta, QOI_BYTES_MAGIC, QOI_HEADER_LEN};
use crate::op::Op;
use crate::pixel::{Pixel, PixelDiff};
use crate::state::State;

/// Decodes a QOI encoded image. The decoded pixel data is written to the
/// provided `writer`, and upon success, the image's `ImageMeta` data is
/// returned.
/// 
/// This function supports reading and writing to in-memory structures or IO
/// streams by accepting a generic trait bound of `std::io::Read` for the
/// encoded image source, and `std::io::Write` for the decoded pixel data
/// destination.
/// 
/// Note that this function performs frequent reads and writes, so it's
/// recommended to provide a buffered IO implementation such as
/// `std::io::BufReader` and `std::io::BufWriter` for streaming applications.
pub fn decode_image<R: io::Read, W: io::Write>(
  mut reader: R,
  mut writer: W,
) -> Result<ImageMeta, Error> {
  let meta = decode_header(&mut reader)?;
  let mut state = State::new();
  let mut bytes = reader.bytes();

  for _ in 0..meta.num_pixels() {
    let pixel = decode_pixel(&mut state, &mut bytes)?;

    if pixel != state.prev_pixel {
      state.cache_insert(pixel);
      state.prev_pixel = pixel;
    }

    match meta.colorspace {
      Colorspace::Linear => {
        writer.write_all(&[pixel.r, pixel.g, pixel.b])?;
      }
      Colorspace::Srgb => {
        writer.write_all(&[pixel.r, pixel.g, pixel.b, pixel.a])?;
      }
    }
  }

  writer.flush()?;

  Ok(meta)
}

// Attempts to decode the image's header, returning the image's `ImageMeta`
// data upon success.
fn decode_header<R: io::Read>(mut reader: R) -> Result<ImageMeta, Error> {
  let mut header_buf = [0; QOI_HEADER_LEN];
  reader.read_exact(&mut header_buf)?;

  if &header_buf[..4] != QOI_BYTES_MAGIC {
    return Err(Error::InvalidHeader);
  }

  Ok(ImageMeta {
    width: u32::from_be_bytes(header_buf[4..8].try_into()?),
    height: u32::from_be_bytes(header_buf[8..12].try_into()?),
    channels: header_buf[12],
    colorspace: Colorspace::try_from(header_buf[13])?,
  })
}

// Attempts to decode a single "next" pixel from the provided encoding `state`
// and encoded `bytes`.
fn decode_pixel<I: Iterator<Item = Result<u8, io::Error>>>(
  state: &mut State,
  bytes: &mut I,
) -> Result<Pixel, Error> {
  if state.run_count > 0 {
    state.run_count -= 1;
    return Ok(state.prev_pixel);
  }

  let pixel = match Op::try_from_bytes(bytes)? {
    Op::Color(diff_r, diff_g, diff_b) => {
      Pixel::from_diff(PixelDiff::Color(diff_r, diff_g, diff_b), &state.prev_pixel)
    }
    Op::Index(index) => {
      state.cache[index as usize]
    }
    Op::Luma(luma_g, luma_rg, luma_bg) => {
      Pixel::from_diff(PixelDiff::Luma(luma_g, luma_rg, luma_bg), &state.prev_pixel)
    }
    Op::Rgb(r, g, b) => {
      Pixel { r, g, b, a: state.prev_pixel.a }
    }
    Op::Rgba(r, g, b, a) => {
      Pixel { r, g, b, a }
    }
    Op::Run(count) => {
      state.run_count = count;
      state.prev_pixel
    }
  };

  Ok(pixel)
}

#[cfg(test)]
mod tests {
  use std::io::Read;

  use super::*;

  #[test]
  fn test_decoding_image_header() {
    let mut header = Vec::new();

    header.extend_from_slice(QOI_BYTES_MAGIC);
    header.extend_from_slice(&0u32.to_be_bytes());
    header.extend_from_slice(&0u32.to_be_bytes());
    header.extend_from_slice(&[4, 0]);

    assert_eq!(
      decode_header(header.as_slice()),
      Ok(ImageMeta { width: 0, height: 0, channels: 4, colorspace: Colorspace::Srgb })
    );
  }

  #[test]
  fn test_decoding_invalid_image_header() {
    let mut header = Vec::new();

    header.extend_from_slice(&[b'q', b'q', b'q', b'q']);
    header.extend_from_slice(&0usize.to_be_bytes());
    header.extend_from_slice(&0usize.to_be_bytes());
    header.extend_from_slice(&[5, 2]);

    assert!(decode_header(header.as_slice()).is_err());
  }

  #[test]
  fn test_decoding_rgb_op() {
    let mut state = State::new();
    let mut source = Vec::new();

    Op::Rgb(101, 102, 103)
      .into_bytes(&mut source)
      .expect("Failed to write op");

    assert_eq!(
      decode_pixel(&mut state, &mut source.as_slice().bytes()),
      Ok(Pixel { r: 101, g: 102, b: 103, a: 255 })
    );
  }

  #[test]
  fn test_decoding_rgba_op() {
    let mut state = State::new();
    let mut source = Vec::new();

    Op::Rgba(101, 102, 103, 104)
      .into_bytes(&mut source)
      .expect("Failed to write op");

    assert_eq!(
      decode_pixel(&mut state, &mut source.as_slice().bytes()),
      Ok(Pixel { r: 101, g: 102, b: 103, a: 104 })
    );
  }

  #[test]
  fn test_decoding_run_op() {
    let mut state = State::new();
    let pixel = Pixel { r: 101, g: 102, b: 103, a: 104 };
    let mut source = Vec::new();

    Op::Run(1)
      .into_bytes(&mut source)
      .expect("Failed to write op");

    state.prev_pixel = pixel;

    assert_eq!(
      decode_pixel(&mut state, &mut source.as_slice().bytes()),
      Ok(pixel)
    );

    assert_eq!(state.run_count, 0);
  }

  #[test]
  fn test_decoding_index_op() {
    let mut state = State::new();
    let pixel = Pixel { r: 101, g: 102, b: 103, a: 104 };
    let mut source = Vec::new();

    Op::Index((pixel.qoi_hash() % 64) as u8)
      .into_bytes(&mut source)
      .expect("Failed to write op");

    state.cache_insert(pixel);

    assert_eq!(
      decode_pixel(&mut state, &mut source.as_slice().bytes()),
      Ok(pixel)
    );
  }

  #[test]
  fn test_decoding_color_op() {
    let mut state = State::new();
    let pixel_a = Pixel { r: 100, g: 100, b: 100, a: 255 };
    let pixel_b = Pixel { r: 101, g: 101, b: 101, a: 255 };
    let mut source = Vec::new();

    state.prev_pixel = pixel_a;

    match pixel_b.diff(&pixel_a) {
      Some(PixelDiff::Color(diff_r, diff_g, diff_b)) => {
        Op::Color(diff_r, diff_g, diff_b)
          .into_bytes(&mut source)
          .expect("Failed to write op");
      }
      _ => {
        panic!("Expected color diff");
      }
    };

    assert_eq!(
      decode_pixel(&mut state, &mut source.as_slice().bytes()),
      Ok(pixel_b)
    );
  }

  #[test]
  fn test_decoding_luma_op() {
    let mut state = State::new();
    let pixel_a = Pixel { r: 100, g: 100, b: 100, a: 255 };
    let pixel_b = Pixel { r: 100, g: 108, b: 100, a: 255 };
    let mut source = Vec::new();

    state.prev_pixel = pixel_a;

    match pixel_b.diff(&pixel_a) {
      Some(PixelDiff::Luma(diff_g, diff_rg, diff_bg)) => {
        Op::Luma(diff_g, diff_rg, diff_bg)
          .into_bytes(&mut source)
          .expect("Failed to write op");
      }
      _ => {
        panic!("Expected luma diff");
      }
    };

    assert_eq!(
      decode_pixel(&mut state, &mut source.as_slice().bytes()),
      Ok(pixel_b)
    );
  }
}
