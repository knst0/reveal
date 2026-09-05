use std::path::Path;
use std::sync::{Arc, OnceLock};

use resvg::tiny_skia;
use usvg::Transform;

use super::{DecodeError, DecodeRequest, Decoded, DecodedImage, Decoder, extension_of};

pub struct SvgDecoder;

static SYSTEM_FONTS: OnceLock<Arc<usvg::fontdb::Database>> = OnceLock::new();

fn system_fonts() -> Arc<usvg::fontdb::Database> {
    SYSTEM_FONTS
        .get_or_init(|| {
            let mut db = usvg::fontdb::Database::new();
            db.load_system_fonts();
            Arc::new(db)
        })
        .clone()
}

pub fn warm_font_database() {
    let _ = system_fonts();
}

pub const SVG_EXTENSIONS: &[&str] = &["svg", "svgz"];

pub fn is_svg_extension(ext: &str) -> bool {
    matches!(ext, "svg" | "svgz")
}

fn looks_like_svg(bytes: &[u8]) -> bool {
    let head_len = bytes.len().min(1024);
    let head = String::from_utf8_lossy(&bytes[..head_len]);
    head.contains("<svg")
}

fn is_gzip(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0x1f, 0x8b])
}

impl Decoder for SvgDecoder {
    fn name(&self) -> &'static str {
        "svg"
    }

    fn probe(&self, req: &DecodeRequest<'_>) -> bool {
        if looks_like_svg(req.bytes) || is_gzip(req.bytes) {
            return true;
        }
        extension_of(req.path).is_some_and(|e| is_svg_extension(&e))
    }

    fn decode(&self, req: &DecodeRequest<'_>) -> Result<Decoded, DecodeError> {
        let mut options = usvg::Options {
            resources_dir: req.path.parent().map(Path::to_path_buf),
            ..usvg::Options::default()
        };
        options.fontdb = system_fonts();
        let tree = usvg::Tree::from_data(req.bytes, &options)
            .map_err(|e| DecodeError::Decode(e.to_string()))?;

        let intrinsic = tree.size();
        if intrinsic.width() <= 0.0 || intrinsic.height() <= 0.0 {
            return Err(DecodeError::Decode("svg has zero size".into()));
        }

        let scale = if req.target_width == 0 || req.target_height == 0 {
            1.0
        } else {
            let sx = req.target_width as f32 / intrinsic.width();
            let sy = req.target_height as f32 / intrinsic.height();
            sx.min(sy).max(f32::MIN_POSITIVE)
        };

        let width = (intrinsic.width() * scale).round().max(1.0) as u32;
        let height = (intrinsic.height() * scale).round().max(1.0) as u32;

        let mut pixmap = tiny_skia::Pixmap::new(width, height)
            .ok_or_else(|| DecodeError::Decode("svg raster allocation failed".into()))?;
        resvg::render(&tree, Transform::from_scale(scale, scale), &mut pixmap.as_mut());

        Ok(Decoded::Still(DecodedImage { width, height, rgba: demultiply(pixmap) }))
    }
}

fn demultiply(pixmap: tiny_skia::Pixmap) -> Vec<u8> {
    let mut out = Vec::with_capacity(pixmap.width() as usize * pixmap.height() as usize * 4);
    for pixel in pixmap.pixels() {
        let c = pixel.demultiply();
        out.extend_from_slice(&[c.red(), c.green(), c.blue(), c.alpha()]);
    }
    out
}
