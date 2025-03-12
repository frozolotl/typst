use std::fmt::{self, Debug};
use std::str::FromStr;

use ecow::{eco_format, EcoString};
use palette::FromColor;

use crate::foundations::Repr;

use super::*;

impl Color {
    /// Construct a new RGBA color from 8-bit values.
    pub fn from_u8(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self::Rgb(Rgb::new(
            f32::from(r) / 255.0,
            f32::from(g) / 255.0,
            f32::from(b) / 255.0,
            f32::from(a) / 255.0,
        ))
    }

    /// Converts a 32-bit integer to an RGBA color.
    pub fn from_u32(color: u32) -> Self {
        Self::from_u8(
            ((color >> 24) & 0xFF) as u8,
            ((color >> 16) & 0xFF) as u8,
            ((color >> 8) & 0xFF) as u8,
            (color & 0xFF) as u8,
        )
    }

    /// Converts the color to a vec of four floats.
    pub fn to_vec4(&self) -> [f32; 4] {
        match self {
            Color::Luma(c) => [c.luma, c.luma, c.luma, c.alpha],
            Color::Oklab(c) => [c.l, c.a, c.b, c.alpha],
            Color::Oklch(c) => {
                [c.l, c.chroma, c.hue.into_degrees().rem_euclid(360.0), c.alpha]
            }
            Color::Rgb(c) => [c.red, c.green, c.blue, c.alpha],
            Color::LinearRgb(c) => [c.red, c.green, c.blue, c.alpha],
            Color::Cmyk(c) => [c.c, c.m, c.y, c.k],
            Color::Hsl(c) => [
                c.hue.into_degrees().rem_euclid(360.0),
                c.saturation,
                c.lightness,
                c.alpha,
            ],
            Color::Hsv(c) => {
                [c.hue.into_degrees().rem_euclid(360.0), c.saturation, c.value, c.alpha]
            }
        }
    }

    /// Converts the color to a vec of four [`u8`]s.
    pub fn to_vec4_u8(&self) -> [u8; 4] {
        self.to_vec4().map(|x| (x * 255.0).round() as u8)
    }

    pub fn to_space(self, space: ColorSpace) -> Self {
        match space {
            ColorSpace::Oklab => self.to_oklab(),
            ColorSpace::Oklch => self.to_oklch(),
            ColorSpace::Srgb => self.to_rgb(),
            ColorSpace::LinearRgb => self.to_linear_rgb(),
            ColorSpace::Hsl => self.to_hsl(),
            ColorSpace::Hsv => self.to_hsv(),
            ColorSpace::Cmyk => self.to_cmyk(),
            ColorSpace::D65Gray => self.to_luma(),
        }
    }

    pub fn to_luma(self) -> Self {
        Self::Luma(match self {
            Self::Luma(c) => c,
            Self::Oklab(c) => Luma::from_color(c),
            Self::Oklch(c) => Luma::from_color(c),
            Self::Rgb(c) => Luma::from_color(c),
            Self::LinearRgb(c) => Luma::from_color(c),
            Self::Cmyk(c) => Luma::from_color(c.to_rgba()),
            Self::Hsl(c) => Luma::from_color(c),
            Self::Hsv(c) => Luma::from_color(c),
        })
    }

    pub fn to_oklab(self) -> Self {
        Self::Oklab(match self {
            Self::Luma(c) => Oklab::from_color(c),
            Self::Oklab(c) => c,
            Self::Oklch(c) => Oklab::from_color(c),
            Self::Rgb(c) => Oklab::from_color(c),
            Self::LinearRgb(c) => Oklab::from_color(c),
            Self::Cmyk(c) => Oklab::from_color(c.to_rgba()),
            Self::Hsl(c) => Oklab::from_color(c),
            Self::Hsv(c) => Oklab::from_color(c),
        })
    }

    pub fn to_oklch(self) -> Self {
        Self::Oklch(match self {
            Self::Luma(c) => Oklch::from_color(c),
            Self::Oklab(c) => Oklch::from_color(c),
            Self::Oklch(c) => c,
            Self::Rgb(c) => Oklch::from_color(c),
            Self::LinearRgb(c) => Oklch::from_color(c),
            Self::Cmyk(c) => Oklch::from_color(c.to_rgba()),
            Self::Hsl(c) => Oklch::from_color(c),
            Self::Hsv(c) => Oklch::from_color(c),
        })
    }

    pub fn to_rgb(self) -> Self {
        Self::Rgb(match self {
            Self::Luma(c) => Rgb::from_color(c),
            Self::Oklab(c) => Rgb::from_color(c),
            Self::Oklch(c) => Rgb::from_color(c),
            Self::Rgb(c) => c,
            Self::LinearRgb(c) => Rgb::from_linear(c),
            Self::Cmyk(c) => Rgb::from_color(c.to_rgba()),
            Self::Hsl(c) => Rgb::from_color(c),
            Self::Hsv(c) => Rgb::from_color(c),
        })
    }

    pub fn to_linear_rgb(self) -> Self {
        Self::LinearRgb(match self {
            Self::Luma(c) => LinearRgb::from_color(c),
            Self::Oklab(c) => LinearRgb::from_color(c),
            Self::Oklch(c) => LinearRgb::from_color(c),
            Self::Rgb(c) => LinearRgb::from_color(c),
            Self::LinearRgb(c) => c,
            Self::Cmyk(c) => LinearRgb::from_color(c.to_rgba()),
            Self::Hsl(c) => Rgb::from_color(c).into_linear(),
            Self::Hsv(c) => Rgb::from_color(c).into_linear(),
        })
    }

    pub fn to_cmyk(self) -> Self {
        Self::Cmyk(match self {
            Self::Luma(c) => Cmyk::from_luma(c),
            Self::Oklab(c) => Cmyk::from_rgba(Rgb::from_color(c)),
            Self::Oklch(c) => Cmyk::from_rgba(Rgb::from_color(c)),
            Self::Rgb(c) => Cmyk::from_rgba(c),
            Self::LinearRgb(c) => Cmyk::from_rgba(Rgb::from_linear(c)),
            Self::Cmyk(c) => c,
            Self::Hsl(c) => Cmyk::from_rgba(Rgb::from_color(c)),
            Self::Hsv(c) => Cmyk::from_rgba(Rgb::from_color(c)),
        })
    }

    pub fn to_hsl(self) -> Self {
        Self::Hsl(match self {
            Self::Luma(c) => Hsl::from_color(c),
            Self::Oklab(c) => Hsl::from_color(c),
            Self::Oklch(c) => Hsl::from_color(c),
            Self::Rgb(c) => Hsl::from_color(c),
            Self::LinearRgb(c) => Hsl::from_color(Rgb::from_linear(c)),
            Self::Cmyk(c) => Hsl::from_color(c.to_rgba()),
            Self::Hsl(c) => c,
            Self::Hsv(c) => Hsl::from_color(c),
        })
    }

    pub fn to_hsv(self) -> Self {
        Self::Hsv(match self {
            Self::Luma(c) => Hsv::from_color(c),
            Self::Oklab(c) => Hsv::from_color(c),
            Self::Oklch(c) => Hsv::from_color(c),
            Self::Rgb(c) => Hsv::from_color(c),
            Self::LinearRgb(c) => Hsv::from_color(Rgb::from_linear(c)),
            Self::Cmyk(c) => Hsv::from_color(c.to_rgba()),
            Self::Hsl(c) => Hsv::from_color(c),
            Self::Hsv(c) => c,
        })
    }
}

impl From<Luma> for Color {
    fn from(c: Luma) -> Self {
        Self::Luma(c)
    }
}

impl From<Oklab> for Color {
    fn from(c: Oklab) -> Self {
        Self::Oklab(c)
    }
}

impl From<Oklch> for Color {
    fn from(c: Oklch) -> Self {
        Self::Oklch(c)
    }
}

impl From<Rgb> for Color {
    fn from(c: Rgb) -> Self {
        Self::Rgb(c)
    }
}

impl From<LinearRgb> for Color {
    fn from(c: LinearRgb) -> Self {
        Self::LinearRgb(c)
    }
}

impl From<Cmyk> for Color {
    fn from(c: Cmyk) -> Self {
        Self::Cmyk(c)
    }
}

impl From<Hsl> for Color {
    fn from(c: Hsl) -> Self {
        Self::Hsl(c)
    }
}

impl From<Hsv> for Color {
    fn from(c: Hsv) -> Self {
        Self::Hsv(c)
    }
}

impl FromStr for Color {
    type Err = &'static str;

    /// Constructs a new color from hex strings like the following:
    /// - `#aef` (shorthand, with leading hash),
    /// - `7a03c2` (without alpha),
    /// - `abcdefff` (with alpha).
    ///
    /// The hash is optional and both lower and upper case are fine.
    fn from_str(hex_str: &str) -> Result<Self, Self::Err> {
        let hex_str = hex_str.strip_prefix('#').unwrap_or(hex_str);
        if hex_str.chars().any(|c| !c.is_ascii_hexdigit()) {
            return Err("color string contains non-hexadecimal letters");
        }

        let len = hex_str.len();
        let long = len == 6 || len == 8;
        let short = len == 3 || len == 4;
        let alpha = len == 4 || len == 8;
        if !long && !short {
            return Err("color string has wrong length");
        }

        let mut values: [u8; 4] = [u8::MAX; 4];
        for elem in if alpha { 0..4 } else { 0..3 } {
            let item_len = if long { 2 } else { 1 };
            let pos = elem * item_len;

            let item = &hex_str[pos..(pos + item_len)];
            values[elem] = u8::from_str_radix(item, 16).unwrap();

            if short {
                // Duplicate number for shorthand notation, i.e. `a` -> `aa`
                values[elem] += values[elem] * 16;
            }
        }

        Ok(Self::from_u8(values[0], values[1], values[2], values[3]))
    }
}

impl Debug for Color {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Luma(v) => write!(f, "Luma({}, {})", v.luma, v.alpha),
            Self::Oklab(v) => write!(f, "Oklab({}, {}, {}, {})", v.l, v.a, v.b, v.alpha),
            Self::Oklch(v) => {
                write!(
                    f,
                    "Oklch({}, {}, {:?}, {})",
                    v.l,
                    v.chroma,
                    v.hue.into_degrees(),
                    v.alpha
                )
            }
            Self::Rgb(v) => {
                write!(f, "Rgb({}, {}, {}, {})", v.red, v.green, v.blue, v.alpha)
            }
            Self::LinearRgb(v) => {
                write!(f, "LinearRgb({}, {}, {}, {})", v.red, v.green, v.blue, v.alpha)
            }
            Self::Cmyk(v) => write!(f, "Cmyk({}, {}, {}, {})", v.c, v.m, v.y, v.k),
            Self::Hsl(v) => write!(
                f,
                "Hsl({:?}, {}, {}, {})",
                v.hue.into_degrees(),
                v.saturation,
                v.lightness,
                v.alpha
            ),
            Self::Hsv(v) => write!(
                f,
                "Hsv({:?}, {}, {}, {})",
                v.hue.into_degrees(),
                v.saturation,
                v.value,
                v.alpha
            ),
        }
    }
}

impl Repr for Color {
    fn repr(&self) -> EcoString {
        match self {
            Self::Luma(c) => {
                eco_format!("luma({}{})", RatioComponent(c.luma), AlphaComponent(c.alpha))
            }
            Self::Rgb(c) => {
                if c.red.is_nan()
                    || c.green.is_nan()
                    || c.blue.is_nan()
                    || c.alpha.is_nan()
                {
                    eco_format!(
                        "rgb({}, {}, {}{})",
                        RatioComponent(c.red),
                        RatioComponent(c.green),
                        RatioComponent(c.blue),
                        AlphaComponent(c.alpha),
                    )
                } else {
                    eco_format!("rgb({})", self.to_hex().repr())
                }
            }
            Self::LinearRgb(c) => {
                eco_format!(
                    "color.linear-rgb({}, {}, {}{})",
                    RatioComponent(c.red),
                    RatioComponent(c.green),
                    RatioComponent(c.blue),
                    AlphaComponent(c.alpha),
                )
            }
            Self::Cmyk(c) => {
                eco_format!(
                    "cmyk({}, {}, {}, {})",
                    RatioComponent(c.c),
                    RatioComponent(c.m),
                    RatioComponent(c.y),
                    RatioComponent(c.k),
                )
            }
            Self::Oklab(c) => {
                eco_format!(
                    "oklab({}, {}, {}{})",
                    RatioComponent(c.l),
                    ChromaComponent(c.a),
                    ChromaComponent(c.b),
                    AlphaComponent(c.alpha),
                )
            }
            Self::Oklch(c) => {
                eco_format!(
                    "oklch({}, {}, {}{})",
                    RatioComponent(c.l),
                    ChromaComponent(c.chroma),
                    AngleComponent(c.hue.into_degrees()),
                    AlphaComponent(c.alpha),
                )
            }
            Self::Hsl(c) => {
                eco_format!(
                    "color.hsl({}, {}, {}{})",
                    AngleComponent(c.hue.into_degrees()),
                    RatioComponent(c.saturation),
                    RatioComponent(c.lightness),
                    AlphaComponent(c.alpha),
                )
            }
            Self::Hsv(c) => {
                eco_format!(
                    "color.hsv({}, {}, {}{})",
                    AngleComponent(c.hue.into_degrees()),
                    RatioComponent(c.saturation),
                    RatioComponent(c.value),
                    AlphaComponent(c.alpha),
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_color_strings() {
        #[track_caller]
        fn test(hex: &str, r: u8, g: u8, b: u8, a: u8) {
            assert_eq!(Color::from_str(hex), Ok(Color::from_u8(r, g, b, a)));
        }

        test("f61243ff", 0xf6, 0x12, 0x43, 255);
        test("b3d8b3", 0xb3, 0xd8, 0xb3, 255);
        test("fCd2a9AD", 0xfc, 0xd2, 0xa9, 0xad);
        test("233", 0x22, 0x33, 0x33, 255);
        test("111b", 0x11, 0x11, 0x11, 0xbb);
    }

    #[test]
    fn test_parse_invalid_colors() {
        #[track_caller]
        fn test(hex: &str, message: &str) {
            assert_eq!(Color::from_str(hex), Err(message));
        }

        test("a5", "color string has wrong length");
        test("12345", "color string has wrong length");
        test("f075ff011", "color string has wrong length");
        test("hmmm", "color string contains non-hexadecimal letters");
        test("14B2AH", "color string contains non-hexadecimal letters");
    }
}
