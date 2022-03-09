//! A [WebAssembly](https://developer.mozilla.org/en-US/docs/WebAssembly)
//! wrapper of the QOI crate. This makes it possible to use the crate on any
//! host system that supports WebAssembly such as a web browser or on systems
//! that have implemented [WASI](https://wasi.dev/).
//! 
//! Passing and translating rich data types through the WASM FFI boundary isn't
//! supported as of writing this, so some manual setup and teardown steps need
//! to be done to copy the decoded and encoded image data to and from the WASM
//! instance. It's important that these steps are executed in the right order
//! since they involve manually allocating and deallocating raw memory.
//! 
//! # JavaScript WebAssembly decode example
//! 
//! ```js
//! let wasm = await WebAssembly.instantiateStreaming(fetch('./path-to-wasm.wasm'), {
//!   // "Extern" functions that must be imported into the WASM instance.
//!   env: {
//!     // Callback invoked by the WASM instance when decoding an image has
//!     // successfully completed.
//!     on_decode_complete: (pointer, width, height, channels, colorspace) => {
//!       // Address that points to the beginning of the decoded image data in
//!       // the WASM instance's memory.
//!       console.log(pointer);
//! 
//!       // The image's dimensions.
//!       console.log(width, height);
//! 
//!       // Number of pixel color channels and the image's colorspace.
//!       console.log(channels, colorspace);
//! 
//!       // Copy the decoded pixel data out of the WASM instance's memory.
//!       let size = width * height * channels;
//!       let decodedImage = wasm.instance.exports.memory.slice(pointer, size);
//! 
//!       // Deallocate the decoded pixel data WASM memory.
//!       wasm.instance.exports.qoi_dealloc(pointer, size);
//!     },
//!   },
//!   
//!   // Callback invoked by the WASM instance when an error occurs while
//!   // decoding an image.
//!   on_decode_error: (code) => {
//!     // Error code that maps to the type of error that occured.
//!     console.error(code);
//!   },
//! 
//!   // Ignore these for this example.
//!   on_encode_complete: () => {},
//!   on_encode_error: () => {},
//! });
//! 
//! // An imaginary QOI image buffer.
//! let encodedImage = new ArrayBuffer(..);
//! 
//! // Copy the QOI image buffer into the WASM instance's memory.
//! let size = encodedImage.byteLength;
//! let pointer = wasm.instance.exports.qoi_malloc(size);
//! let slice = new Uint8Array(wasm.instance.exports.memory, pointer, size);
//! slice.set(encodedImage);
//! 
//! // Call the WASM instance's `qoi_image_decode` function with the image's
//! // pointer and size in memory.
//! wasm.instance.exports.qoi_image_decode(pointer, size);
//! 
//! // Deallocate the memory used to copy the encoded image into the WASM
//! // instance.
//! wasm.instance.exports.qoi_dealloc(pointer, size);
//! ```
//! 
//! # JavaScript WebAssembly encode example
//! 
//! ```js
//! let wasm = await WebAssembly.instantiateStreaming(fetch('./path-to-wasm.wasm'), {
//!   // Callback invoked by the WASM instance when encoding an image has
//!   // successfully completed.
//!   on_encode_complete: (pointer, size) => {
//!     // Address that points to the beginning of the encoded image data in
//!     // WASM instance's memory.
//!     console.log(pointer);
//! 
//!     // Size of the encoded image in memory.
//!     console.log(size);
//! 
//!     // Copy the encoded image data out of the WASM instance's memory.
//!     let encodedImage = wasm.instance.exports.memory.slice(pointer, size);
//! 
//!     // Deallocate the encoded image data WASM memory.
//!     wasm.instance.exports.qoi_dealloc(pointer, size);
//!   },
//! 
//!   on_encode_error: (code) => {
//!     // Error code that maps to the type of error that occured.
//!     console.error(code);
//!   },
//! 
//!   // Ignore these for this example.
//!   on_decode_complete: () => {},
//!   on_decode_error: () => {}.
//! });
//! 
//! // An imaginary buffer of image pixel data to encode.
//! let decodedImage = new ArrayBuffer(..);
//! 
//! // Copy the pixel data into the WASM instance's memory.
//! let size = decodedImage.byteLength;
//! let pointer = wasm.instance.exports.qoi_malloc(size);
//! let slice = new Uint8Array(wasm.instance.exports.memory, pointer, size);
//! slice.set(decodedImage);
//! 
//! // Call the WASM instance's `qoi_image_encode` function with the pixel
//! // data's pointer and size in memory, as well was the image's width,
//! // height, channels, and colorspace.
//! let imageWidth = 100;
//! let imageHeight = 100;
//! let colorspace = 1; // Or 0 for Srgb.
//! wasm.instance.exports.qoi_image_encode(
//!   imageWidth,
//!   imageHeight,
//!   colorspace,
//!   pointer,
//!   size,
//! );
//! 
//! // Deallocate the memory used to copy the decoded pixel data into the WASM
//! // instance.
//! wasm.instance.exports.qoi_dealloc(pointer, size);
//! ```
//! 

use std::mem;

use qoi::{decode_image, encode_image, Colorspace, Error, ImageMeta};

// Maps a QOI crate error into an integer that can be trivially passed through
// the WASM FFI boundary.
struct ErrorCode {
  code: u8,
}

impl From<Error> for ErrorCode {
  fn from(error: Error) -> Self {
    match error {
      Error::InvalidColorspace(_) => ErrorCode { code: 1 },
      Error::InvalidDimensions => ErrorCode { code: 2 },
      Error::InvalidHeader => ErrorCode { code: 3 },
      Error::InvalidIndex(_) => ErrorCode { code: 4 },
      Error::IoError(_) => ErrorCode { code: 5 },
      Error::UnexpectedEof => ErrorCode { code: 6 },
      Error::UnknownTag(_) => ErrorCode { code: 7 },
    }
  }
}

// External functions that are expected to be imported into the WASM instance
// from the host.
extern "C" {
  fn on_decode_complete(buf_ptr: *mut u8, width: u32, height: u32, channels: u8, colorspace: u8);
  fn on_decode_error(err_code: u8);
  fn on_encode_complete(buf_ptr: *mut u8, size: usize);
  fn on_encode_error(err_code: u8);
}

/// Allocates a chunk of linear memory of the given `size`, intended to contain
/// byte (u8) values.
#[no_mangle]
pub extern "C" fn qoi_malloc(size: usize) -> *mut u8 {
  let mut buf = Vec::with_capacity(size);
  let ptr = buf.as_mut_ptr();

  mem::forget(buf);

  ptr
}

/// Deallocates the memory starting at `ptr` up to `size`.
/// 
/// # Safety
/// 
/// This is highly unsafe, due to unchecked variants. This function should only
/// be called once for each allocation created with `qoi_malloc` using the same
/// `size` and the returned `ptr`, otherwise the WASM instance's memory will
/// be corrupted.
#[no_mangle]
pub unsafe extern "C" fn qoi_dealloc(ptr: *mut u8, size: usize) {
  Vec::from_raw_parts(ptr, size, size);
}

/// Takes an image's metadata, the size of it's pixel data in memory, and a
/// pointer to the data and encodes it as a QOI image.
/// 
/// Calls `on_encode_complete` with a pointer to the encoded image's data and
/// size.
/// 
/// Calls `on_encode_error` with an error code if an error occurs.
/// 
/// # Safety
/// 
/// Requires creating a `Vec` of pixel data from raw memory created by calling
/// `qoi_malloc`. Providing an invalid `buf_ptr` or `buf_size` will result in
/// corrupted memory and likely crash the WASM instance.
#[no_mangle]
pub unsafe extern "C" fn qoi_image_encode(
  width: u32,
  height: u32,
  colorspace: u8,
  buf_ptr: *mut u8,
  buf_size: usize,
) {
  let colorspace = match Colorspace::try_from(colorspace) {
    Ok(colorspace) => colorspace,
    Err(e) => {
      on_encode_error(ErrorCode::from(e).code);
      return;
    }
  };

  let channels = match colorspace {
    Colorspace::Linear => 3,
    Colorspace::Srgb => 4,
  };

  let image_meta = ImageMeta { channels, colorspace, height, width };
  let source = Vec::from_raw_parts(buf_ptr, buf_size, buf_size);
  let mut dest = Vec::new();

  match encode_image(&mut source.as_slice(), &mut dest, &image_meta) {
    Ok(_) => {
      let size = dest.len();
      let ptr = dest.as_mut_ptr();

      mem::forget(source);
      mem::forget(dest);

      on_encode_complete(ptr, size);
    }
    Err(e) => {
      on_encode_error(ErrorCode::from(e).code);
    }
  }
}

/// Takes a `buf_ptr` and `buf_size` to a chunk of memory that represents a QOI
/// encoded image and decodes it into raw pixel data.
/// 
/// Calls `on_decode_complete` with a `ptr` to the decoded pixel data and the
/// image's metadata. The memory size of the image can be derived by
/// multiplying the width * height * channels.
/// 
/// Calls `on_decode_error` with an error code if an error occurs.
/// 
/// # Safety
/// 
/// Requires creating a `Vec` of image data from raw memory created by calling
/// `qoi_malloc`. Providing an invalid `buf_ptr` or `buf_size` will result in
/// corrupted memory and likely crash the WASM instance.
#[no_mangle]
pub unsafe extern "C" fn qoi_image_decode(buf_ptr: *mut u8, buf_size: usize) {
  let source = Vec::from_raw_parts(buf_ptr, buf_size, buf_size);
  let mut dest = Vec::new();

  match decode_image(&mut source.as_slice(), &mut dest) {
    Ok(image_meta) => {
      let ImageMeta { channels, colorspace, height, width } = image_meta;

      let ptr = dest.as_mut_ptr();

      mem::forget(source);
      mem::forget(dest);

      on_decode_complete(ptr, width, height, channels, colorspace as u8);
    }
    Err(e) => {
      on_decode_error(ErrorCode::from(e).code);
    }
  }
}
