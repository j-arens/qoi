use crate::pixel::Pixel;

// A collection of stateful properties and methods maintained during decoding
// or encoding of an image.
pub struct State {
  // A cache of previously seen pixels, indexed by their hash value % 64.
  pub cache: [Pixel; 64],
  // The previously decoded/encoded pixel.
  pub prev_pixel: Pixel,
  // Length of the current run (Op::Run) (if any).
  pub run_count: u8,
}

impl State {
  pub fn new() -> Self {
    Self {
      cache: [Pixel { r: 0, g: 0, b: 0, a: 0 }; 64],
      prev_pixel: Pixel::default(),
      run_count: 0,
    }
  }

  // Inserts the given pixel into the cache. Overwrites any pixel that was
  // previously cached at the computed index.
  pub fn cache_insert(&mut self, pixel: Pixel) {
    self.cache[pixel.qoi_hash() % 64] = pixel;
  }

  // Checks if the given pixel matches the cached pixel at the computed index
  // and returns the index. If there is no match, the given pixel is inserted,
  // overwriting the pixel that was previously cached, and the index is not
  // returned.
  pub fn cache_match_or_replace(&mut self, pixel: Pixel) -> Option<usize> {
    let index = pixel.qoi_hash() % 64;
    let other_pixel = self.cache[index];

    if other_pixel == pixel {
      return Some(index);
    }

    self.cache[index] = pixel;

    None
  }
}
