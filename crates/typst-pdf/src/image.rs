use std::collections::HashMap;

use ecow::eco_format;
use image::{DynamicImage, GenericImageView, LumaA, Pixel, Rgba};
use pdf_writer::{Chunk, Filter, Finish, Ref};
use typst_library::diag::{At, SourceResult, StrResult};
use typst_library::foundations::Smart;
use typst_library::visualize::{
    ColorSpace, ExchangeFormat, Image, ImageKind, ImageScaling, RasterFormat,
    RasterImage, SvgImage,
};
use typst_utils::Deferred;

use crate::{color, deflate, PdfChunk, WithGlobalRefs};

/// Embed all used images into the PDF.
#[typst_macros::time(name = "write images")]
pub fn write_images(
    context: &WithGlobalRefs,
) -> SourceResult<(PdfChunk, HashMap<Image, Ref>)> {
    let mut chunk = PdfChunk::new();
    let mut out = HashMap::new();
    context.resources.traverse(&mut |resources| {
        for (i, image) in resources.images.items().enumerate() {
            if out.contains_key(image) {
                continue;
            }

            let (handle, span) = resources.deferred_images.get(&i).unwrap();
            let encoded = handle.wait().as_ref().map_err(Clone::clone).at(*span)?;

            match encoded {
                EncodedImage::Raster {
                    data,
                    filter,
                    color_space,
                    bits_per_component,
                    width,
                    height,
                    compressed_icc,
                    alpha,
                    interpolate,
                } => {
                    let image_ref = chunk.alloc();
                    out.insert(image.clone(), image_ref);

                    let mut image = chunk.chunk.image_xobject(image_ref, data);
                    image.filter(*filter);
                    image.width(*width as i32);
                    image.height(*height as i32);
                    image.bits_per_component(i32::from(*bits_per_component));
                    image.interpolate(*interpolate);

                    let mut icc_ref = None;
                    let space = image.color_space();
                    if compressed_icc.is_some() {
                        let id = chunk.alloc.bump();
                        space.icc_based(id);
                        icc_ref = Some(id);
                    } else {
                        color::write(
                            *color_space,
                            space,
                            &context.globals.color_functions,
                        );
                    }

                    // Add a second gray-scale image containing the alpha values if
                    // this image has an alpha channel.
                    if let Some(alpha) = alpha {
                        let mask_ref = chunk.alloc.bump();
                        image.s_mask(mask_ref);
                        image.finish();

                        let mut mask = chunk.image_xobject(mask_ref, &alpha.data);
                        mask.filter(alpha.filter);
                        mask.width(*width as i32);
                        mask.height(*height as i32);
                        mask.color_space().device_gray();
                        mask.bits_per_component(i32::from(alpha.bits_per_component));
                        mask.interpolate(*interpolate);
                    } else {
                        image.finish();
                    }

                    if let (Some(compressed_icc), Some(icc_ref)) =
                        (compressed_icc, icc_ref)
                    {
                        let mut stream = chunk.icc_profile(icc_ref, compressed_icc);
                        stream.filter(Filter::FlateDecode);
                        match color_space {
                            ColorSpace::Srgb => {
                                stream.n(3);
                                stream.alternate().srgb();
                            }
                            ColorSpace::D65Gray => {
                                stream.n(1);
                                stream.alternate().d65_gray();
                            }
                            _ => unimplemented!(),
                        }
                    }
                }
                EncodedImage::Svg(svg_chunk, id) => {
                    let mut map = HashMap::new();
                    svg_chunk.renumber_into(&mut chunk.chunk, |old| {
                        *map.entry(old).or_insert_with(|| chunk.alloc.bump())
                    });
                    out.insert(image.clone(), map[id]);
                }
            }
        }

        Ok(())
    })?;

    Ok((chunk, out))
}

/// Creates a new PDF image from the given image.
///
/// Also starts the deferred encoding of the image.
#[comemo::memoize]
pub fn deferred_image(
    image: Image,
    pdfa: bool,
) -> (Deferred<StrResult<EncodedImage>>, Option<ColorSpace>) {
    let color_space = match image.kind() {
        ImageKind::Raster(raster) if raster.icc().is_none() => {
            Some(to_color_space(raster.dynamic().color()))
        }
        _ => None,
    };

    // PDF/A does not appear to allow interpolation.
    // See https://github.com/typst/typst/issues/2942.
    let interpolate = !pdfa && image.scaling() == Smart::Custom(ImageScaling::Smooth);

    let deferred = Deferred::new(move || match image.kind() {
        ImageKind::Raster(raster)
            if raster.format() == RasterFormat::Exchange(ExchangeFormat::Jpg) =>
        {
            Ok(encode_raster_jpeg(raster, interpolate))
        }
        ImageKind::Raster(raster) => Ok(encode_raster_flate(raster, interpolate)),
        ImageKind::Svg(svg) => {
            let (chunk, id) = encode_svg(svg, pdfa)
                .map_err(|err| eco_format!("failed to convert SVG to PDF: {err}"))?;
            Ok(EncodedImage::Svg(chunk, id))
        }
    });

    (deferred, color_space)
}

/// Include the source image's JPEG data without re-encoding.
fn encode_raster_jpeg(image: &RasterImage, interpolate: bool) -> EncodedImage {
    let dynamic = image.dynamic();

    let color_type = dynamic.color();
    let color_space = to_color_space(color_type);

    let bits_per_component = (image.source_color_type().bits_per_pixel()
        / u16::from(image.source_color_type().channel_count()))
        as u8;

    let compressed_icc = image.icc().map(|bytes| deflate(bytes.as_ref()));
    let alpha = encode_alpha(dynamic);

    EncodedImage::Raster {
        data: image.data().to_vec(),
        filter: Filter::DctDecode,
        color_space,
        bits_per_component,
        width: dynamic.width(),
        height: dynamic.height(),
        compressed_icc,
        alpha,
        interpolate,
    }
}

/// Encode an arbitrary raster image with a suitable filter.
#[typst_macros::time(name = "encode raster image flate")]
fn encode_raster_flate(image: &RasterImage, interpolate: bool) -> EncodedImage {
    let dynamic = image.dynamic();
    let color_space = to_color_space(dynamic.color());

    // Encode image data in big-endian. The alpha channel is excluded.
    // TODO: Encode flate streams with PNG-predictor?
    let (bits_per_component, data) = match dynamic {
        DynamicImage::ImageLuma8(buf) => (8, deflate(buf.as_raw())),
        DynamicImage::ImageLumaA8(_) => (8, deflate(dynamic.to_luma8().as_raw())),
        DynamicImage::ImageLuma16(buf) => {
            let encoded: Vec<u8> =
                buf.as_raw().iter().flat_map(|&c| c.to_be_bytes()).collect();
            (16, deflate(&encoded))
        }
        DynamicImage::ImageLumaA16(buf) => {
            let encoded: Vec<u8> =
                buf.pixels().flat_map(|&LumaA([l, _])| l.to_be_bytes()).collect();
            (16, deflate(&encoded))
        }
        DynamicImage::ImageRgb8(buf) => (8, deflate(buf.as_raw())),
        DynamicImage::ImageRgba8(_) => (8, deflate(dynamic.to_rgb8().as_raw())),
        DynamicImage::ImageRgb16(buf) => {
            let encoded: Vec<u8> =
                buf.as_raw().iter().flat_map(|&c| c.to_be_bytes()).collect();
            (16, deflate(&encoded))
        }
        DynamicImage::ImageRgba16(buf) => {
            let encoded: Vec<u8> = buf
                .pixels()
                .flat_map(|px| px.to_rgb().0)
                .flat_map(|c| c.to_be_bytes())
                .collect();
            (16, deflate(&encoded))
        }
        DynamicImage::ImageRgb32F(buf) => {
            let encoded: Vec<u8> =
                buf.as_raw().iter().flat_map(|&c| c.to_be_bytes()).collect();
            (32, deflate(&encoded))
        }
        DynamicImage::ImageRgba32F(buf) => {
            let encoded: Vec<u8> = buf
                .pixels()
                .flat_map(|px| px.to_rgb().0)
                .flat_map(|c| c.to_be_bytes())
                .collect();
            (32, deflate(&encoded))
        }
        // Anything else
        _ => (8, deflate(dynamic.to_rgb8().as_raw())),
    };

    let compressed_icc = image.icc().map(|bytes| deflate(bytes.as_ref()));
    let alpha = encode_alpha(dynamic);

    EncodedImage::Raster {
        data,
        filter: Filter::FlateDecode,
        color_space,
        bits_per_component,
        width: dynamic.width(),
        height: dynamic.height(),
        compressed_icc,
        alpha,
        interpolate,
    }
}

/// Matches an [`image::ColorType`] to [`ColorSpace`].
fn to_color_space(color: image::ColorType) -> ColorSpace {
    use image::ColorType::*;
    match color {
        L8 | La8 | L16 | La16 => ColorSpace::D65Gray,
        Rgb8 | Rgba8 | Rgb16 | Rgba16 | Rgb32F | Rgba32F => ColorSpace::Srgb,
        _ => unimplemented!(),
    }
}

/// Encode an image's alpha channel if present.
#[typst_macros::time(name = "encode alpha")]
fn encode_alpha(image: &DynamicImage) -> Option<AlphaChannel> {
    if !image.color().has_alpha() {
        return None;
    }

    let bits_per_component = match image.color() {
        image::ColorType::La8 => 8,
        image::ColorType::Rgba8 => 8,
        image::ColorType::La16 => 16,
        image::ColorType::Rgba16 => 16,
        image::ColorType::Rgba32F => 32,
        _ => 8,
    };

    // Encode the alpha channel as big-endian.
    let alpha: Vec<u8> = match image {
        DynamicImage::ImageLumaA8(buf) => buf.pixels().map(|&LumaA([_, a])| a).collect(),
        DynamicImage::ImageLumaA16(buf) => {
            buf.pixels().flat_map(|&LumaA([_, a])| a.to_be_bytes()).collect()
        }
        DynamicImage::ImageRgba16(buf) => {
            buf.pixels().flat_map(|&Rgba([_, _, _, a])| a.to_be_bytes()).collect()
        }
        DynamicImage::ImageRgba32F(buf) => {
            buf.pixels().flat_map(|&Rgba([_, _, _, a])| a.to_be_bytes()).collect()
        }
        // Everything else is encoded as RGBA8.
        _ => image.pixels().map(|(_, _, Rgba([_, _, _, a]))| a).collect(),
    };
    Some(AlphaChannel {
        data: deflate(&alpha),
        filter: Filter::FlateDecode,
        bits_per_component,
    })
}

/// Encode an SVG into a chunk of PDF objects.
#[typst_macros::time(name = "encode svg")]
fn encode_svg(
    svg: &SvgImage,
    pdfa: bool,
) -> Result<(Chunk, Ref), svg2pdf::ConversionError> {
    svg2pdf::to_chunk(
        svg.tree(),
        svg2pdf::ConversionOptions { pdfa, ..Default::default() },
    )
}

/// A pre-encoded image.
pub enum EncodedImage {
    /// A pre-encoded rasterized image.
    Raster {
        /// The raw, pre-deflated image data.
        data: Vec<u8>,
        /// The filter to use for the image.
        filter: Filter,
        /// Which color space this image is encoded in.
        color_space: ColorSpace,
        /// How many bits of each color component are stored.
        bits_per_component: u8,
        /// The image's width.
        width: u32,
        /// The image's height.
        height: u32,
        /// The image's ICC profile, deflated, if any.
        compressed_icc: Option<Vec<u8>>,
        /// The alpha channel of the image, pre-deflated, if any.
        alpha: Option<AlphaChannel>,
        /// Whether image interpolation should be enabled.
        interpolate: bool,
    },
    /// A vector graphic.
    ///
    /// The chunk is the SVG converted to PDF objects.
    Svg(Chunk, Ref),
}

/// The alpha channel data.
pub struct AlphaChannel {
    /// The raw alpha channel, encoded using the given filter.
    data: Vec<u8>,
    /// The filter used for the alpha channel.
    filter: Filter,
    /// The number of bits the alpha component is encoded in.
    bits_per_component: u8,
}
