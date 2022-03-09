use qoi::{decode_image, Colorspace, ImageMeta};

#[test]
fn test_decoding_blank_image() {
  let source = include_bytes!("./blank_rgb.qoi");
  let mut dest = Vec::new();
  let meta = decode_image(source.as_slice(), &mut dest).expect("Failed to decode image");

  assert_eq!(dest.len(), 0);

  assert_eq!(
    meta,
    ImageMeta {
      width: 0,
      height: 0,
      channels: 3,
      colorspace: Colorspace::Linear,
    }
  );
}

#[test]
fn test_decoding_incomplete_image() {
  let source = include_bytes!("./incomplete_rgb.qoi");
  let mut dest = Vec::new();
  let result = decode_image(source.as_slice(), &mut dest);

  assert!(result.is_err());
}

#[test]
fn compare_decoded_image_to_reference() {
  let source = include_bytes!("./testcard_rgba_256x256.qoi");
  let mut dest = Vec::new();
  let meta = decode_image(&mut source.as_slice(), &mut dest).expect("Failed to decode image");

  assert_eq!(
    meta,
    ImageMeta {
      width: 256,
      height: 256,
      channels: 4,
      colorspace: Colorspace::Srgb,
    }
  );

  assert_eq!(
    include_bytes!("./testcard_rgba_256x256.bin").as_slice(),
    dest.as_slice(),
  );
}
