use palette::RgbHue;
use typst_macros::cast;

use crate::diag::{bail, StrResult};
use crate::foundations::{array, Array};
use crate::layout::Ratio;

use super::{Cmyk, Color, ColorSpace, Hsl, Hsv, LinearRgb, Luma, Oklab, Oklch, Rgb};

/// Same as [`Color::mix`], but takes an iterator instead of a vector.
pub fn mix_iter(
    colors: impl IntoIterator<
        Item = WeightedColor,
        IntoIter = impl ExactSizeIterator<Item = WeightedColor>,
    >,
    space: ColorSpace,
) -> StrResult<Color> {
    let mut colors = colors.into_iter();
    if space.hue_index().is_some() && colors.len() > 2 {
        bail!("cannot mix more than two colors in a hue-based space");
    }

    let m = if space.hue_index().is_some() && colors.len() == 2 {
        let mut m = [0.0; 4];

        let WeightedColor { color: c0, weight: w0 } = colors.next().unwrap();
        let WeightedColor { color: c1, weight: w1 } = colors.next().unwrap();

        let c0 = c0.to_space(space).to_vec4();
        let c1 = c1.to_space(space).to_vec4();
        let w0 = w0 as f32;
        let w1 = w1 as f32;

        if w0 + w1 <= 0.0 {
            bail!("sum of weights must be positive");
        }

        for i in 0..4 {
            m[i] = (w0 * c0[i] + w1 * c1[i]) / (w0 + w1);
        }

        // Ensure that the hue circle is traversed in the short direction.
        if let Some(index) = space.hue_index() {
            if (c0[index] - c1[index]).abs() > 180.0 {
                let (h0, h1) = if c0[index] < c1[index] {
                    (c0[index] + 360.0, c1[index])
                } else {
                    (c0[index], c1[index] + 360.0)
                };
                m[index] = (w0 * h0 + w1 * h1) / (w0 + w1);
            }
        }

        m
    } else {
        let mut total = 0.0;
        let mut acc = [0.0; 4];

        for WeightedColor { color, weight } in colors {
            let weight = weight as f32;
            let v = color.to_space(space).to_vec4();
            acc[0] += weight * v[0];
            acc[1] += weight * v[1];
            acc[2] += weight * v[2];
            acc[3] += weight * v[3];
            total += weight;
        }

        if total <= 0.0 {
            bail!("sum of weights must be positive");
        }

        acc.map(|v| v / total)
    };

    Ok(match space {
        ColorSpace::Oklab => Color::Oklab(Oklab::new(m[0], m[1], m[2], m[3])),
        ColorSpace::Oklch => Color::Oklch(Oklch::new(m[0], m[1], m[2], m[3])),
        ColorSpace::Srgb => Color::Rgb(Rgb::new(m[0], m[1], m[2], m[3])),
        ColorSpace::LinearRgb => Color::LinearRgb(LinearRgb::new(m[0], m[1], m[2], m[3])),
        ColorSpace::Hsl => {
            Color::Hsl(Hsl::new(RgbHue::from_degrees(m[0]), m[1], m[2], m[3]))
        }
        ColorSpace::Hsv => {
            Color::Hsv(Hsv::new(RgbHue::from_degrees(m[0]), m[1], m[2], m[3]))
        }
        ColorSpace::Cmyk => Color::Cmyk(Cmyk::new(m[0], m[1], m[2], m[3])),
        ColorSpace::D65Gray => Color::Luma(Luma::new(m[0], m[3])),
    })
}

/// A color with a weight.
pub struct WeightedColor {
    color: Color,
    weight: f64,
}

impl WeightedColor {
    /// Create a new weighted color.
    pub const fn new(color: Color, weight: f64) -> Self {
        Self { color, weight }
    }
}

cast! {
    WeightedColor,
    self => array![self.color, self.weight].into_value(),
    color: Color => Self { color, weight: 1.0 },
    v: Array => {
        let mut iter = v.into_iter();
        match (iter.next(), iter.next(), iter.next()) {
            (Some(c), Some(w), None) => Self {
                color: c.cast()?,
                weight: w.cast::<Weight>()?.0,
            },
            _ => bail!("expected a color or color-weight pair"),
        }
    }
}

/// A weight for color mixing.
struct Weight(f64);

cast! {
    Weight,
    v: f64 => Self(v),
    v: Ratio => Self(v.get()),
}
