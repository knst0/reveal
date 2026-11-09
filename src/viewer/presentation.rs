use std::path::{Path, PathBuf};
use std::sync::Arc;

use gpui::RenderImage;

use super::view::ViewState;
use crate::cache::CachedImage;
use crate::decode::{DecodeOutput, Decoded, DecodedImage};
use crate::render::{
    FitMode, RenderConfig, Resample, fit_factor_to_budget, into_render_image_still, magnify_factor,
    magnify_nearest_crop, oriented, oriented_size, prepare_display, to_render_image,
};

fn scale_crop_to_base(
    crop: (u32, u32, u32, u32),
    intrinsic: (f32, f32),
    base: (u32, u32),
) -> (u32, u32, u32, u32) {
    if intrinsic.0 <= 0.0 || intrinsic.1 <= 0.0 || base.0 == 0 || base.1 == 0 {
        return crop;
    }
    let sx = base.0 as f32 / intrinsic.0;
    let sy = base.1 as f32 / intrinsic.1;
    let x = ((crop.0 as f32 * sx).floor() as u32).min(base.0.saturating_sub(1));
    let y = ((crop.1 as f32 * sy).floor() as u32).min(base.1.saturating_sub(1));
    let w = ((crop.2 as f32 * sx).ceil() as u32).max(1).min(base.0 - x);
    let h = ((crop.3 as f32 * sy).ceil() as u32).max(1).min(base.1 - y);
    (x, y, w, h)
}

/// Where the shown pixels came from. File-backed images support reload,
/// delete and position labels; pasted images do not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    File,
    Clipboard,
}

pub struct Prepared {
    pub path: PathBuf,
    pub render: Arc<RenderImage>,
    pub intrinsic: (f32, f32),
    pub source: (u32, u32),
    pub output: Arc<crate::decode::DecodeOutput>,
    pub origin: Origin,
    pub render_config: RenderConfig,
    base: Option<DecodedImage>,
    magnified: Option<Magnified>,
}

struct Magnified {
    factor: u32,
    crop: (u32, u32, u32, u32),
    render: Arc<RenderImage>,
}

#[derive(Default)]
pub struct Presentation {
    current: Option<Prepared>,
    prepared: std::collections::HashMap<PathBuf, Prepared>,
}

impl Presentation {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn current_path(&self) -> Option<&Path> {
        self.current.as_ref().map(|c| c.path.as_path())
    }

    /// The shown path only when it is a real file, not a pasted image.
    pub fn current_file(&self) -> Option<&Path> {
        self.current.as_ref().filter(|c| c.origin == Origin::File).map(|c| c.path.as_path())
    }

    pub fn current_origin(&self) -> Option<Origin> {
        self.current.as_ref().map(|c| c.origin)
    }

    pub fn has_current(&self) -> bool {
        self.current.is_some()
    }

    pub fn clear_current(&mut self) {
        self.current = None;
    }

    pub fn clear_prepared(&mut self) {
        self.prepared.clear();
    }

    pub fn drop_magnification(&mut self) {
        if let Some(current) = self.current.as_mut() {
            current.magnified = None;
        }
    }

    pub fn is_ready(&self, path: &Path, view: &ViewState) -> bool {
        self.prepared.get(path).is_some_and(|p| p.render_config == view.render_config())
    }

    pub fn current_output(&self) -> Option<Arc<crate::decode::DecodeOutput>> {
        self.current.as_ref().map(|c| c.output.clone())
    }

    pub fn current_intrinsic(&self) -> (f32, f32) {
        self.current.as_ref().map_or((0.0, 0.0), |c| c.intrinsic)
    }

    pub fn current_source_size(&self) -> (u32, u32) {
        self.current.as_ref().map_or((0, 0), |c| c.source)
    }

    pub fn is_animated(&self) -> bool {
        matches!(
            self.current.as_ref().map(|c| &c.output.decoded),
            Some(Decoded::Animation(frames)) if frames.len() > 1
        )
    }

    pub fn original_zoom(&self, view: &ViewState) -> f32 {
        let Some(current) = self.current.as_ref() else {
            return 1.0;
        };
        if current.intrinsic.0 <= 0.0 || current.source.0 == 0 {
            return 1.0;
        }
        current.source.0 as f32 / (current.intrinsic.0 * view.dpr())
    }

    pub fn render_image(&mut self, view: &ViewState) -> Option<Arc<RenderImage>> {
        self.sync_magnification(view);
        let current = self.current.as_ref()?;
        match &current.magnified {
            Some(m) => Some(m.render.clone()),
            None => Some(current.render.clone()),
        }
    }

    pub fn render_crop(&self) -> Option<(u32, u32, u32, u32)> {
        self.current.as_ref()?.magnified.as_ref().map(|m| m.crop)
    }

    pub fn prepare(
        path: &Path,
        output: Arc<DecodeOutput>,
        cached_display: Option<Arc<crate::render::Display>>,
        origin: Origin,
        view: &ViewState,
    ) -> Prepared {
        let config = view.render_config();
        let physical = (config.physical.0 as f32, config.physical.1 as f32);

        let (render, intrinsic, source) = match &output.decoded {
            Decoded::Still(img) => {
                let display = match cached_display.as_ref() {
                    Some(d)
                        if d.resample == config.resample
                            && d.fits(img, output.orientation, physical) =>
                    {
                        d.clone()
                    }
                    _ => Arc::new(prepare_display(
                        img,
                        output.orientation,
                        physical,
                        config.resample,
                    )),
                };
                let intrinsic =
                    (display.width as f32 / config.dpr, display.height as f32 / config.dpr);
                // The full-resolution `base` for nearest-neighbour magnification is
                // materialized lazily in `sync_magnification`: most frames are viewed at
                // zoom 1 where it would be a dead `width*height*4` copy.
                (display.render.clone(), intrinsic, display.source)
            }
            Decoded::Animation(frames) => match frames.first() {
                Some(f) => {
                    let size = oriented_size((f.image.width, f.image.height), output.orientation);
                    (to_render_image(&output.decoded), (size.0 as f32, size.1 as f32), size)
                }
                None => (to_render_image(&output.decoded), (0.0, 0.0), (0, 0)),
            },
        };

        Prepared {
            path: path.to_path_buf(),
            render,
            intrinsic,
            source,
            output,
            origin,
            render_config: config,
            base: None,
            magnified: None,
        }
    }

    pub fn present(
        &mut self,
        path: &Path,
        entry: Option<&CachedImage>,
        origin: Origin,
        view: &ViewState,
    ) -> bool {
        if let Some(ready) = self.prepared.remove(path)
            && ready.render_config == view.render_config()
        {
            self.current = Some(ready);
            return true;
        }
        let Some(entry) = entry else {
            return false;
        };
        self.current =
            Some(Self::prepare(path, entry.output.clone(), entry.display.clone(), origin, view));
        true
    }

    pub fn adopt(&mut self, prepared: Prepared) {
        self.current = Some(prepared);
    }

    pub fn stage(&mut self, path: PathBuf, prepared: Prepared) {
        self.prepared.insert(path, prepared);
    }

    pub fn retain_prepared(&mut self, keep: &[PathBuf]) {
        self.prepared.retain(|p, _| keep.contains(p));
    }

    pub fn unstage(&mut self, path: &Path) {
        self.prepared.remove(path);
    }

    /// True when every staged key is inside `keep`, without cloning the keys.
    pub fn staged_within(&self, keep: &[PathBuf]) -> bool {
        self.prepared.keys().all(|key| keep.contains(key))
    }

    pub fn reprepare_current(&mut self, view: &mut ViewState) {
        self.prepared.clear();
        self.drop_magnification();
        let Some((path, output, origin)) =
            self.current.as_ref().map(|c| (c.path.clone(), c.output.clone(), c.origin))
        else {
            return;
        };
        let offset = view.transform.offset;
        let zoom = view.transform.zoom;
        let fit = view.transform.fit;
        let previous = self.current_intrinsic();
        self.current = Some(Self::prepare(&path, output, None, origin, view));
        view.apply_fit(self.current_intrinsic(), self.original_zoom(view));
        if fit == FitMode::Free {
            let intrinsic = self.current_intrinsic();
            let ratio =
                if previous.0 > 0.0 && intrinsic.0 > 0.0 { intrinsic.0 / previous.0 } else { 1.0 };
            view.transform.fit = FitMode::Free;
            view.transform.zoom = zoom / ratio;
            view.transform.offset = offset;
        }
        view.transform.clamp_offset(self.current_intrinsic(), view.viewport);
    }

    fn sync_magnification(&mut self, view: &ViewState) {
        let zoom = view.transform.zoom * view.dpr();
        let transform = view.transform;
        let viewport = view.viewport;
        let resample = view.resample;
        let Some(current) = self.current.as_mut() else {
            return;
        };
        let factor = if resample == Resample::Nearest { magnify_factor(zoom) } else { 1 };
        if factor <= 1 {
            current.magnified = None;
            // Release the full-resolution copy when it is not magnifying anything.
            current.base = None;
            return;
        }
        if current.base.is_none() {
            let Decoded::Still(img) = &current.output.decoded else {
                current.magnified = None;
                return;
            };
            let orientation = current.output.orientation;
            current.base = Some(oriented(img, orientation).into_owned());
        }
        let Some(base) = current.base.as_ref() else {
            return;
        };
        let Some(crop) = transform.visible_source_rect(current.intrinsic, viewport) else {
            current.magnified = None;
            return;
        };
        let src_crop = scale_crop_to_base(crop, current.intrinsic, (base.width, base.height));
        let base_scale = (base.width as f32 / current.intrinsic.0.max(1.0)).max(1.0);
        let factor = ((factor as f32 / base_scale).ceil() as u32).max(1);
        let factor = fit_factor_to_budget(src_crop, factor);
        if factor <= 1 && base_scale <= 1.0 {
            current.magnified = None;
            return;
        }
        if current.magnified.as_ref().is_some_and(|m| m.factor == factor && m.crop == crop) {
            return;
        }
        let enlarged = magnify_nearest_crop(base, src_crop, factor);
        if enlarged.width == 0 || enlarged.height == 0 {
            current.magnified = None;
            return;
        }
        let render = into_render_image_still(enlarged);
        current.magnified = Some(Magnified { factor, crop, render });
    }
}
