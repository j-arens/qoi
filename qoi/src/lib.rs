//! This crate implements an encoder and decoder for the
//! [QOI image format](https://qoiformat.org).
//! 
//! The two primary exports are the `decode_image` and `encode_image`
//! functions. Both support reading and writing to IO streams or in-memory
//! structures by accepting a generic trait bound of `std::io::Read` for the
//! image source, and `std::io::Write` for the image destination.
//! 
//! Both functions perform frequent reads and writes, so it's recommended to
//! use buffered IO implementations such as `std::io::BufReader` and
//! `std::io::BufWriter` for streaming applications.
//! 
//! To keep this crate simple, it does not support decoding other image
//! formats. To encode an image, it will first need to be decoded using another
//! method. From there, the decoded pixel data can then be encoded.
//! 
//! # In-memory encode example
//!
//! ```rust
//! use qoi::{encode_image, Colorspace, ImageMeta};
//! 
//! // A 1x1 representation of an image's pixel data made up of an opaque black pixel.
//! let image_source = vec![0, 0, 0, 255];
//! 
//! // Buffer to write the encoded image to.
//! let mut image_destination = Vec::new();
//! 
//! // Metadata describing the image to be encoded.
//! let image_meta = ImageMeta {
//!   width: 1,
//!   height: 1,
//!   channels: 4,
//!   colorspace: Colorspace::Srgb,
//! };
//! 
//! match encode_image(&mut image_source.as_slice(), &mut image_destination, &image_meta) {
//!   Ok(()) => {
//!     // `image_destination` will contain the encoded QOI image bytes.
//!     dbg!(image_destination);
//!   }
//!   Err(e) => {
//!     // See `error.rs` for all possible errors.
//!     panic!("{}", e);
//!   }
//! }
//! ```
//! 
//! # Streaming decode example
//! 
//! ```rust
//! use std::fs::File;
//! use std::io::{BufReader, BufWriter, sink};
//! use qoi::{decode_image, Error, ImageMeta};
//! 
//! let image_source = File::open("./tests/testcard_rgba_256x256.qoi")
//!   .expect("Failed to open image file");
//! 
//! let mut reader = BufReader::new(image_source);
//! 
//! // For example purposes, write decoded bytes into the void.
//! let image_destination = sink();
//! let mut writer = BufWriter::new(image_destination);
//! 
//! match decode_image(&mut reader, &mut writer) {
//!   Ok(image_meta) => {
//!     // Metadata describing the decoded image.
//!     dbg!(image_meta);
//!   }
//!   Err(e) => {
//!     // See `error.rs` for all possible errors.
//!     panic!("{}", e);
//!   }
//! }
//! ```
//! 

pub use crate::decode::decode_image;
pub use crate::encode::encode_image;
pub use crate::error::Error;
pub use crate::meta::{Colorspace, ImageMeta};

mod decode;
mod encode;
mod error;
mod meta;
mod op;
mod pixel;
mod state;
