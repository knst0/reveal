use zenpixels::descriptor::{ChannelLayout, ChannelType};

use super::{DecodeError, DecodeRequest, Decoded, DecodedImage, Decoder, Frame, extension_of};

pub struct AvifDecoder;

pub const AVIF_EXTENSIONS: &[&str] = &["avif", "avifs"];

pub fn is_avif_extension(ext: &str) -> bool {
    matches!(ext, "avif" | "avifs")
}

fn looks_like_avif(bytes: &[u8]) -> bool {
    bytes.len() >= 12 && &bytes[4..8] == b"ftyp" && matches!(&bytes[8..12], b"avif" | b"avis")
}

fn config() -> zenavif::DecoderConfig {
    zenavif::DecoderConfig::new().prefer_8bit(true)
}

fn to_rgba(pixels: &zenpixels::PixelBuffer) -> Result<DecodedImage, DecodeError> {
    let (width, height) = (pixels.width(), pixels.height());
    let descriptor = pixels.descriptor();
    let format = descriptor.pixel_format();

    if format.channel_type() != ChannelType::U8 {
        return Err(DecodeError::Decode("avif did not decode to 8-bit pixels".into()));
    }

    let bpp = descriptor.bytes_per_pixel();
    let source = pixels.copy_to_contiguous_bytes();
    let count = width as usize * height as usize;
    let mut rgba = Vec::with_capacity(count * 4);

    for i in 0..count {
        let px = &source[i * bpp..i * bpp + bpp];
        let [r, g, b, a] = match format.layout() {
            ChannelLayout::Rgb => [px[0], px[1], px[2], 255],
            ChannelLayout::Rgba => [px[0], px[1], px[2], px[3]],
            ChannelLayout::Bgra => [px[2], px[1], px[0], px[3]],
            ChannelLayout::Gray => [px[0], px[0], px[0], 255],
            ChannelLayout::GrayAlpha => [px[0], px[0], px[0], px[1]],
            _ => return Err(DecodeError::Decode("avif has an unsupported channel layout".into())),
        };
        rgba.extend_from_slice(&[r, g, b, a]);
    }

    Ok(DecodedImage { width, height, rgba })
}

impl Decoder for AvifDecoder {
    fn name(&self) -> &'static str {
        "avif"
    }

    fn probe(&self, req: &DecodeRequest<'_>) -> bool {
        looks_like_avif(req.bytes) || extension_of(req.path).is_some_and(|e| is_avif_extension(&e))
    }

    fn decode(&self, req: &DecodeRequest<'_>) -> Result<Decoded, DecodeError> {
        if let Ok(animation) =
            zenavif::decode_animation_with(req.bytes, &config(), &zenavif::Unstoppable)
            && animation.frames.len() > 1
        {
            let mut frames = Vec::with_capacity(animation.frames.len());
            for frame in &animation.frames {
                frames.push(Frame {
                    image: to_rgba(&frame.pixels)?,
                    delay: std::time::Duration::from_millis(u64::from(frame.duration_ms.max(1))),
                });
            }
            return Ok(Decoded::Animation(frames));
        }

        let pixels = zenavif::decode_with(req.bytes, &config(), &zenavif::Unstoppable)
            .map_err(|e| DecodeError::Decode(e.to_string()))?;
        Ok(Decoded::Still(to_rgba(&pixels)?))
    }
}

#[cfg(test)]
mod tests {
    use super::{AVIF_EXTENSIONS, is_avif_extension};

    #[test]
    fn every_listed_avif_extension_is_recognised() {
        for ext in AVIF_EXTENSIONS {
            assert!(is_avif_extension(ext), "{ext} is listed but not recognised");
        }
    }
}
