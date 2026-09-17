use rawler::decoders::RawDecodeParams;
use rawler::imgop::develop::RawDevelop;
use rawler::rawsource::RawSource;

use super::{DecodeError, DecodeRequest, Decoded, DecodedImage, Decoder, has_extension};

pub struct RawDecoder;

pub const RAW_EXTENSIONS: &[&str] = &[
    "cr2", "cr3", "crw", "nef", "nrw", "arw", "srf", "sr2", "orf", "rw2", "raf", "dng", "pef",
    "erf", "kdc", "dcr", "mrw", "3fr", "iiq", "mos", "srw", "x3f",
];

pub fn is_raw_extension(ext: &str) -> bool {
    RAW_EXTENSIONS.iter().any(|want| ext.eq_ignore_ascii_case(want))
}

fn to_decoded(image: image::DynamicImage) -> DecodedImage {
    let rgba = image.into_rgba8();
    DecodedImage { width: rgba.width(), height: rgba.height(), rgba: rgba.into_raw() }
}

impl Decoder for RawDecoder {
    fn name(&self) -> &'static str {
        "raw"
    }

    fn probe(&self, req: &DecodeRequest<'_>) -> bool {
        has_extension(req.path, RAW_EXTENSIONS)
            && rawler::get_decoder(&RawSource::new_from_slice(req.bytes)).is_ok()
    }

    fn decode(&self, req: &DecodeRequest<'_>) -> Result<Decoded, DecodeError> {
        let source = RawSource::new_from_slice(req.bytes);
        let decoder =
            rawler::get_decoder(&source).map_err(|e| DecodeError::Decode(e.to_string()))?;
        let params = RawDecodeParams::default();

        if req.target_width > 0
            && req.target_height > 0
            && let Ok(Some(preview)) = decoder
                .full_image(&source, &params)
                .or_else(|_| decoder.preview_image(&source, &params))
            && preview.width() >= req.target_width
            && preview.height() >= req.target_height
        {
            return Ok(Decoded::Still(to_decoded(preview)));
        }

        let raw = decoder
            .raw_image(&source, &params, false)
            .map_err(|e| DecodeError::Decode(e.to_string()))?;
        let developed = RawDevelop::default()
            .develop_intermediate(&raw)
            .map_err(|e| DecodeError::Decode(e.to_string()))?
            .to_dynamic_image()
            .ok_or_else(|| DecodeError::Decode("raw develop produced no image".into()))?;

        Ok(Decoded::Still(to_decoded(developed)))
    }
}
