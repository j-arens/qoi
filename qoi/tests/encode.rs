use qoi::{encode_image, Colorspace, ImageMeta};

#[test]
fn test_encoding_blank_image() {
  let source = [];
  let mut dest = Vec::new();

  let result = encode_image(
    source.as_slice(),
    &mut dest,
    &ImageMeta {
      width: 0,
      height: 0,
      channels: 4,
      colorspace: Colorspace::Srgb,
    },
  );

  assert!(result.is_ok());
  assert_eq!(dest.len(), 22); // QOI_HEADER_LEN + QOI_BYTES_END.len()
}

#[test]
fn test_encoding_image_with_bad_dimensions() {
  let source = [101, 102, 103];
  let mut dest = Vec::new();

  let result = encode_image(
    source.as_slice(),
    &mut dest,
    &ImageMeta {
      width: 999,
      height: 1,
      channels: 4,
      colorspace: Colorspace::Srgb,
    },
  );

  assert!(result.is_err());
}

#[test]
fn compare_encoded_image_to_reference() {
  let source = include_bytes!("./testcard_rgba_256x256.bin");
  let mut dest = Vec::new();

  let meta = ImageMeta {
    width: 256,
    height: 256,
    channels: 4,
    colorspace: Colorspace::Srgb,
  };

  let result = encode_image(&mut source.as_slice(), &mut dest, &meta);

  assert!(result.is_ok());
  assert_eq!(
    include_bytes!("./testcard_rgba_256x256.qoi").as_slice(),
    dest.as_slice(),
  );
}
