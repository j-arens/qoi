// An enumeration of the possible pixel "diffs".
pub enum PixelDiff {
  // A color, or `QOI_OP_DIFF` diff with bias applied.
  Color(u8, u8, u8),
  // A luma, or `QOI_OP_LUMA` diff with bias applied.
  Luma(u8, u8, u8),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Pixel {
  // Red channel value.
  pub r: u8,
  // Green channel value.
  pub g: u8,
  // Blue channel value.
  pub b: u8,
  // Alpha channel value.
  pub a: u8,
}

impl Default for Pixel {
  fn default() -> Self {
    Self { r: 0, g: 0, b: 0, a: 255 }
  }
}

impl Pixel {
  // Attempts to produce a `PixelDiff` against the provided `other` pixel.
  // Returns `Some(PixelDiff)` when there is a diff within range, otherwise
  // `None` is returned.
  pub fn diff(&self, other: &Pixel) -> Option<PixelDiff> {
    if self.a != other.a {
      return None;
    }

    let diff_r = self.r.wrapping_sub(other.r);
    let diff_g = self.g.wrapping_sub(other.g);
    let diff_b = self.b.wrapping_sub(other.b);

    let color_r = diff_r.wrapping_add(2);
    let color_g = diff_g.wrapping_add(2);
    let color_b = diff_b.wrapping_add(2);
    let range = 0..=3;

    if range.contains(&color_r) && range.contains(&color_g) && range.contains(&color_b) {
      return Some(PixelDiff::Color(color_r, color_g, color_b));
    }

    let luma_g = diff_g.wrapping_add(32);

    if !(0..=63).contains(&luma_g) {
      return None;
    }

    let luma_rg = diff_r.wrapping_add(8).wrapping_sub(diff_g);
    let luma_bg = diff_b.wrapping_add(8).wrapping_sub(diff_g);
    let range = 0..=15;

    if range.contains(&luma_rg) && range.contains(&luma_bg) {
      return Some(PixelDiff::Luma(luma_g, luma_rg, luma_bg));
    }

    None
  }

  // Recreates a `Pixel` from the provided `diff` and `diff_pixel`.
  pub fn from_diff(diff: PixelDiff, diff_pixel: &Pixel) -> Self {
    match diff {
      PixelDiff::Color(diff_r, diff_g, diff_b) => Self {
        r: diff_pixel.r.wrapping_add(diff_r.wrapping_sub(2)),
        g: diff_pixel.g.wrapping_add(diff_g.wrapping_sub(2)),
        b: diff_pixel.b.wrapping_add(diff_b.wrapping_sub(2)),
        a: diff_pixel.a,
      },
      PixelDiff::Luma(luma_g, luma_rg, luma_bg) => {
        let diff_g = luma_g.wrapping_sub(32);
        let diff_r = luma_rg.wrapping_sub(8).wrapping_add(diff_g);
        let diff_b = luma_bg.wrapping_sub(8).wrapping_add(diff_g);

        Self {
          r: diff_pixel.r.wrapping_add(diff_r),
          g: diff_pixel.g.wrapping_add(diff_g),
          b: diff_pixel.b.wrapping_add(diff_b),
          a: diff_pixel.a,
        }
      }
    }
  }

  // QOI color hash function, not implemented via the `Hash` trait to keep
  // things simple.
  pub fn qoi_hash(&self) -> usize {
    let r = self.r as usize;
    let g = self.g as usize;
    let b = self.b as usize;
    let a = self.a as usize;

    r * 3 + g * 5 + b * 7 + a * 11
  }
}
