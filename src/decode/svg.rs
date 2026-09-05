use std::path::Path;
use std::sync::{Arc, OnceLock};

use resvg::tiny_skia;
use usvg::Transform;

use super::{DecodeError, DecodeRequest, Decoded, DecodedImage, Decoder, extension_of};

pub struct SvgDecoder;

static SYSTEM_FONTS: OnceLock<Arc<FontSetup>> = OnceLock::new();

struct FontSetup {
    db: Arc<usvg::fontdb::Database>,
    default_family: Option<String>,
}

#[derive(Default)]
struct GenericFamilies {
    serif: Option<String>,
    sans_serif: Option<String>,
    cursive: Option<String>,
    fantasy: Option<String>,
    monospace: Option<String>,
}

fn first_present(db: &usvg::fontdb::Database, candidates: &[&str]) -> Option<String> {
    let has = |name: &str| {
        db.faces()
            .any(|face| face.families.iter().any(|(family, _)| family.eq_ignore_ascii_case(name)))
    };
    candidates.iter().find(|name| has(name)).map(|name| (*name).to_string())
}

fn any_family(db: &usvg::fontdb::Database, monospace: bool) -> Option<String> {
    db.faces()
        .find(|face| face.monospaced == monospace)
        .or_else(|| db.faces().next())
        .and_then(|face| face.families.first().map(|(family, _)| family.clone()))
}

fn resolve_generic_families(db: &usvg::fontdb::Database) -> GenericFamilies {
    let fallback = any_family(db, false);
    let mono_fallback = any_family(db, true).or_else(|| fallback.clone());
    GenericFamilies {
        serif: first_present(
            db,
            &["Times New Roman", "Liberation Serif", "DejaVu Serif", "Noto Serif", "Georgia"],
        )
        .or_else(|| fallback.clone()),
        sans_serif: first_present(
            db,
            &["Arial", "Helvetica", "Liberation Sans", "DejaVu Sans", "Noto Sans", "Ubuntu"],
        )
        .or_else(|| fallback.clone()),
        cursive: first_present(db, &["Comic Sans MS", "Comic Neue", "URW Chancery L"])
            .or_else(|| fallback.clone()),
        fantasy: first_present(db, &["Impact", "Papyrus", "Ubuntu"]).or_else(|| fallback.clone()),
        monospace: first_present(
            db,
            &[
                "Courier New",
                "Liberation Mono",
                "DejaVu Sans Mono",
                "Noto Sans Mono",
                "Ubuntu Mono",
            ],
        )
        .or(mono_fallback),
    }
}

fn font_setup() -> Arc<FontSetup> {
    SYSTEM_FONTS
        .get_or_init(|| {
            let mut db = usvg::fontdb::Database::new();
            db.load_system_fonts();
            let families = resolve_generic_families(&db);
            if let Some(name) = families.serif.clone() {
                db.set_serif_family(name);
            }
            if let Some(name) = families.sans_serif.clone() {
                db.set_sans_serif_family(name);
            }
            if let Some(name) = families.cursive.clone() {
                db.set_cursive_family(name);
            }
            if let Some(name) = families.fantasy.clone() {
                db.set_fantasy_family(name);
            }
            if let Some(name) = families.monospace.clone() {
                db.set_monospace_family(name);
            }
            Arc::new(FontSetup { db: Arc::new(db), default_family: families.sans_serif })
        })
        .clone()
}

fn apply_font_setup(options: &mut usvg::Options<'_>) {
    let setup = font_setup();
    options.fontdb = setup.db.clone();
    if let Some(name) = setup.default_family.clone() {
        options.font_family = name;
    }
}

pub fn warm_font_database() {
    let _ = font_setup();
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
        apply_font_setup(&mut options);
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
        let scale = zoom_headroom(scale, intrinsic.width(), intrinsic.height());

        let width = (intrinsic.width() * scale).round().max(1.0) as u32;
        let height = (intrinsic.height() * scale).round().max(1.0) as u32;

        let mut pixmap = tiny_skia::Pixmap::new(width, height)
            .ok_or_else(|| DecodeError::Decode("svg raster allocation failed".into()))?;
        resvg::render(&tree, Transform::from_scale(scale, scale), &mut pixmap.as_mut());

        Ok(Decoded::Still(DecodedImage { width, height, rgba: demultiply(pixmap) }))
    }
}

const ZOOM_HEADROOM: f32 = 2.0;
const MAX_RASTER_PIXELS: f32 = 96.0 * 1024.0 * 1024.0;

fn zoom_headroom(scale: f32, width: f32, height: f32) -> f32 {
    let wanted = scale * ZOOM_HEADROOM;
    let pixels = (width * wanted) * (height * wanted);
    if pixels <= MAX_RASTER_PIXELS {
        return wanted;
    }
    let budgeted = (MAX_RASTER_PIXELS / (width * height)).sqrt();
    budgeted.max(scale)
}

fn demultiply(pixmap: tiny_skia::Pixmap) -> Vec<u8> {
    let mut out = Vec::with_capacity(pixmap.width() as usize * pixmap.height() as usize * 4);
    for pixel in pixmap.pixels() {
        let c = pixel.demultiply();
        out.extend_from_slice(&[c.red(), c.green(), c.blue(), c.alpha()]);
    }
    out
}
