use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use gpui::RenderImage;
use rand::seq::IndexedRandom;

use crate::cache::ImageCache;
use crate::decode::{Decoded, DecodedImage};
use crate::directory::{Directory, Navigation};
use crate::playback::{Playback, PlaybackState};
use crate::render::{
    FitMode, Resample, ViewTransform, fit_factor_to_budget, into_render_image_still,
    magnify_factor, magnify_nearest_crop, oriented, oriented_size, prepare_display,
    to_render_image,
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

pub struct Viewer {
    pub directory: Directory,
    pub cache: ImageCache,
    pub playback: Playback,
    pub transform: ViewTransform,
    pub viewport: (f32, f32),
    pub scale_factor: f32,
    resample: Resample,
    current: Option<Prepared>,
    status: Option<String>,
    prepared: std::collections::HashMap<PathBuf, Prepared>,
    paused_paths: std::collections::HashSet<PathBuf>,
    pending: Option<PathBuf>,
    scan: Option<crate::directory::PendingScan>,
}

struct Prepared {
    path: PathBuf,
    render: Arc<RenderImage>,
    intrinsic: (f32, f32),
    source: (u32, u32),
    output: Arc<crate::decode::DecodeOutput>,
    resample: Resample,
    base: Option<DecodedImage>,
    magnified: Option<Magnified>,
}

struct Magnified {
    factor: u32,
    crop: (u32, u32, u32, u32),
    render: Arc<RenderImage>,
}

impl Default for Viewer {
    fn default() -> Self {
        Self::new()
    }
}

impl Viewer {
    pub fn new() -> Self {
        Self {
            directory: Directory::empty(),
            cache: ImageCache::default(),
            playback: Playback::default(),
            transform: ViewTransform::default(),
            viewport: (0.0, 0.0),
            scale_factor: 1.0,
            resample: Resample::Filtered,
            current: None,
            status: None,
            prepared: std::collections::HashMap::new(),
            paused_paths: std::collections::HashSet::new(),
            pending: None,
            scan: None,
        }
    }

    pub fn status(&self) -> Option<&str> {
        self.status.as_deref()
    }

    pub fn current_path(&self) -> Option<&Path> {
        self.current.as_ref().map(|c| c.path.as_path())
    }

    pub fn render_image(&mut self) -> Option<Arc<RenderImage>> {
        self.sync_magnification();
        let current = self.current.as_ref()?;
        match &current.magnified {
            Some(m) => Some(m.render.clone()),
            None => Some(current.render.clone()),
        }
    }

    pub fn render_crop(&self) -> Option<(u32, u32, u32, u32)> {
        self.current.as_ref()?.magnified.as_ref().map(|m| m.crop)
    }

    fn sync_magnification(&mut self) {
        let zoom = self.transform.zoom * self.scale_factor.max(1.0);
        let transform = self.transform;
        let viewport = self.viewport;
        let resample = self.resample;
        let Some(current) = self.current.as_mut() else {
            return;
        };
        let factor = if resample == Resample::Nearest && current.base.is_some() {
            magnify_factor(zoom)
        } else {
            1
        };
        if factor <= 1 {
            current.magnified = None;
            return;
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

    pub fn set_viewport(&mut self, width: f32, height: f32) {
        self.viewport = (width, height);
        self.sync_target_size();
    }

    fn sync_target_size(&mut self) {
        let dpr = self.scale_factor.max(1.0);
        self.cache.set_target_size(
            (self.viewport.0 * dpr).max(1.0) as u32,
            (self.viewport.1 * dpr).max(1.0) as u32,
        );
    }

    pub fn set_scale_factor(&mut self, scale_factor: f32) {
        if (scale_factor - self.scale_factor).abs() < f32::EPSILON {
            return;
        }
        self.scale_factor = scale_factor;
        self.sync_target_size();
        self.prepared.clear();
        if let Some(path) = self.current_path().map(Path::to_path_buf) {
            let keep = self.transform;
            self.present(&path);
            self.transform = keep;
        }
    }

    pub fn current_source_size(&self) -> (u32, u32) {
        self.current.as_ref().map_or((0, 0), |c| c.source)
    }

    pub fn open(&mut self, path: &Path) -> std::io::Result<()> {
        self.cache.cancel_all_inflight();
        self.prepared.clear();
        self.pending = None;
        self.status = None;
        self.current = None;
        let scan = crate::directory::PendingScan::spawn(path);
        match scan.target() {
            Some(target) => {
                let target = target.to_path_buf();
                self.directory = Directory::single(&target);
                self.cache.sync_to_directory(&self.directory);
                self.show(&target);
            }
            None => {
                self.directory = Directory::empty();
                self.cache.sync_to_directory(&self.directory);
                self.cache.set_current_index(0);
            }
        }
        self.scan = Some(scan);
        Ok(())
    }

    pub fn wait_for_scan(&mut self) {
        self.adopt_scan(true);
    }

    fn adopt_scan(&mut self, blocking: bool) -> bool {
        let Some(scan) = self.scan.as_mut() else {
            return false;
        };
        let result = if blocking { Some(scan.wait()) } else { scan.take() };
        let Some(result) = result else {
            return false;
        };
        self.scan = None;
        let Ok(mut scanned) = result else {
            return false;
        };
        let anchor = self
            .current_path()
            .or(self.pending.as_deref())
            .or_else(|| self.directory.current())
            .map(Path::to_path_buf);
        if let Some(anchor) = anchor.as_deref() {
            scanned.jump_to(anchor);
        }
        self.directory = scanned;
        self.cache.sync_to_directory(&self.directory);
        let index = self.directory.current_index();
        self.cache.set_current_index(index);
        let mut redraw = false;
        match anchor {
            Some(_) => {
                if self.resolve_pending() {
                    redraw = true;
                }
            }
            None => {
                if let Some(first) = self.directory.current().map(Path::to_path_buf) {
                    self.show(&first);
                    redraw = true;
                }
            }
        }
        self.cache.prefetch_neighbours(&self.directory);
        redraw || self.current.is_some()
    }

    fn scan_pending(&self) -> bool {
        self.scan.is_some()
    }

    fn present(&mut self, path: &Path) {
        if let Some(ready) = self.prepared.remove(path)
            && ready.resample == self.resample
        {
            self.playback.reset();
            self.status = None;
            self.current = Some(ready);
            self.apply_current_fit();
            self.restore_playback(path);
            return;
        }
        let Some(entry) = self.cache.get(path) else {
            return;
        };
        let prepared = Self::prepare(
            path,
            entry.output.clone(),
            self.viewport,
            self.scale_factor,
            self.resample,
        );
        self.playback.reset();
        self.status = None;
        self.current = Some(prepared);
        self.apply_current_fit();
        self.restore_playback(path);
    }

    fn apply_current_fit(&mut self) {
        let intrinsic = self.current_intrinsic();
        let original = self.original_zoom();
        self.transform.apply_fit_with(intrinsic, self.viewport, original);
    }

    fn restore_playback(&mut self, path: &Path) {
        if matches!(self.playback.state, PlaybackState::Present | PlaybackState::PresentRandom) {
            return;
        }
        let state = if self.paused_paths.contains(path) {
            PlaybackState::Paused
        } else {
            PlaybackState::Playing
        };
        self.playback.set_state(state);
    }

    pub fn is_animated(&self) -> bool {
        matches!(
            self.current.as_ref().map(|c| &c.output.decoded),
            Some(Decoded::Animation(frames)) if frames.len() > 1
        )
    }

    pub fn toggle_play(&mut self) {
        self.playback.toggle_play();
        let Some(path) = self.current_path().map(Path::to_path_buf) else {
            return;
        };
        if self.playback.state == PlaybackState::Paused {
            self.paused_paths.insert(path);
        } else {
            self.paused_paths.remove(&path);
        }
    }

    fn prepare(
        path: &Path,
        output: Arc<crate::decode::DecodeOutput>,
        viewport: (f32, f32),
        scale_factor: f32,
        resample: Resample,
    ) -> Prepared {
        let dpr = scale_factor.max(1.0);
        let physical = (viewport.0 * dpr, viewport.1 * dpr);

        let (render, intrinsic, source, base) = match &output.decoded {
            Decoded::Still(img) => {
                let display = match output.display.as_ref() {
                    Some(d)
                        if d.resample == resample && d.fits(img, output.orientation, physical) =>
                    {
                        d.clone()
                    }
                    _ => Arc::new(prepare_display(img, output.orientation, physical, resample)),
                };
                let intrinsic = (display.width as f32 / dpr, display.height as f32 / dpr);
                let base = if resample == Resample::Nearest {
                    Some(oriented(img, output.orientation).into_owned())
                } else {
                    None
                };
                (display.render.clone(), intrinsic, display.source, base)
            }
            Decoded::Animation(frames) => match frames.first() {
                Some(f) => {
                    let size = oriented_size((f.image.width, f.image.height), output.orientation);
                    (to_render_image(&output.decoded), (size.0 as f32, size.1 as f32), size, None)
                }
                None => (to_render_image(&output.decoded), (0.0, 0.0), (0, 0), None),
            },
        };

        Prepared {
            path: path.to_path_buf(),
            render,
            intrinsic,
            source,
            output: output.clone(),
            resample,
            base,
            magnified: None,
        }
    }

    pub fn prepare_neighbours(&mut self) {
        for offset in [1isize, -1] {
            let Some(index) = self.directory.offset_index(offset) else {
                continue;
            };
            let Some(path) = self.directory.path_at(index).map(Path::to_path_buf) else {
                continue;
            };
            if self.prepared.get(&path).is_some_and(|p| p.resample == self.resample) {
                continue;
            }
            let Some(entry) = self.cache.get(&path) else {
                continue;
            };
            let prepared = Self::prepare(
                &path,
                entry.output.clone(),
                self.viewport,
                self.scale_factor,
                self.resample,
            );
            self.prepared.insert(path, prepared);
        }
        let keep: Vec<PathBuf> = [1isize, -1, 0]
            .iter()
            .filter_map(|o| self.directory.offset_index(*o))
            .filter_map(|i| self.directory.path_at(i).map(Path::to_path_buf))
            .collect();
        self.prepared.retain(|p, _| keep.contains(p));
    }
    pub fn current_intrinsic(&self) -> (f32, f32) {
        self.current.as_ref().map_or((0.0, 0.0), |c| c.intrinsic)
    }

    pub fn navigate(&mut self, nav: Navigation) {
        let Some(path) = self.directory.navigate(nav).map(Path::to_path_buf) else {
            return;
        };
        self.show(&path);
    }

    pub fn jump_random(&mut self) {
        let mut rng = rand::rng();
        let Some(path) = self.directory.entries().choose(&mut rng).cloned() else {
            return;
        };
        self.directory.jump_to(&path);
        self.show(&path);
    }

    fn show(&mut self, path: &Path) {
        let index = self.directory.current_index();
        self.cache.set_current_index(index);
        self.cache.cancel_outside_window(index);
        if self.prepared.contains_key(path) || self.cache.get(path).is_some() {
            self.pending = None;
            self.present(path);
        } else {
            self.pending = Some(path.to_path_buf());
            self.cache.request(path, index);
        }
        self.cache.prefetch_neighbours(&self.directory);
    }

    pub fn settle(&mut self) {
        self.wait_for_scan();
        while self.pending.is_some() {
            let index = self.directory.current_index();
            let Some((path, outcome)) = self.cache.drain_one(index) else {
                break;
            };
            self.absorb(&path, outcome);
            self.resolve_pending();
        }
    }

    fn absorb(&mut self, path: &Path, outcome: Result<(), crate::decode::DecodeError>) {
        if let Err(e) = outcome
            && self.pending.as_deref() == Some(path)
        {
            self.pending = None;
            self.current = None;
            self.status = Some(format!("{}: {e}", path.display()));
        }
    }

    fn resolve_pending(&mut self) -> bool {
        let Some(pending) = self.pending.clone() else {
            return false;
        };
        if self.directory.current().is_none_or(|c| c != pending) {
            self.pending = None;
            return false;
        }
        if self.prepared.contains_key(&pending) || self.cache.get(&pending).is_some() {
            self.pending = None;
            self.present(&pending);
            return true;
        }
        false
    }

    pub fn antialias(&self) -> bool {
        self.resample == Resample::Filtered
    }

    pub fn set_antialias(&mut self, antialias: bool) {
        let resample = Resample::from_antialias(antialias);
        if resample == self.resample {
            return;
        }
        self.resample = resample;
        self.cache.set_resample(resample);
        self.prepared.clear();
        if let Some(current) = self.current.as_mut() {
            current.magnified = None;
        }
        if let Some(path) = self.current_path().map(Path::to_path_buf) {
            let fit = self.transform;
            self.present(&path);
            self.transform = fit;
        }
    }

    pub fn toggle_antialias(&mut self) {
        self.set_antialias(!self.antialias());
    }

    pub fn show_pasted(&mut self, image: DecodedImage) {
        let output = Arc::new(crate::decode::DecodeOutput {
            decoded: Decoded::Still(image),
            orientation: crate::decode::Orientation::Normal,
            display: None,
        });
        let path = PathBuf::from("(clipboard)");
        let prepared =
            Self::prepare(&path, output, self.viewport, self.scale_factor, self.resample);
        self.pending = None;
        self.prepared.clear();
        self.playback.reset();
        self.status = None;
        self.transform.fit = FitMode::Fit;
        self.current = Some(prepared);
        self.apply_current_fit();
    }

    pub fn original_zoom(&self) -> f32 {
        let Some(current) = self.current.as_ref() else {
            return 1.0;
        };
        if current.intrinsic.0 <= 0.0 || current.source.0 == 0 {
            return 1.0;
        }
        current.source.0 as f32 / current.intrinsic.0
    }

    pub fn set_fit(&mut self, fit: FitMode) {
        let intrinsic = self.current_intrinsic();
        let original = self.original_zoom();
        self.transform.set_fit_with(fit, intrinsic, self.viewport, original);
    }

    pub fn pan(&mut self, delta: (f32, f32)) {
        self.transform.pan(delta);
    }

    pub fn zoom_at(&mut self, factor: f32, cursor: (f32, f32)) {
        self.transform.zoom_at(factor, cursor, self.viewport);
    }

    pub fn tick(&mut self, now: Instant) -> bool {
        let mut redraw = self.adopt_scan(false);
        for (path, outcome) in self.cache.pump(self.directory.current_index()) {
            redraw = true;
            self.absorb(&path, outcome);
        }
        if self.resolve_pending() {
            redraw = true;
        }
        self.prepare_neighbours();
        if let Some(current) = &self.current {
            let output = current.output.clone();
            if self.playback.advance(&output.decoded, now) {
                redraw = true;
            }
        }
        if self.playback.present_due(now) {
            match self.playback.state {
                PlaybackState::PresentRandom => self.jump_random(),
                _ => self.navigate(Navigation::Next),
            }
            redraw = true;
        }
        redraw
    }

    pub fn needs_ticking(&self) -> bool {
        if self.scan_pending() || self.pending.is_some() || self.cache.inflight_len() > 0 {
            return true;
        }
        if matches!(self.playback.state, PlaybackState::Present | PlaybackState::PresentRandom) {
            return true;
        }
        self.is_animated() && self.playback.state == PlaybackState::Playing
    }

    pub fn frame_index(&self) -> usize {
        self.playback.frame_index()
    }
}

impl Viewer {
    pub fn current_output(&self) -> Option<Arc<crate::decode::DecodeOutput>> {
        self.current.as_ref().map(|c| c.output.clone())
    }

    pub fn delete_current(&mut self) {
        let Some(path) = self.current_path().map(Path::to_path_buf) else {
            return;
        };
        if let Err(e) = crate::actions::move_to_trash(&path) {
            self.status = Some(format!("delete failed: {e}"));
            return;
        }
        self.cache.forget(&path);
        if self.directory.refresh().is_ok()
            && let Some(next) = self.directory.current().map(Path::to_path_buf)
        {
            self.show(&next);
        } else {
            self.current = None;
        }
    }
}
