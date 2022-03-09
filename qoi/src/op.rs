use std::io;

use crate::error::Error;

// An enumeration of each possible QOI encoding "chunk", or Op.
pub enum Op {
  // `QOI_OP_DIFF`, contains the red, green, and blue color difference from the
  // previous pixel with a bias of +2.
  // 
  // | 7 6   5  4  3  2  1  0 |
  // |------------------------|
  // | 0 1 |  dr |  dg |  db  |
  // 
  Color(u8, u8, u8),

  // `QOI_OP_INDEX`, index into the state's pixel cache.
  //
  // | 7 6   5  4  3  2  1  0 |
  // |------------------------|
  // | 0 0 |      index       |
  // 
  Index(u8),

  // `QOI_OP_LUMA`, contains the red, green, and blue luma difference from the
  // previous pixel. The green difference has a bias of +32, the red-green, and
  // blue-green difference has a bias of +8.
  // 
  // | 7 6   5  4  3  2  1  0 | 7  6  5  4   3  2  1  0 |
  // |------------------------|-------------------------|
  // | 1 0 |       dg         |   dr - dg  |   db - dg  |
  // 
  Luma(u8, u8, u8),

  // `QOI_OP_RGB`, contains the red, green, and blue values of a pixel.
  // 
  // | 7  6  5  4  3  2  1  0 | 7..0 | 7..0 | 7..0 |
  // |------------------------|------|------|------|
  // | 1  1  1  1  1  1  1  0 |   r  |   g  |   b  |
  // 
  Rgb(u8, u8, u8),

  // `QOI_OP_RGBA`, contains the red, green, blue, and alpha values of a pixel.
  // 
  // | 7  6  5  4  3  2  1  0 | 7..0 | 7..0 | 7..0 | 7..0 |
  // |------------------------|------|------|------|------|
  // | 1  1  1  1  1  1  1  1 |   r  |   g  |   b  |   a  |
  // 
  Rgba(u8, u8, u8, u8),

  // `QOI_OP_RUN`, contains the length of the run.
  // 
  // | 7 6   5  4  3  2  1  0 |
  // |------------------------|
  // | 1 1 |      run         |
  Run(u8),
}

impl Op {
  const MASK_COLOR: u8 = 0x03;
  const MASK_LUMA_1: u8 = 0x3f;
  const MASK_LUMA_2: u8 = 0x0f;
  const MASK_RUN: u8 = 0x3f;
  const MASK_TAG: u8 = 0xc0;

  const TAG_COLOR: u8 = 0x40;
  const TAG_INDEX: u8 = 0x00;
  const TAG_LUMA: u8 = 0x80;
  const TAG_RGB: u8 = 0xfe;
  const TAG_RGBA: u8 = 0xff;
  const TAG_RUN: u8 = 0xc0;

  // Encodes the `Op` and writes it as bytes into the given writer.
  pub fn into_bytes<W: io::Write>(self, mut writer: W) -> Result<(), io::Error> {
    match self {
      Op::Color(diff_r, diff_g, diff_b) => {
        writer.write_all(&[Op::TAG_COLOR | (diff_r << 4) | (diff_g << 2) | diff_b])?;
      }
      Op::Index(index) => {
        writer.write_all(&[Op::TAG_INDEX | index as u8])?;
      }
      Op::Luma(luma_g, luma_rg, luma_bg) => {
        writer.write_all(&[Op::TAG_LUMA | luma_g, (luma_rg << 4) | luma_bg])?;
      }
      Op::Rgb(r, g, b) => {
        writer.write_all(&[Op::TAG_RGB, r, g, b])?;
      }
      Op::Rgba(r, g, b, a) => {
        writer.write_all(&[Op::TAG_RGBA, r, g, b, a])?;
      }
      Op::Run(run_count) => {
        writer.write_all(&[Op::TAG_RUN | (run_count - 1)])?;
      }
    }

    Ok(())
  }

  // Attempts to decode an `Op` from the given bytes.
  pub fn try_from_bytes<I>(bytes: &mut I) -> Result<Self, Error>
  where
    I: Iterator<Item = Result<u8, io::Error>>,
  {
    let byte = bytes.next().ok_or(Error::UnexpectedEof)??;

    if byte == Op::TAG_RGB {
      return Ok(Op::Rgb(
        bytes.next().ok_or(Error::UnexpectedEof)??,
        bytes.next().ok_or(Error::UnexpectedEof)??,
        bytes.next().ok_or(Error::UnexpectedEof)??,
      ));
    }

    if byte == Op::TAG_RGBA {
      return Ok(Op::Rgba(
        bytes.next().ok_or(Error::UnexpectedEof)??,
        bytes.next().ok_or(Error::UnexpectedEof)??,
        bytes.next().ok_or(Error::UnexpectedEof)??,
        bytes.next().ok_or(Error::UnexpectedEof)??,
      ));
    }

    match byte & Op::MASK_TAG {
      Op::TAG_COLOR => {
        Ok(Op::Color(
          byte >> 4 & Op::MASK_COLOR,
          byte >> 2 & Op::MASK_COLOR,
          byte & Op::MASK_COLOR,
        ))
      }
      Op::TAG_INDEX => {
        if !(0..=64).contains(&byte) {
          return Err(Error::InvalidIndex(byte));
        }

        Ok(Op::Index(byte))
      }
      Op::TAG_LUMA => {
        let next_byte = bytes.next().ok_or(Error::UnexpectedEof)??;

        Ok(Op::Luma(
          byte & Op::MASK_LUMA_1,
          next_byte >> 4 & Op::MASK_LUMA_2,
          next_byte & Op::MASK_LUMA_2,
        ))
      }
      Op::TAG_RUN => {
        Ok(Op::Run(byte & Op::MASK_RUN))
      }
      _ => {
        Err(Error::UnknownTag(byte))
      },
    }
  }
}
