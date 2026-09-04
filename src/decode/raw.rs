use rawler::decoders::RawDecodeParams;
use rawler::imgop::develop::RawDevelop;
use rawler::rawsource::RawSource;

use super::{DecodeError, DecodeRequest, Decoded, DecodedImage, Decoder, extension_of};

pub struct RawDecoder;

pub const RAW_EXTENSIONS: &[&str] = &[
    "cr2", "cr3", "crw", "nef", "nrw", "arw", "srf", "sr2", "orf", "rw2", "raf", "dng", "pef",
    "erf", "kdc", "dcr", "mrw", "3fr", "iiq", "mos", "srw", "x3f",
];

pub fn is_raw_extension(ext: &str) -> bool {
    RAW_EXTENSIONS.contains(&ext)
}

impl Decoder for RawDecoder {
    fn name(&self) -> &'static str {
        "raw"
    }

    fn probe(&self, req: &DecodeRequest<'_>) -> bool {
        extension_of(req.path).is_some_and(|e| is_raw_extension(&e))
            && rawler::get_decoder(&RawSource::new_from_slice(req.bytes)).is_ok()
    }

    fn decode(&self, req: &DecodeRequest<'_>) -> Result<Decoded, DecodeError> {
        let source = RawSource::new_from_slice(req.bytes);
        let decoder =
            rawler::get_decoder(&source).map_err(|e| DecodeError::Decode(e.to_string()))?;
        let raw = decoder
            .raw_image(&source, &RawDecodeParams::default(), false)
            .map_err(|e| DecodeError::Decode(e.to_string()))?;
        let developed = RawDevelop::default()
            .develop_intermediate(&raw)
            .map_err(|e| DecodeError::Decode(e.to_string()))?
            .to_dynamic_image()
            .ok_or_else(|| DecodeError::Decode("raw develop produced no image".into()))?;

        let rgba = developed.to_rgba8();
        Ok(Decoded::Still(DecodedImage {
            width: rgba.width(),
            height: rgba.height(),
            rgba: rgba.into_raw(),
        }))
    }
}
