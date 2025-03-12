use std::sync::LazyLock;

use super::{Luma, Rgb};

/// The ICC profile used to convert from CMYK to RGB.
///
/// This is a minimal CMYK profile that only contains the necessary information
/// to convert from CMYK to RGB. It is based on the CGATS TR 001-1995
/// specification. See
/// <https://github.com/saucecontrol/Compact-ICC-Profiles#cmyk>.
static CMYK_TO_XYZ: LazyLock<Box<qcms::Profile>> = LazyLock::new(|| {
    qcms::Profile::new_from_slice(typst_assets::icc::CMYK_TO_XYZ, false).unwrap()
});

/// The target sRGB profile.
static SRGB_PROFILE: LazyLock<Box<qcms::Profile>> = LazyLock::new(|| {
    let mut out = qcms::Profile::new_sRGB();
    out.precache_output_transform();
    out
});

static TO_SRGB: LazyLock<qcms::Transform> = LazyLock::new(|| {
    qcms::Transform::new_to(
        &CMYK_TO_XYZ,
        &SRGB_PROFILE,
        qcms::DataType::CMYK,
        qcms::DataType::RGB8,
        // Our input profile only supports perceptual intent.
        qcms::Intent::Perceptual,
    )
    .unwrap()
});

/// An 8-bit CMYK color.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Cmyk {
    /// The cyan component.
    pub c: f32,
    /// The magenta component.
    pub m: f32,
    /// The yellow component.
    pub y: f32,
    /// The key (black) component.
    pub k: f32,
}

impl Cmyk {
    pub fn new(c: f32, m: f32, y: f32, k: f32) -> Self {
        Self { c, m, y, k }
    }

    pub fn from_luma(luma: Luma) -> Self {
        let l = 1.0 - luma.luma;
        Cmyk::new(l * 0.75, l * 0.68, l * 0.67, l * 0.90)
    }

    // This still uses naive conversion, because qcms does not support
    // converting to CMYK yet.
    pub fn from_rgba(rgba: Rgb) -> Self {
        let r = rgba.red;
        let g = rgba.green;
        let b = rgba.blue;

        let k = 1.0 - r.max(g).max(b);
        if k == 1.0 {
            return Cmyk::new(0.0, 0.0, 0.0, 1.0);
        }

        let c = (1.0 - r - k) / (1.0 - k);
        let m = (1.0 - g - k) / (1.0 - k);
        let y = (1.0 - b - k) / (1.0 - k);

        Cmyk::new(c, m, y, k)
    }

    pub fn to_rgba(self) -> Rgb {
        let mut dest: [u8; 3] = [0; 3];
        TO_SRGB.convert(
            &[
                (self.c * 255.0).round() as u8,
                (self.m * 255.0).round() as u8,
                (self.y * 255.0).round() as u8,
                (self.k * 255.0).round() as u8,
            ],
            &mut dest,
        );

        Rgb::new(
            f32::from(dest[0]) / 255.0,
            f32::from(dest[1]) / 255.0,
            f32::from(dest[2]) / 255.0,
            1.0,
        )
    }

    pub fn lighten(self, factor: f32) -> Self {
        let lighten = |u: f32| (u - u * factor).clamp(0.0, 1.0);
        Self::new(lighten(self.c), lighten(self.m), lighten(self.y), lighten(self.k))
    }

    pub fn darken(self, factor: f32) -> Self {
        let darken = |u: f32| (u + (1.0 - u) * factor).clamp(0.0, 1.0);
        Self::new(darken(self.c), darken(self.m), darken(self.y), darken(self.k))
    }
}
