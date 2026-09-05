use std::io::Cursor;
use std::time::Duration;

use image::AnimationDecoder;
use image::ImageFormat;
use image::ImageReader;

use super::{DecodeError, DecodeRequest, Decoded, DecodedImage, Decoder, Frame, extension_of};

pub struct RasterDecoder;

pub const RASTER_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "webp", "bmp", "ico", "tif", "tiff", "tga", "ff", "qoi", "pnm",
    "pbm", "pgm", "ppm", "pam", "exr", "hdr",
];

pub fn is_raster_extension(ext: &str) -> bool {
    ImageFormat::from_extension(ext).is_some_and(|f| f.reading_enabled())
}

fn sniff(bytes: &[u8]) -> Option<ImageFormat> {
    image::guess_format(bytes).ok()
}

fn format_for(req: &DecodeRequest<'_>) -> Option<ImageFormat> {
    sniff(req.bytes).or_else(|| extension_of(req.path).and_then(ImageFormat::from_extension))
}

fn to_decoded_image(img: image::DynamicImage) -> DecodedImage {
    let rgba = img.to_rgba8();
    DecodedImage { width: rgba.width(), height: rgba.height(), rgba: rgba.into_raw() }
}

fn map_err(e: image::ImageError) -> DecodeError {
    DecodeError::Decode(e.to_string())
}

fn collect_frames(frames: image::Frames<'_>) -> Result<Vec<Frame>, DecodeError> {
    let mut out = Vec::new();
    for frame in frames {
        let frame = frame.map_err(map_err)?;
        let delay = Duration::from(frame.delay());
        let buffer = frame.into_buffer();
        out.push(Frame {
            image: DecodedImage {
                width: buffer.width(),
                height: buffer.height(),
                rgba: buffer.into_raw(),
            },
            delay,
        });
    }
    Ok(out)
}

fn decode_animated(format: ImageFormat, bytes: &[u8]) -> Option<Result<Vec<Frame>, DecodeError>> {
    let cursor = Cursor::new(bytes);
    match format {
        ImageFormat::Gif => Some(
            image::codecs::gif::GifDecoder::new(cursor)
                .map_err(map_err)
                .and_then(|d| collect_frames(d.into_frames())),
        ),
        ImageFormat::WebP => {
            Some(image::codecs::webp::WebPDecoder::new(cursor).map_err(map_err).and_then(|d| {
                if d.has_animation() { collect_frames(d.into_frames()) } else { Ok(Vec::new()) }
            }))
        }
        ImageFormat::Png => {
            Some(image::codecs::png::PngDecoder::new(cursor).map_err(map_err).and_then(|d| {
                if d.is_apng().map_err(map_err)? {
                    collect_frames(d.apng().map_err(map_err)?.into_frames())
                } else {
                    Ok(Vec::new())
                }
            }))
        }
        _ => None,
    }
}

impl Decoder for RasterDecoder {
    fn name(&self) -> &'static str {
        "raster"
    }

    fn probe(&self, req: &DecodeRequest<'_>) -> bool {
        format_for(req).is_some()
    }

    fn decode(&self, req: &DecodeRequest<'_>) -> Result<Decoded, DecodeError> {
        let format = format_for(req).ok_or(DecodeError::UnsupportedFormat)?;

        if let Some(frames) = decode_animated(format, req.bytes) {
            let frames = frames?;
            if frames.len() > 1 {
                return Ok(Decoded::Animation(frames));
            }
            if let Some(frame) = frames.into_iter().next() {
                return Ok(Decoded::Still(frame.image));
            }
        }

        let mut reader = ImageReader::new(Cursor::new(req.bytes));
        reader.set_format(format);
        let img = reader.decode().map_err(map_err)?;
        Ok(Decoded::Still(to_decoded_image(img)))
    }
}

#[cfg(test)]
mod tests {
    use super::{RASTER_EXTENSIONS, is_raster_extension};

    #[test]
    fn every_listed_raster_extension_is_actually_supported() {
        for ext in RASTER_EXTENSIONS {
            assert!(is_raster_extension(ext), "{ext} is listed but not decodable");
        }
    }
}
