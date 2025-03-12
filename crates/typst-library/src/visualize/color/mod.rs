mod cmyk;
mod convert;
mod map;
mod mix;

pub use cmyk::Cmyk;
pub use map::map;
pub use mix::{mix_iter, WeightedColor};

use std::fmt;
use std::hash::{Hash, Hasher};
use std::str::FromStr;

use ecow::{eco_format, EcoString};
use palette::encoding::{self, Linear};
use palette::{Alpha, Darken, Desaturate, Lighten, OklabHue, RgbHue, Saturate, ShiftHue};
use typst_syntax::{Span, Spanned};

use crate::diag::{bail, At, SourceResult, StrResult};
use crate::foundations::{
    array, cast, func, repr, scope, ty, Args, Array, IntoValue, Module, NoneValue, Str,
    Value,
};
use crate::layout::{Angle, Ratio};

// Type aliases for `palette` internal types in f32.
pub type Oklab = palette::oklab::Oklaba<f32>;
pub type Oklch = palette::oklch::Oklcha<f32>;
pub type LinearRgb = palette::rgb::Rgba<Linear<encoding::Srgb>, f32>;
pub type Rgb = palette::rgb::Rgba<encoding::Srgb, f32>;
pub type Hsl = palette::hsl::Hsla<encoding::Srgb, f32>;
pub type Hsv = palette::hsv::Hsva<encoding::Srgb, f32>;
pub type Luma = palette::luma::Lumaa<encoding::Srgb, f32>;

/// A color in a specific color space.
///
/// Typst supports:
/// - sRGB through the [`rgb` function]($color.rgb)
/// - Device CMYK through [`cmyk` function]($color.cmyk)
/// - D65 Gray through the [`luma` function]($color.luma)
/// - Oklab through the [`oklab` function]($color.oklab)
/// - Oklch through the [`oklch` function]($color.oklch)
/// - Linear RGB through the [`color.linear-rgb` function]($color.linear-rgb)
/// - HSL through the [`color.hsl` function]($color.hsl)
/// - HSV through the [`color.hsv` function]($color.hsv)
///
///
/// # Example
///
/// ```example
/// #rect(fill: aqua)
/// ```
///
/// # Predefined colors
/// Typst defines the following built-in colors:
///
/// | Color     | Definition         |
/// |-----------|:-------------------|
/// | `black`   | `{luma(0)}`        |
/// | `gray`    | `{luma(170)}`      |
/// | `silver`  | `{luma(221)}`      |
/// | `white`   | `{luma(255)}`      |
/// | `navy`    | `{rgb("#001f3f")}` |
/// | `blue`    | `{rgb("#0074d9")}` |
/// | `aqua`    | `{rgb("#7fdbff")}` |
/// | `teal`    | `{rgb("#39cccc")}` |
/// | `eastern` | `{rgb("#239dad")}` |
/// | `purple`  | `{rgb("#b10dc9")}` |
/// | `fuchsia` | `{rgb("#f012be")}` |
/// | `maroon`  | `{rgb("#85144b")}` |
/// | `red`     | `{rgb("#ff4136")}` |
/// | `orange`  | `{rgb("#ff851b")}` |
/// | `yellow`  | `{rgb("#ffdc00")}` |
/// | `olive`   | `{rgb("#3d9970")}` |
/// | `green`   | `{rgb("#2ecc40")}` |
/// | `lime`    | `{rgb("#01ff70")}` |
///
/// The predefined colors and the most important color constructors are
/// available globally and also in the color type's scope, so you can write
/// either `color.red` or just `red`.
///
/// ```preview
/// #let colors = (
///   "black", "gray", "silver", "white",
///   "navy", "blue", "aqua", "teal",
///   "eastern", "purple", "fuchsia",
///   "maroon", "red", "orange", "yellow",
///   "olive", "green", "lime",
/// )
///
/// #set text(font: "PT Sans")
/// #set page(width: auto)
/// #grid(
///   columns: 9,
///   gutter: 10pt,
///   ..colors.map(name => {
///       let col = eval(name)
///       let luminance = luma(col).components().first()
///       set text(fill: white) if luminance < 50%
///       set square(stroke: black) if col == white
///       set align(center + horizon)
///       square(size: 50pt,  fill: col, name)
///   })
/// )
/// ```
///
/// # Predefined color maps
/// Typst also includes a number of preset color maps that can be used for
/// [gradients]($gradient/#stops). These are simply arrays of colors defined in
/// the module `color.map`.
///
/// ```example
/// #circle(fill: gradient.linear(..color.map.crest))
/// ```
///
/// | Map        | Details                                                     |
/// |------------|:------------------------------------------------------------|
/// | `turbo`    | A perceptually uniform rainbow-like color map. Read [this blog post](https://ai.googleblog.com/2019/08/turbo-improved-rainbow-colormap-for.html) for more details. |
/// | `cividis`  | A blue to gray to yellow color map. See [this blog post](https://bids.github.io/colormap/) for more details. |
/// | `rainbow`  | Cycles through the full color spectrum. This color map is best used by setting the interpolation color space to [HSL]($color.hsl). The rainbow gradient is **not suitable** for data visualization because it is not perceptually uniform, so the differences between values become unclear to your readers. It should only be used for decorative purposes. |
/// | `spectral` | Red to yellow to blue color map.                            |
/// | `viridis`  | A purple to teal to yellow color map.                       |
/// | `inferno`  | A black to red to yellow color map.                         |
/// | `magma`    | A black to purple to yellow color map.                      |
/// | `plasma`   | A purple to pink to yellow color map.                       |
/// | `rocket`   | A black to red to white color map.                          |
/// | `mako`     | A black to teal to yellow color map.                        |
/// | `vlag`     | A light blue to white to red color map.                     |
/// | `icefire`  | A light teal to black to yellow color map.                  |
/// | `flare`    | A orange to purple color map that is perceptually uniform.  |
/// | `crest`    | A blue to white to red color map.                           |
///
/// Some popular presets are not included because they are not available under a
/// free licence. Others, like
/// [Jet](https://jakevdp.github.io/blog/2014/10/16/how-bad-is-your-colormap/),
/// are not included because they are not color blind friendly. Feel free to use
/// or create a package with other presets that are useful to you!
///
/// ```preview
/// #set page(width: auto, height: auto)
/// #set text(font: "PT Sans", size: 8pt)
///
/// #let maps = (
///   "turbo", "cividis", "rainbow", "spectral",
///   "viridis", "inferno", "magma", "plasma",
///   "rocket", "mako", "vlag", "icefire",
///   "flare", "crest",
/// )
///
/// #stack(dir: ltr, spacing: 3pt, ..maps.map((name) => {
///   let map = eval("color.map." + name)
///   stack(
///     dir: ttb,
///     block(
///       width: 15pt,
///       height: 100pt,
///       fill: gradient.linear(..map, angle: 90deg),
///     ),
///     block(
///       width: 15pt,
///       height: 32pt,
///       move(dy: 8pt, rotate(90deg, name)),
///     ),
///   )
/// }))
/// ```
#[ty(scope, cast)]
#[derive(Copy, Clone)]
pub enum Color {
    /// A 32-bit luma color.
    Luma(Luma),
    /// A 32-bit L\*a\*b\* color in the Oklab color space.
    Oklab(Oklab),
    /// A 32-bit LCh color in the Oklab color space.
    Oklch(Oklch),
    /// A 32-bit RGB color.
    Rgb(Rgb),
    /// A 32-bit linear RGB color.
    LinearRgb(LinearRgb),
    /// A 32-bit CMYK color.
    Cmyk(Cmyk),
    /// A 32-bit HSL color.
    Hsl(Hsl),
    /// A 32-bit HSV color.
    Hsv(Hsv),
}

#[scope]
impl Color {
    /// The module of preset color maps.
    pub const MAP: fn() -> Module = || typst_utils::singleton!(Module, map()).clone();

    pub const BLACK: Self = Self::Luma(Luma::new(0.0, 1.0));
    pub const GRAY: Self = Self::Luma(Luma::new(0.6666666, 1.0));
    pub const WHITE: Self = Self::Luma(Luma::new(1.0, 1.0));
    pub const SILVER: Self = Self::Luma(Luma::new(0.8666667, 1.0));
    pub const NAVY: Self = Self::Rgb(Rgb::new(0.0, 0.121569, 0.247059, 1.0));
    pub const BLUE: Self = Self::Rgb(Rgb::new(0.0, 0.454902, 0.85098, 1.0));
    pub const AQUA: Self = Self::Rgb(Rgb::new(0.4980392, 0.858823, 1.0, 1.0));
    pub const TEAL: Self = Self::Rgb(Rgb::new(0.223529, 0.8, 0.8, 1.0));
    pub const EASTERN: Self = Self::Rgb(Rgb::new(0.13725, 0.615686, 0.678431, 1.0));
    pub const PURPLE: Self = Self::Rgb(Rgb::new(0.694118, 0.050980, 0.788235, 1.0));
    pub const FUCHSIA: Self = Self::Rgb(Rgb::new(0.941177, 0.070588, 0.745098, 1.0));
    pub const MAROON: Self = Self::Rgb(Rgb::new(0.521569, 0.078431, 0.294118, 1.0));
    pub const RED: Self = Self::Rgb(Rgb::new(1.0, 0.254902, 0.211765, 1.0));
    pub const ORANGE: Self = Self::Rgb(Rgb::new(1.0, 0.521569, 0.105882, 1.0));
    pub const YELLOW: Self = Self::Rgb(Rgb::new(1.0, 0.8627451, 0.0, 1.0));
    pub const OLIVE: Self = Self::Rgb(Rgb::new(0.239216, 0.6, 0.4392157, 1.0));
    pub const GREEN: Self = Self::Rgb(Rgb::new(0.1803922, 0.8, 0.2509804, 1.0));
    pub const LIME: Self = Self::Rgb(Rgb::new(0.0039216, 1.0, 0.4392157, 1.0));

    /// Create a grayscale color.
    ///
    /// A grayscale color is represented internally by a single `lightness`
    /// component.
    ///
    /// These components are also available using the
    /// [`components`]($color.components) method.
    ///
    /// ```example
    /// #for x in range(250, step: 50) {
    ///   box(square(fill: luma(x)))
    /// }
    /// ```
    #[func]
    pub fn luma(
        args: &mut Args,
        /// The lightness component.
        #[external]
        lightness: Component,
        /// The alpha component.
        #[external]
        alpha: RatioComponent,
        /// Alternatively: The color to convert to grayscale.
        ///
        /// If this is given, the `lightness` should not be given.
        #[external]
        color: Color,
    ) -> SourceResult<Color> {
        Ok(if let Some(color) = args.find::<Color>()? {
            color.to_luma()
        } else {
            let Component(gray) = args.expect("gray component").unwrap_or(Component(1.0));
            let RatioComponent(alpha) = args.eat()?.unwrap_or(RatioComponent(1.0));
            Self::Luma(Luma::new(gray, alpha))
        })
    }

    /// Create an [Oklab](https://bottosson.github.io/posts/oklab/) color.
    ///
    /// This color space is well suited for the following use cases:
    /// - Color manipulation such as saturating while keeping perceived hue
    /// - Creating grayscale images with uniform perceived lightness
    /// - Creating smooth and uniform color transition and gradients
    ///
    /// A linear Oklab color is represented internally by an array of four
    /// components:
    /// - lightness ([`ratio`])
    /// - a ([`float`] or [`ratio`].
    ///   Ratios are relative to `{0.4}`; meaning `{50%}` is equal to `{0.2}`)
    /// - b ([`float`] or [`ratio`].
    ///   Ratios are relative to `{0.4}`; meaning `{50%}` is equal to `{0.2}`)
    /// - alpha ([`ratio`])
    ///
    /// These components are also available using the
    /// [`components`]($color.components) method.
    ///
    /// ```example
    /// #square(
    ///   fill: oklab(27%, 20%, -3%, 50%)
    /// )
    /// ```
    #[func]
    pub fn oklab(
        args: &mut Args,
        /// The lightness component.
        #[external]
        lightness: RatioComponent,
        /// The a ("green/red") component.
        #[external]
        a: ChromaComponent,
        /// The b ("blue/yellow") component.
        #[external]
        b: ChromaComponent,
        /// The alpha component.
        #[external]
        alpha: RatioComponent,
        /// Alternatively: The color to convert to Oklab.
        ///
        /// If this is given, the individual components should not be given.
        #[external]
        color: Color,
    ) -> SourceResult<Color> {
        Ok(if let Some(color) = args.find::<Color>()? {
            color.to_oklab()
        } else {
            let RatioComponent(l) = args.expect("lightness component")?;
            let ChromaComponent(a) = args.expect("A component")?;
            let ChromaComponent(b) = args.expect("B component")?;
            let RatioComponent(alpha) = args.eat()?.unwrap_or(RatioComponent(1.0));
            Self::Oklab(Oklab::new(l, a, b, alpha))
        })
    }

    /// Create an [Oklch](https://bottosson.github.io/posts/oklab/) color.
    ///
    /// This color space is well suited for the following use cases:
    /// - Color manipulation involving lightness, chroma, and hue
    /// - Creating grayscale images with uniform perceived lightness
    /// - Creating smooth and uniform color transition and gradients
    ///
    /// A linear Oklch color is represented internally by an array of four
    /// components:
    /// - lightness ([`ratio`])
    /// - chroma ([`float`] or [`ratio`].
    ///   Ratios are relative to `{0.4}`; meaning `{50%}` is equal to `{0.2}`)
    /// - hue ([`angle`])
    /// - alpha ([`ratio`])
    ///
    /// These components are also available using the
    /// [`components`]($color.components) method.
    ///
    /// ```example
    /// #square(
    ///   fill: oklch(40%, 0.2, 160deg, 50%)
    /// )
    /// ```
    #[func]
    pub fn oklch(
        args: &mut Args,
        /// The lightness component.
        #[external]
        lightness: RatioComponent,
        /// The chroma component.
        #[external]
        chroma: ChromaComponent,
        /// The hue component.
        #[external]
        hue: AngleComponent,
        /// The alpha component.
        #[external]
        alpha: RatioComponent,
        /// Alternatively: The color to convert to Oklch.
        ///
        /// If this is given, the individual components should not be given.
        #[external]
        color: Color,
    ) -> SourceResult<Color> {
        Ok(if let Some(color) = args.find::<Color>()? {
            color.to_oklch()
        } else {
            let RatioComponent(l) = args.expect("lightness component")?;
            let ChromaComponent(c) = args.expect("chroma component")?;
            let AngleComponent(h) = args.expect("hue component")?;
            let RatioComponent(alpha) = args.eat()?.unwrap_or(RatioComponent(1.0));
            Self::Oklch(Oklch::new(l, c, OklabHue::from_degrees(h), alpha))
        })
    }

    /// Create an RGB(A) color with linear luma.
    ///
    /// This color space is similar to sRGB, but with the distinction that the
    /// color component are not gamma corrected. This makes it easier to perform
    /// color operations such as blending and interpolation. Although, you
    /// should prefer to use the [`oklab` function]($color.oklab) for these.
    ///
    /// A linear RGB(A) color is represented internally by an array of four
    /// components:
    /// - red ([`ratio`])
    /// - green ([`ratio`])
    /// - blue ([`ratio`])
    /// - alpha ([`ratio`])
    ///
    /// These components are also available using the
    /// [`components`]($color.components) method.
    ///
    /// ```example
    /// #square(fill: color.linear-rgb(
    ///   30%, 50%, 10%,
    /// ))
    /// ```
    #[func(title = "Linear RGB")]
    pub fn linear_rgb(
        args: &mut Args,
        /// The red component.
        #[external]
        red: Component,
        /// The green component.
        #[external]
        green: Component,
        /// The blue component.
        #[external]
        blue: Component,
        /// The alpha component.
        #[external]
        alpha: Component,
        /// Alternatively: The color to convert to linear RGB(A).
        ///
        /// If this is given, the individual components should not be given.
        #[external]
        color: Color,
    ) -> SourceResult<Color> {
        Ok(if let Some(color) = args.find::<Color>()? {
            color.to_linear_rgb()
        } else {
            let Component(r) = args.expect("red component")?;
            let Component(g) = args.expect("green component")?;
            let Component(b) = args.expect("blue component")?;
            let Component(a) = args.eat()?.unwrap_or(Component(1.0));
            Self::LinearRgb(LinearRgb::new(r, g, b, a))
        })
    }

    /// Create an RGB(A) color.
    ///
    /// The color is specified in the sRGB color space.
    ///
    /// An RGB(A) color is represented internally by an array of four components:
    /// - red ([`ratio`])
    /// - green ([`ratio`])
    /// - blue ([`ratio`])
    /// - alpha ([`ratio`])
    ///
    /// These components are also available using the [`components`]($color.components)
    /// method.
    ///
    /// ```example
    /// #square(fill: rgb("#b1f2eb"))
    /// #square(fill: rgb(87, 127, 230))
    /// #square(fill: rgb(25%, 13%, 65%))
    /// ```
    #[func(title = "RGB")]
    pub fn rgb(
        args: &mut Args,
        /// The red component.
        #[external]
        red: Component,
        /// The green component.
        #[external]
        green: Component,
        /// The blue component.
        #[external]
        blue: Component,
        /// The alpha component.
        #[external]
        alpha: Component,
        /// Alternatively: The color in hexadecimal notation.
        ///
        /// Accepts three, four, six or eight hexadecimal digits and optionally
        /// a leading hash.
        ///
        /// If this is given, the individual components should not be given.
        ///
        /// ```example
        /// #text(16pt, rgb("#239dad"))[
        ///   *Typst*
        /// ]
        /// ```
        #[external]
        hex: Str,
        /// Alternatively: The color to convert to RGB(a).
        ///
        /// If this is given, the individual components should not be given.
        #[external]
        color: Color,
    ) -> SourceResult<Color> {
        Ok(if let Some(string) = args.find::<Spanned<Str>>()? {
            Self::from_str(&string.v).at(string.span)?
        } else if let Some(color) = args.find::<Color>()? {
            color.to_rgb()
        } else {
            let Component(r) = args.expect("red component")?;
            let Component(g) = args.expect("green component")?;
            let Component(b) = args.expect("blue component")?;
            let Component(a) = args.eat()?.unwrap_or(Component(1.0));
            Self::Rgb(Rgb::new(r, g, b, a))
        })
    }

    /// Create a CMYK color.
    ///
    /// This is useful if you want to target a specific printer. The conversion
    /// to RGB for display preview might differ from how your printer reproduces
    /// the color.
    ///
    /// A CMYK color is represented internally by an array of four components:
    /// - cyan ([`ratio`])
    /// - magenta ([`ratio`])
    /// - yellow ([`ratio`])
    /// - key ([`ratio`])
    ///
    /// These components are also available using the
    /// [`components`]($color.components) method.
    ///
    /// Note that CMYK colors are not currently supported when PDF/A output is
    /// enabled.
    ///
    /// ```example
    /// #square(
    ///   fill: cmyk(27%, 0%, 3%, 5%)
    /// )
    /// ```
    #[func(title = "CMYK")]
    pub fn cmyk(
        args: &mut Args,
        /// The cyan component.
        #[external]
        cyan: RatioComponent,
        /// The magenta component.
        #[external]
        magenta: RatioComponent,
        /// The yellow component.
        #[external]
        yellow: RatioComponent,
        /// The key component.
        #[external]
        key: RatioComponent,
        /// Alternatively: The color to convert to CMYK.
        ///
        /// If this is given, the individual components should not be given.
        #[external]
        color: Color,
    ) -> SourceResult<Color> {
        Ok(if let Some(color) = args.find::<Color>()? {
            color.to_cmyk()
        } else {
            let RatioComponent(c) = args.expect("cyan component")?;
            let RatioComponent(m) = args.expect("magenta component")?;
            let RatioComponent(y) = args.expect("yellow component")?;
            let RatioComponent(k) = args.expect("key/black component")?;
            Self::Cmyk(Cmyk::new(c, m, y, k))
        })
    }

    /// Create an HSL color.
    ///
    /// This color space is useful for specifying colors by hue, saturation and
    /// lightness. It is also useful for color manipulation, such as saturating
    /// while keeping perceived hue.
    ///
    /// An HSL color is represented internally by an array of four components:
    /// - hue ([`angle`])
    /// - saturation ([`ratio`])
    /// - lightness ([`ratio`])
    /// - alpha ([`ratio`])
    ///
    /// These components are also available using the
    /// [`components`]($color.components) method.
    ///
    /// ```example
    /// #square(
    ///   fill: color.hsl(30deg, 50%, 60%)
    /// )
    /// ```
    #[func(title = "HSL")]
    pub fn hsl(
        args: &mut Args,
        /// The hue angle.
        #[external]
        hue: AngleComponent,
        /// The saturation component.
        #[external]
        saturation: Component,
        /// The lightness component.
        #[external]
        lightness: Component,
        /// The alpha component.
        #[external]
        alpha: Component,
        /// Alternatively: The color to convert to HSL.
        ///
        /// If this is given, the individual components should not be given.
        #[external]
        color: Color,
    ) -> SourceResult<Color> {
        Ok(if let Some(color) = args.find::<Color>()? {
            color.to_hsl()
        } else {
            let AngleComponent(h) = args.expect("hue component")?;
            let Component(s) = args.expect("saturation component")?;
            let Component(l) = args.expect("lightness component")?;
            let Component(a) = args.eat()?.unwrap_or(Component(1.0));
            Self::Hsl(Hsl::new(RgbHue::from_degrees(h), s, l, a))
        })
    }

    /// Create an HSV color.
    ///
    /// This color space is useful for specifying colors by hue, saturation and
    /// value. It is also useful for color manipulation, such as saturating
    /// while keeping perceived hue.
    ///
    /// An HSV color is represented internally by an array of four components:
    /// - hue ([`angle`])
    /// - saturation ([`ratio`])
    /// - value ([`ratio`])
    /// - alpha ([`ratio`])
    ///
    /// These components are also available using the
    /// [`components`]($color.components) method.
    ///
    /// ```example
    /// #square(
    ///   fill: color.hsv(30deg, 50%, 60%)
    /// )
    /// ```
    #[func(title = "HSV")]
    pub fn hsv(
        args: &mut Args,
        /// The hue angle.
        #[external]
        hue: AngleComponent,
        /// The saturation component.
        #[external]
        saturation: Component,
        /// The value component.
        #[external]
        value: Component,
        /// The alpha component.
        #[external]
        alpha: Component,
        /// Alternatively: The color to convert to HSL.
        ///
        /// If this is given, the individual components should not be given.
        #[external]
        color: Color,
    ) -> SourceResult<Color> {
        Ok(if let Some(color) = args.find::<Color>()? {
            color.to_hsv()
        } else {
            let AngleComponent(h) = args.expect("hue component")?;
            let Component(s) = args.expect("saturation component")?;
            let Component(v) = args.expect("value component")?;
            let Component(a) = args.eat()?.unwrap_or(Component(1.0));
            Self::Hsv(Hsv::new(RgbHue::from_degrees(h), s, v, a))
        })
    }

    /// Extracts the components of this color.
    ///
    /// The size and values of this array depends on the color space. You can
    /// obtain the color space using [`space`]($color.space). Below is a table
    /// of the color spaces and their components:
    ///
    /// |       Color space       |     C1    |     C2     |     C3    |   C4   |
    /// |-------------------------|-----------|------------|-----------|--------|
    /// | [`luma`]($color.luma)   | Lightness |            |           |        |
    /// | [`oklab`]($color.oklab) | Lightness |    `a`     |    `b`    |  Alpha |
    /// | [`oklch`]($color.oklch) | Lightness |   Chroma   |    Hue    |  Alpha |
    /// | [`linear-rgb`]($color.linear-rgb) | Red  |   Green |    Blue |  Alpha |
    /// | [`rgb`]($color.rgb)     |    Red    |   Green    |    Blue   |  Alpha |
    /// | [`cmyk`]($color.cmyk)   |    Cyan   |   Magenta  |   Yellow  |  Key   |
    /// | [`hsl`]($color.hsl)     |     Hue   | Saturation | Lightness |  Alpha |
    /// | [`hsv`]($color.hsv)     |     Hue   | Saturation |   Value   |  Alpha |
    ///
    /// For the meaning and type of each individual value, see the documentation
    /// of the corresponding color space. The alpha component is optional and
    /// only included if the `alpha` argument is `true`. The length of the
    /// returned array depends on the number of components and whether the alpha
    /// component is included.
    ///
    /// ```example
    /// // note that the alpha component is included by default
    /// #rgb(40%, 60%, 80%).components()
    /// ```
    #[func]
    pub fn components(
        self,
        /// Whether to include the alpha component.
        #[named]
        #[default(true)]
        alpha: bool,
    ) -> Array {
        fn scalar(x: f32) -> Value {
            if x.is_nan() {
                NoneValue.into_value()
            } else {
                f64::from(x).into_value()
            }
        }
        fn ratio(x: f32) -> Value {
            if x.is_nan() {
                NoneValue.into_value()
            } else {
                Ratio::new(x.into()).into_value()
            }
        }
        fn angle(degrees: f32) -> Value {
            if degrees.is_nan() {
                NoneValue.into_value()
            } else {
                Angle::deg(f64::from(degrees).rem_euclid(360.0)).into_value()
            }
        }

        let mut components = match self {
            Self::Luma(c) => {
                array![ratio(c.luma), ratio(c.alpha)]
            }
            Self::Oklab(c) => {
                array![ratio(c.l), scalar(c.a), scalar(c.b), ratio(c.alpha)]
            }
            Self::Oklch(c) => {
                array![
                    ratio(c.l),
                    scalar(c.chroma),
                    angle(c.hue.into_degrees()),
                    ratio(c.alpha),
                ]
            }
            Self::LinearRgb(c) => {
                array![ratio(c.red), ratio(c.green), ratio(c.blue), ratio(c.alpha),]
            }
            Self::Rgb(c) => {
                array![ratio(c.red), ratio(c.green), ratio(c.blue), ratio(c.alpha),]
            }
            Self::Cmyk(c) => {
                array![ratio(c.c), ratio(c.m), ratio(c.y), ratio(c.k)]
            }
            Self::Hsl(c) => {
                array![
                    angle(c.hue.into_degrees()),
                    ratio(c.saturation),
                    ratio(c.lightness),
                    ratio(c.alpha),
                ]
            }
            Self::Hsv(c) => {
                array![
                    angle(c.hue.into_degrees()),
                    ratio(c.saturation),
                    ratio(c.value),
                    ratio(c.alpha),
                ]
            }
        };
        // Remove the alpha component if the corresponding argument was set.
        if !alpha && !matches!(self, Self::Cmyk(_)) {
            let _ = components.pop();
        }
        components
    }

    /// Returns the constructor function for this color's space:
    /// - [`luma`]($color.luma)
    /// - [`oklab`]($color.oklab)
    /// - [`oklch`]($color.oklch)
    /// - [`linear-rgb`]($color.linear-rgb)
    /// - [`rgb`]($color.rgb)
    /// - [`cmyk`]($color.cmyk)
    /// - [`hsl`]($color.hsl)
    /// - [`hsv`]($color.hsv)
    ///
    /// ```example
    /// #let color = cmyk(1%, 2%, 3%, 4%)
    /// #(color.space() == cmyk)
    /// ```
    #[func]
    pub fn space(self) -> ColorSpace {
        match self {
            Self::Luma(_) => ColorSpace::D65Gray,
            Self::Oklab(_) => ColorSpace::Oklab,
            Self::Oklch(_) => ColorSpace::Oklch,
            Self::LinearRgb(_) => ColorSpace::LinearRgb,
            Self::Rgb(_) => ColorSpace::Srgb,
            Self::Cmyk(_) => ColorSpace::Cmyk,
            Self::Hsl(_) => ColorSpace::Hsl,
            Self::Hsv(_) => ColorSpace::Hsv,
        }
    }

    /// Returns the color's RGB(A) hex representation (such as `#ffaa32` or
    /// `#020304fe`). The alpha component (last two digits in `#020304fe`) is
    /// omitted if it is equal to `ff` (255 / 100%).
    ///
    /// Missing components are normalized to zero.
    #[func]
    pub fn to_hex(self) -> EcoString {
        let [r, g, b, a] = self.to_rgb().normalize().to_vec4_u8();
        if a != 255 {
            eco_format!("#{:02x}{:02x}{:02x}{:02x}", r, g, b, a)
        } else {
            eco_format!("#{:02x}{:02x}{:02x}", r, g, b)
        }
    }

    /// Lightens a color by a given factor.
    #[func]
    pub fn lighten(
        self,
        /// The factor to lighten the color by.
        factor: Ratio,
    ) -> Color {
        let factor = factor.get() as f32;
        match self {
            Self::Luma(c) => Self::Luma(c.lighten(factor)),
            Self::Oklab(c) => Self::Oklab(c.lighten(factor)),
            Self::Oklch(c) => Self::Oklch(c.lighten(factor)),
            Self::LinearRgb(c) => Self::LinearRgb(c.lighten(factor)),
            Self::Rgb(c) => Self::Rgb(c.lighten(factor)),
            Self::Cmyk(c) => Self::Cmyk(c.lighten(factor)),
            Self::Hsl(c) => Self::Hsl(c.lighten(factor)),
            Self::Hsv(c) => Self::Hsv(c.lighten(factor)),
        }
    }

    /// Darkens a color by a given factor.
    #[func]
    pub fn darken(
        self,
        /// The factor to darken the color by.
        factor: Ratio,
    ) -> Color {
        let factor = factor.get() as f32;
        match self {
            Self::Luma(c) => Self::Luma(c.darken(factor)),
            Self::Oklab(c) => Self::Oklab(c.darken(factor)),
            Self::Oklch(c) => Self::Oklch(c.darken(factor)),
            Self::LinearRgb(c) => Self::LinearRgb(c.darken(factor)),
            Self::Rgb(c) => Self::Rgb(c.darken(factor)),
            Self::Cmyk(c) => Self::Cmyk(c.darken(factor)),
            Self::Hsl(c) => Self::Hsl(c.darken(factor)),
            Self::Hsv(c) => Self::Hsv(c.darken(factor)),
        }
    }

    /// Increases the saturation of a color by a given factor.
    #[func]
    pub fn saturate(
        self,
        span: Span,
        /// The factor to saturate the color by.
        factor: Ratio,
    ) -> SourceResult<Color> {
        Ok(match self {
            Self::Luma(_) => {
                bail!(
                    span, "cannot saturate grayscale color";
                    hint: "try converting your color to RGB first"
                );
            }
            Self::Oklab(_) => self.to_hsv().saturate(span, factor)?.to_oklab(),
            Self::Oklch(_) => self.to_hsv().saturate(span, factor)?.to_oklch(),
            Self::LinearRgb(_) => self.to_hsv().saturate(span, factor)?.to_linear_rgb(),
            Self::Rgb(_) => self.to_hsv().saturate(span, factor)?.to_rgb(),
            Self::Cmyk(_) => self.to_hsv().saturate(span, factor)?.to_cmyk(),
            Self::Hsl(c) => Self::Hsl(c.saturate(factor.get() as f32)),
            Self::Hsv(c) => Self::Hsv(c.saturate(factor.get() as f32)),
        })
    }

    /// Decreases the saturation of a color by a given factor.
    #[func]
    pub fn desaturate(
        self,
        span: Span,
        /// The factor to desaturate the color by.
        factor: Ratio,
    ) -> SourceResult<Color> {
        Ok(match self {
            Self::Luma(_) => {
                bail!(
                    span, "cannot desaturate grayscale color";
                    hint: "try converting your color to RGB first"
                );
            }
            Self::Oklab(_) => self.to_hsv().desaturate(span, factor)?.to_oklab(),
            Self::Oklch(_) => self.to_hsv().desaturate(span, factor)?.to_oklch(),
            Self::LinearRgb(_) => self.to_hsv().desaturate(span, factor)?.to_linear_rgb(),
            Self::Rgb(_) => self.to_hsv().desaturate(span, factor)?.to_rgb(),
            Self::Cmyk(_) => self.to_hsv().desaturate(span, factor)?.to_cmyk(),
            Self::Hsl(c) => Self::Hsl(c.desaturate(factor.get() as f32)),
            Self::Hsv(c) => Self::Hsv(c.desaturate(factor.get() as f32)),
        })
    }

    /// Produces the complementary color using a provided color space.
    /// You can think of it as the opposite side on a color wheel.
    ///
    /// ```example
    /// #square(fill: yellow)
    /// #square(fill: yellow.negate())
    /// #square(fill: yellow.negate(space: rgb))
    /// ```
    #[func]
    pub fn negate(
        self,
        /// The color space used for the transformation. By default, a perceptual color space is used.
        #[named]
        #[default(ColorSpace::Oklab)]
        space: ColorSpace,
    ) -> Color {
        let result = match self.to_space(space) {
            Self::Luma(c) => Self::Luma(Luma::new(1.0 - c.luma, c.alpha)),
            Self::Oklab(c) => Self::Oklab(Oklab::new(1.0 - c.l, -c.a, -c.b, c.alpha)),
            Self::Oklch(c) => Self::Oklch(Oklch::new(
                1.0 - c.l,
                c.chroma,
                OklabHue::from_degrees(c.hue.into_degrees() + 180.0),
                c.alpha,
            )),
            Self::LinearRgb(c) => Self::LinearRgb(LinearRgb::new(
                1.0 - c.red,
                1.0 - c.green,
                1.0 - c.blue,
                c.alpha,
            )),
            Self::Rgb(c) => {
                Self::Rgb(Rgb::new(1.0 - c.red, 1.0 - c.green, 1.0 - c.blue, c.alpha))
            }
            Self::Cmyk(c) => Self::Cmyk(Cmyk::new(1.0 - c.c, 1.0 - c.m, 1.0 - c.y, c.k)),
            Self::Hsl(c) => Self::Hsl(Hsl::new(
                RgbHue::from_degrees(c.hue.into_degrees() + 180.0),
                c.saturation,
                c.lightness,
                c.alpha,
            )),
            Self::Hsv(c) => Self::Hsv(Hsv::new(
                RgbHue::from_degrees(c.hue.into_degrees() + 180.0),
                c.saturation,
                c.value,
                c.alpha,
            )),
        };
        result.to_space(self.space())
    }

    /// Rotates the hue of the color by a given angle.
    #[func]
    pub fn rotate(
        self,
        span: Span,
        /// The angle to rotate the hue by.
        angle: Angle,
        /// The color space used to rotate. By default, this happens in a perceptual
        /// color space ([`oklch`]($color.oklch)).
        #[named]
        #[default(ColorSpace::Oklch)]
        space: ColorSpace,
    ) -> SourceResult<Color> {
        Ok(match space {
            ColorSpace::Oklch => {
                let Self::Oklch(oklch) = self.to_oklch() else {
                    unreachable!();
                };
                let rotated = oklch.shift_hue(angle.to_deg() as f32);
                Self::Oklch(rotated).to_space(self.space())
            }
            ColorSpace::Hsl => {
                let Self::Hsl(hsl) = self.to_hsl() else {
                    unreachable!();
                };
                let rotated = hsl.shift_hue(angle.to_deg() as f32);
                Self::Hsl(rotated).to_space(self.space())
            }
            ColorSpace::Hsv => {
                let Self::Hsv(hsv) = self.to_hsv() else {
                    unreachable!();
                };
                let rotated = hsv.shift_hue(angle.to_deg() as f32);
                Self::Hsv(rotated).to_space(self.space())
            }
            _ => bail!(span, "this colorspace does not support hue rotation"),
        })
    }

    /// Create a color by mixing two or more colors.
    ///
    /// In color spaces with a hue component (hsl, hsv, oklch), only two colors
    /// can be mixed at once. Mixing more than two colors in such a space will
    /// result in an error!
    ///
    /// ```example
    /// #set block(height: 20pt, width: 100%)
    /// #block(fill: red.mix(blue))
    /// #block(fill: red.mix(blue, space: rgb))
    /// #block(fill: color.mix(red, blue, white))
    /// #block(fill: color.mix((red, 70%), (blue, 30%)))
    /// ```
    #[func]
    pub fn mix(
        /// The colors, optionally with weights, specified as a pair (array of
        /// length two) of color and weight (float or ratio).
        ///
        /// The weights do not need to add to `{100%}`, they are relative to the
        /// sum of all weights.
        #[variadic]
        colors: Vec<WeightedColor>,
        /// The color space to mix in. By default, this happens in a perceptual
        /// color space ([`oklab`]($color.oklab)).
        #[named]
        #[default(ColorSpace::Oklab)]
        space: ColorSpace,
    ) -> StrResult<Color> {
        mix::mix_iter(colors, space)
    }

    /// Makes a color more transparent by a given factor.
    ///
    /// This method is relative to the existing alpha value.
    /// If the scale is positive, calculates `alpha - alpha * scale`.
    /// Negative scales behave like `color.opacify(-scale)`.
    ///
    /// ```example
    /// #block(fill: red)[opaque]
    /// #block(fill: red.transparentize(50%))[half red]
    /// #block(fill: red.transparentize(75%))[quarter red]
    /// ```
    #[func]
    pub fn transparentize(
        self,
        /// The factor to change the alpha value by.
        scale: Ratio,
    ) -> StrResult<Color> {
        self.scale_alpha(-scale)
    }

    /// Makes a color more opaque by a given scale.
    ///
    /// This method is relative to the existing alpha value.
    /// If the scale is positive, calculates `alpha + scale - alpha * scale`.
    /// Negative scales behave like `color.transparentize(-scale)`.
    ///
    /// ```example
    /// #let half-red = red.transparentize(50%)
    /// #block(fill: half-red.opacify(100%))[opaque]
    /// #block(fill: half-red.opacify(50%))[three quarters red]
    /// #block(fill: half-red.opacify(-50%))[one quarter red]
    /// ```
    #[func]
    pub fn opacify(
        self,
        /// The scale to change the alpha value by.
        scale: Ratio,
    ) -> StrResult<Color> {
        self.scale_alpha(scale)
    }
}

impl Color {
    /// Replace any missing components (represented as NaN) with zero.
    pub fn normalize(mut self) -> Color {
        let components: &mut [f32] = match &mut self {
            Color::Luma(c) => c.as_mut(),
            Color::Oklab(c) => c.as_mut(),
            Color::Oklch(c) => c.as_mut(),
            Color::Rgb(c) => c.as_mut(),
            Color::LinearRgb(c) => c.as_mut(),
            Color::Hsl(c) => c.as_mut(),
            Color::Hsv(c) => c.as_mut(),
            // Special-cased because it's not part of [`palette`].
            Color::Cmyk(c) => {
                if c.c.is_nan() {
                    c.c = 0.0;
                }
                if c.m.is_nan() {
                    c.m = 0.0;
                }
                if c.y.is_nan() {
                    c.y = 0.0;
                }
                if c.k.is_nan() {
                    c.k = 0.0;
                }
                return self;
            }
        };
        for component in components {
            if component.is_nan() {
                *component = 0.0;
            }
        }
        self
    }

    /// Returns the alpha channel of the color, if it has one.
    pub fn alpha(&self) -> Option<f32> {
        match self {
            Color::Cmyk(_) => None,
            Color::Luma(c) => Some(c.alpha),
            Color::Oklab(c) => Some(c.alpha),
            Color::Oklch(c) => Some(c.alpha),
            Color::Rgb(c) => Some(c.alpha),
            Color::LinearRgb(c) => Some(c.alpha),
            Color::Hsl(c) => Some(c.alpha),
            Color::Hsv(c) => Some(c.alpha),
        }
    }

    /// Sets the alpha channel of the color, if it has one.
    pub fn with_alpha(mut self, alpha: f32) -> Self {
        match &mut self {
            Color::Cmyk(_) => {}
            Color::Luma(c) => c.alpha = alpha,
            Color::Oklab(c) => c.alpha = alpha,
            Color::Oklch(c) => c.alpha = alpha,
            Color::Rgb(c) => c.alpha = alpha,
            Color::LinearRgb(c) => c.alpha = alpha,
            Color::Hsl(c) => c.alpha = alpha,
            Color::Hsv(c) => c.alpha = alpha,
        }

        self
    }

    /// Scales the alpha value of a color by a given amount.
    ///
    /// For positive scales, computes `alpha + scale - alpha * scale`.
    /// For non-positive scales, computes `alpha + alpha * scale`.
    fn scale_alpha(self, scale: Ratio) -> StrResult<Color> {
        #[inline]
        fn transform<C>(mut color: Alpha<C, f32>, scale: Ratio) -> Alpha<C, f32> {
            let scale = scale.get() as f32;
            let factor = if scale > 0.0 { 1.0 - color.alpha } else { color.alpha };
            color.alpha = (color.alpha + scale * factor).clamp(0.0, 1.0);
            color
        }

        Ok(match self {
            Color::Luma(c) => Color::Luma(transform(c, scale)),
            Color::Oklab(c) => Color::Oklab(transform(c, scale)),
            Color::Oklch(c) => Color::Oklch(transform(c, scale)),
            Color::Rgb(c) => Color::Rgb(transform(c, scale)),
            Color::LinearRgb(c) => Color::LinearRgb(transform(c, scale)),
            Color::Cmyk(_) => bail!("CMYK does not have an alpha component"),
            Color::Hsl(c) => Color::Hsl(transform(c, scale)),
            Color::Hsv(c) => Color::Hsv(transform(c, scale)),
        })
    }
}

impl PartialEq for Color {
    fn eq(&self, other: &Self) -> bool {
        let space = self.space();
        if space != other.space() {
            return false;
        }

        let mut zipped = self.to_vec4().into_iter().zip(other.to_vec4());
        if matches!(space, ColorSpace::Srgb | ColorSpace::D65Gray) {
            zipped.all(|(a, b)| {
                if a.is_nan() && b.is_nan() {
                    true
                } else {
                    let round = |x: f32| (x * 255.0).round() as u8;
                    round(a) == round(b)
                }
            })
        } else {
            zipped.all(|(a, b)| (a.is_nan() && b.is_nan()) || (a == b))
        }
    }
}

impl Eq for Color {}

impl Hash for Color {
    fn hash<H: Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
        let [x, y, z, w] = self.to_vec4();
        x.to_bits().hash(state);
        y.to_bits().hash(state);
        z.to_bits().hash(state);
        w.to_bits().hash(state);
    }
}

/// A color space for color manipulation.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum ColorSpace {
    /// The perceptual Oklab color space.
    Oklab,
    /// The perceptual Oklch color space.
    Oklch,
    /// The standard RGB color space.
    Srgb,
    /// The D65-gray color space.
    D65Gray,
    /// The linear RGB color space.
    LinearRgb,
    /// The HSL color space.
    Hsl,
    /// The HSV color space.
    Hsv,
    /// The CMYK color space.
    Cmyk,
}

impl ColorSpace {
    /// Returns the index of the hue component in this color space, if it has
    /// one.
    pub fn hue_index(&self) -> Option<usize> {
        match self {
            Self::Hsl | Self::Hsv => Some(0),
            Self::Oklch => Some(2),
            _ => None,
        }
    }
}

cast! {
    ColorSpace,
    self => match self {
        Self::Oklab => Color::oklab_data(),
        Self::Oklch => Color::oklch_data(),
        Self::Srgb => Color::rgb_data(),
        Self::D65Gray => Color::luma_data(),
        Self::LinearRgb => Color::linear_rgb_data(),
        Self::Hsl => Color::hsl_data(),
        Self::Hsv => Color::hsv_data(),
        Self::Cmyk => Color::cmyk_data(),
    }.into_value(),
    v: Value => {
        let expected = "expected `rgb`, `luma`, `cmyk`, `oklab`, `oklch`, `color.linear-rgb`, `color.hsl`, or `color.hsv`";
        let Value::Func(func) = v else {
            bail!("{expected}, found {}", v.ty());
        };

        // Here comparing the function pointer since it's `Eq`
        // whereas the `NativeFuncData` is not.
        if func == Color::oklab_data() {
            Self::Oklab
        } else if func == Color::oklch_data() {
            Self::Oklch
        } else if func == Color::rgb_data() {
            Self::Srgb
        } else if func == Color::luma_data() {
            Self::D65Gray
        } else if func == Color::linear_rgb_data() {
            Self::LinearRgb
        } else if func == Color::hsl_data() {
            Self::Hsl
        } else if func == Color::hsv_data() {
            Self::Hsv
        } else if func == Color::cmyk_data() {
            Self::Cmyk
        } else {
            bail!("{expected}");
        }
    },
}

/// An integer or ratio component.
///
/// Must either be:
/// - a ratio between 0% and 100% inclusive.
/// - an integer between 0 and 255 inclusive.
/// - `{none}`, in which case it is ["missing"](https://www.w3.org/TR/css-color-4/#missing).
pub struct Component(f32);

cast! {
    Component,
    v: i64 => match v {
        0..=255 => Self(v as f32 / 255.0),
        _ => bail!("number must be between 0 and 255"),
    },
    v: Ratio => if (0.0..=1.0).contains(&v.get()) {
        Self(v.get() as f32)
    } else {
        bail!("ratio must be between 0% and 100%");
    },
    _: NoneValue => Self(f32::NAN),
}

impl fmt::Display for Component {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        RatioComponent(self.0).fmt(f)
    }
}

/// A component that must be a ratio.
///
/// Must either be:
/// - a ratio between 0% and 100% inclusive.
/// - `{none}`, in which case it is ["missing"](https://www.w3.org/TR/css-color-4/#missing).
pub struct RatioComponent(f32);

cast! {
    RatioComponent,
    v: Ratio => if (0.0..=1.0).contains(&v.get()) {
        Self(v.get() as f32)
    } else {
        bail!("ratio must be between 0% and 100%");
    },
    _: NoneValue => Self(f32::NAN),
}

impl fmt::Display for RatioComponent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_nan() {
            f.write_str("none")
        } else {
            f.write_str(&repr::format_float_with_unit(f64::from(self.0) * 100.0, "%"))
        }
    }
}

/// A hue angle in degrees.
///
/// Must either be:
/// - an angle.
/// - `{none}`, in which case it is ["missing"](https://www.w3.org/TR/css-color-4/#missing).
pub struct AngleComponent(f32);

cast! {
    AngleComponent,
    v: Angle => Self(v.to_deg() as f32),
    _: NoneValue => Self(f32::NAN),
}

impl fmt::Display for AngleComponent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_nan() {
            f.write_str("none")
        } else {
            let angle = self.0.rem_euclid(360.0).into();
            f.write_str(&repr::format_float_with_unit(angle, "deg"))
        }
    }
}

/// A chroma color component.
///
/// Must either be:
/// - a ratio, in which case it is relative to 0.4.
/// - a float, in which case it is taken literally.
/// - `{none}`, in which case it is ["missing"](https://www.w3.org/TR/css-color-4/#missing).
pub struct ChromaComponent(f32);

cast! {
    ChromaComponent,
    v: f64 => if v.is_finite() {
        Self(v as f32)
    } else {
        bail!("number must neither be infinite nor NaN");
    },
    v: Ratio => Self((v.get() * 0.4) as f32),
    _: NoneValue => Self(f32::NAN),
}

impl fmt::Display for ChromaComponent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_nan() {
            f.write_str("none")
        } else {
            f.write_str(&repr::format_float_component(self.0.into()))
        }
    }
}

/// The alpha value of a color.
///
/// This is exclusively intended for the [`Repr`] implementation.
pub struct AlphaComponent(f32);

impl fmt::Display for AlphaComponent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0 == 1.0 {
            Ok(())
        } else if self.0.is_nan() {
            f.write_str(", none")
        } else {
            write!(
                f,
                ", {}",
                repr::format_float_with_unit(f64::from(self.0) * 100.0, "%")
            )
        }
    }
}
