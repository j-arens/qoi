use std::array;
use std::error;
use std::fmt;
use std::io;

/// An enumeration of all error values this crate may produce.
pub enum Error {
  /// Failed to derive a supported colorspace from a QOI image.
  InvalidColorspace(u8),
  /// Failed to decode a QOI image with invalid image dimensions.
  InvalidDimensions,
  /// Failed to decode a QOI image with a missing or malformed header.
  InvalidHeader,
  /// Failed to decode an index op (Op::Index) because the index value is
  /// greater than the max of 64.
  InvalidIndex(u8),
  /// Any `std::io::Error` that occurs during decoding or encoding. Typically
  /// these will arise from problems with reading an image source or writing to
  /// an image destination.
  IoError(io::Error),
  /// Unexpectedly reached the end of an image source before decoding or
  /// encoding was completed.
  UnexpectedEof,
  /// Encountered an unknown QOI encoding chunk, or `Op`, while decoding a OQI
  /// image.
  UnknownTag(u8),
}

impl From<io::Error> for Error {
  fn from(io_err: io::Error) -> Self {
    Error::IoError(io_err)
  }
}

impl From<array::TryFromSliceError> for Error {
  fn from(_: array::TryFromSliceError) -> Self {
    Error::InvalidDimensions
  }
}

impl fmt::Display for Error {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Error::InvalidColorspace(byte) => {
        write!(f, "invalid image colorspace {}, expected 0 for sRGB or 1 for linear", byte)
      }
      Error::InvalidDimensions => {
        write!(f, "invalid image width or height")
      }
      Error::InvalidHeader => {
        write!(f, "invalid or malformed QOI image header")
      }
      Error::InvalidIndex(index) => {
        write!(f, "invalid index {}", index)
      }
      Error::IoError(io_err) => {
        write!(f, "{}", io_err)
      }
      Error::UnexpectedEof => {
        write!(f, "unexpectedly reached end of file before decoding or encoding was completed")
      }
      Error::UnknownTag(byte) => {
        write!(f, "unknown encoding `{:b}`", byte)
      }
    }
  }
}

impl fmt::Debug for Error {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self)
  }
}

impl error::Error for Error {
  fn source(&self) -> Option<&(dyn error::Error + 'static)> {
    match self {
      Error::IoError(io_err) => Some(io_err),
      _ => None,
    }
  }
}

#[cfg(test)]
impl PartialEq for Error {
  fn eq(&self, other: &Self) -> bool {
    match (self, other) {
      (Error::InvalidColorspace(a), Error::InvalidColorspace(b)) => a == b,
      (Error::InvalidDimensions, Error::InvalidDimensions) => true,
      (Error::InvalidHeader, Error::InvalidHeader) => true,
      (Error::InvalidIndex(a), Error::InvalidIndex(b)) => a == b,
      (Error::IoError(..), Error::IoError(..)) => true,
      (Error::UnexpectedEof, Error::UnexpectedEof) => true,
      (Error::UnknownTag(a), Error::UnknownTag(b)) => a == b,
      _ => false,
    }
  }
}
