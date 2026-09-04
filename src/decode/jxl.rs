use std::time::Duration;

use jxl_oxide::JxlImage;

use super::{DecodeError, DecodeRequest, Decoded, DecodedImage, Decoder, Frame, extension_of};

pub struct JxlDecoder;

const CODESTREAM_SIGNATURE: [u8; 2] = [0xff, 0x0a];
const CONTAINER_SIGNATURE: [u8; 12] =
    [0x00, 0x00, 0x00, 0x0c, b'J', b'X', b'L', b' ', 0x0d, 0x0a, 0x87, 0x0a];

pub fn is_jxl_extension(ext: &str) -> bool {
    ext == "jxl"
}

fn looks_like_jxl(bytes: &[u8]) -> bool {
    bytes.starts_with(&CODESTREAM_SIGNATURE) || bytes.starts_with(&CONTAINER_SIGNATURE)
}

fn to_rgba8(samples: &[f32], channels: usize, width: u32, height: u32) -> Vec<u8> {
    let px = (width as usize) * (height as usize);
    let mut out = vec![0u8; px * 4];
    for i in 0..px {
        let src = &samples[i * channels..];
        let (r, g, b, a) = match channels {
            1 => (src[0], src[0], src[0], 1.0),
            2 => (src[0], src[0], src[0], src[1]),
            3 => (src[0], src[1], src[2], 1.0),
            _ => (src[0], src[1], src[2], src[3]),
        };
        let dst = &mut out[i * 4..i * 4 + 4];
        for (slot, value) in dst.iter_mut().zip([r, g, b, a]) {
            *slot = (value.clamp(0.0, 1.0) * 255.0).round() as u8;
        }
    }
    out
}

impl Decoder for JxlDecoder {
    fn name(&self) -> &'static str {
        "jxl"
    }

    fn probe(&self, req: &DecodeRequest<'_>) -> bool {
        looks_like_jxl(req.bytes) || extension_of(req.path).is_some_and(|e| is_jxl_extension(&e))
    }

    fn decode(&self, req: &DecodeRequest<'_>) -> Result<Decoded, DecodeError> {
        let image =
            JxlImage::builder().read(req.bytes).map_err(|e| DecodeError::Decode(e.to_string()))?;

        let width = image.width();
        let height = image.height();
        let seconds_per_tick = image
            .image_header()
            .metadata
            .animation
            .as_ref()
            .filter(|a| a.tps_numerator != 0)
            .map(|a| a.tps_denominator as f64 / a.tps_numerator as f64);

        let keyframes = image.num_loaded_keyframes().max(1);
        let mut frames = Vec::with_capacity(keyframes);
        for index in 0..keyframes {
            let render =
                image.render_frame(index).map_err(|e| DecodeError::Decode(e.to_string()))?;
            let fb = render.image_all_channels();
            let image = DecodedImage {
                width,
                height,
                rgba: to_rgba8(fb.buf(), fb.channels(), width, height),
            };
            let delay = seconds_per_tick
                .map(|spt| Duration::from_secs_f64(spt * render.duration() as f64))
                .unwrap_or_default();
            frames.push(Frame { image, delay });
        }

        if seconds_per_tick.is_some() && frames.len() > 1 {
            return Ok(Decoded::Animation(frames));
        }
        frames
            .into_iter()
            .next()
            .map(|f| Decoded::Still(f.image))
            .ok_or_else(|| DecodeError::Decode("jxl produced no frames".into()))
    }
}
