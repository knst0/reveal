mod avif;
mod jxl;
mod raster;
#[cfg(feature = "raw")]
mod raw;
mod svg;

use std::path::Path;
use std::time::Duration;

pub use raster::MAX_ANIMATION_BYTES;
pub use svg::warm_font_database;

#[derive(Debug, Clone)]
pub struct DecodedImage {
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone)]
pub struct Frame {
    pub image: DecodedImage,
    pub delay: Duration,
}

#[derive(Debug, Clone)]
pub enum Decoded {
    Still(DecodedImage),
    Animation(Vec<Frame>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    Normal,
    FlipH,
    Rotate180,
    FlipV,
    Transpose,
    Rotate90,
    Transverse,
    Rotate270,
}

impl Orientation {
    /// Rotations and transposes by 90 degrees exchange width and height.
    pub fn swaps_axes(self) -> bool {
        matches!(
            self,
            Orientation::Rotate90
                | Orientation::Rotate270
                | Orientation::Transpose
                | Orientation::Transverse
        )
    }
}

#[derive(Debug, Clone)]
pub struct DecodeRequest<'a> {
    pub path: &'a Path,
    pub bytes: &'a [u8],
    /// Preferred raster size for decoders that can render at a resolution
    /// (SVG, raw previews). Decoding itself never resamples raster images.
    pub target_width: u32,
    pub target_height: u32,
}

#[derive(Debug, Clone)]
pub struct DecodeOutput {
    pub decoded: Decoded,
    pub orientation: Orientation,
}

#[derive(Debug, thiserror::Error)]
pub enum DecodeError {
    #[error("unsupported image format")]
    UnsupportedFormat,
    #[error("no decoder matched this file")]
    NoDecoder,
    #[error("{0}")]
    Decode(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub trait Decoder: Send + Sync {
    /// Extensions this decoder accepts. This list drives directory filtering
    /// and the open dialog; `formats` adds MIME/description metadata per type.
    fn extensions(&self) -> &'static [&'static str];
    fn probe(&self, req: &DecodeRequest<'_>) -> bool;
    fn decode(&self, req: &DecodeRequest<'_>) -> Result<Decoded, DecodeError>;

    fn orientation(&self, _req: &DecodeRequest<'_>) -> Option<Orientation> {
        None
    }
}

fn decoders() -> &'static [&'static dyn Decoder] {
    #[cfg(feature = "raw")]
    {
        &[
            &svg::SvgDecoder,
            &jxl::JxlDecoder,
            &avif::AvifDecoder,
            &raw::RawDecoder,
            &raster::RasterDecoder,
        ]
    }
    #[cfg(not(feature = "raw"))]
    {
        &[&svg::SvgDecoder, &jxl::JxlDecoder, &avif::AvifDecoder, &raster::RasterDecoder]
    }
}

/// Case-insensitive extension match without allocating a lowered `String`.
/// Hot paths (`is_supported` per directory entry, decoder `probe` per load)
/// call this instead of allocating an extension string.
pub fn has_extension(path: &Path, allowed: &[&str]) -> bool {
    let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
        return false;
    };
    allowed.iter().any(|want| ext.eq_ignore_ascii_case(want))
}

pub fn decode(req: &DecodeRequest<'_>) -> Result<DecodeOutput, DecodeError> {
    let decoder =
        decoders().iter().copied().find(|d| d.probe(req)).ok_or(DecodeError::NoDecoder)?;
    let decoded = decoder.decode(req)?;
    let orientation = decoder.orientation(req).unwrap_or_else(|| exif_orientation(req.bytes));
    Ok(DecodeOutput { decoded, orientation })
}

pub fn is_supported(path: &Path) -> bool {
    decoders().iter().any(|decoder| has_extension(path, decoder.extensions()))
}

pub fn supported_extensions() -> Vec<&'static str> {
    let mut list: Vec<&'static str> =
        decoders().iter().flat_map(|decoder| decoder.extensions()).copied().collect();
    list.sort_unstable();
    list.dedup();
    list
}

pub fn exif_orientation(bytes: &[u8]) -> Orientation {
    let mut cursor = std::io::Cursor::new(bytes);
    let reader = exif::Reader::new();
    let Ok(exif) = reader.read_from_container(&mut cursor) else {
        return Orientation::Normal;
    };
    let Some(field) = exif.get_field(exif::Tag::Orientation, exif::In::PRIMARY) else {
        return Orientation::Normal;
    };
    match field.value.get_uint(0) {
        Some(2) => Orientation::FlipH,
        Some(3) => Orientation::Rotate180,
        Some(4) => Orientation::FlipV,
        Some(5) => Orientation::Transpose,
        Some(6) => Orientation::Rotate90,
        Some(7) => Orientation::Transverse,
        Some(8) => Orientation::Rotate270,
        _ => Orientation::Normal,
    }
}
