mod jxl;
mod raster;
#[cfg(feature = "raw")]
mod raw;
mod svg;

use std::path::Path;
use std::time::Duration;

pub use raster::is_raster_extension;

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

#[derive(Debug, Clone)]
pub struct DecodeRequest<'a> {
    pub path: &'a Path,
    pub bytes: &'a [u8],
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
    fn name(&self) -> &'static str;
    fn probe(&self, req: &DecodeRequest<'_>) -> bool;
    fn decode(&self, req: &DecodeRequest<'_>) -> Result<Decoded, DecodeError>;
}

fn decoders() -> Vec<&'static dyn Decoder> {
    let mut list: Vec<&'static dyn Decoder> = vec![&svg::SvgDecoder, &jxl::JxlDecoder];
    #[cfg(feature = "raw")]
    list.push(&raw::RawDecoder);
    list.push(&raster::RasterDecoder);
    list
}

pub fn decode(req: &DecodeRequest<'_>) -> Result<DecodeOutput, DecodeError> {
    let list = decoders();
    let decoder = list.iter().find(|d| d.probe(req)).ok_or(DecodeError::NoDecoder)?;
    let decoded = decoder.decode(req)?;
    Ok(DecodeOutput { decoded, orientation: exif_orientation(req.bytes) })
}

pub fn extension_of(path: &Path) -> Option<String> {
    path.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase())
}

pub fn is_supported(path: &Path) -> bool {
    let Some(ext) = extension_of(path) else {
        return false;
    };
    if is_raster_extension(&ext) || svg::is_svg_extension(&ext) || jxl::is_jxl_extension(&ext) {
        return true;
    }
    #[cfg(feature = "raw")]
    if raw::is_raw_extension(&ext) {
        return true;
    }
    false
}

pub fn supported_extensions() -> Vec<&'static str> {
    let mut list: Vec<&'static str> = raster::RASTER_EXTENSIONS.to_vec();
    list.extend_from_slice(svg::SVG_EXTENSIONS);
    list.extend_from_slice(jxl::JXL_EXTENSIONS);
    #[cfg(feature = "raw")]
    list.extend_from_slice(raw::RAW_EXTENSIONS);
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
