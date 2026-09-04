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
    FitMode, Resample, ViewTransform, apply_orientation, downscale_to_display_with,
    fit_factor_to_budget, magnify_factor, magnify_nearest_crop, to_render_image,
};

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
}

struct Prepared {
    path: PathBuf,
    render: Arc<RenderImage>,
    intrinsic: (f32, f32),
    decoded: Arc<Decoded>,
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
        let factor = fit_factor_to_budget(crop, factor);
        if factor <= 1 {
            current.magnified = None;
            return;
        }
        if current.magnified.as_ref().is_some_and(|m| m.factor == factor && m.crop == crop) {
            return;
        }
        let enlarged = magnify_nearest_crop(base, crop, factor);
        if enlarged.width == 0 || enlarged.height == 0 {
            current.magnified = None;
            return;
        }
        let render = to_render_image(&Decoded::Still(enlarged));
        current.magnified = Some(Magnified { factor, crop, render });
    }

    pub fn set_viewport(&mut self, width: f32, height: f32) {
        self.viewport = (width, height);
        self.cache.set_target_size(width.max(1.0) as u32, height.max(1.0) as u32);
    }

    pub fn open(&mut self, path: &Path) -> std::io::Result<()> {
        match self.cache.block_on(path, 0) {
            Ok(_) => self.present(path),
            Err(e) => self.status = Some(format!("{}: {e}", path.display())),
        }
        self.directory = Directory::open_at(path)?;
        self.cache.sync_to_directory(&self.directory);
        self.cache.prefetch_neighbours(&self.directory);
        Ok(())
    }

    fn present(&mut self, path: &Path) {
        if let Some(ready) = self.prepared.remove(path)
            && ready.resample == self.resample
        {
            self.playback.reset();
            self.transform.apply_fit(ready.intrinsic, self.viewport);
            self.status = None;
            self.current = Some(ready);
            self.restore_playback(path);
            return;
        }
        let Some(entry) = self.cache.get(path) else {
            return;
        };
        let prepared = Self::prepare(
            path,
            &entry.output.decoded,
            entry.output.orientation,
            self.viewport,
            self.resample,
        );
        self.playback.reset();
        self.transform.apply_fit(prepared.intrinsic, self.viewport);
        self.status = None;
        self.current = Some(prepared);
        self.restore_playback(path);
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
            self.current.as_ref().map(|c| c.decoded.as_ref()),
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
        decoded: &Decoded,
        orientation: crate::decode::Orientation,
        viewport: (f32, f32),
        resample: Resample,
    ) -> Prepared {
        let first = match decoded {
            Decoded::Still(img) => Some(apply_orientation(img, orientation)),
            Decoded::Animation(frames) => {
                frames.first().map(|f| apply_orientation(&f.image, orientation))
            }
        };

        let (render, intrinsic, base) = match (decoded, first) {
            (Decoded::Still(_), Some(img)) => {
                let scaled = downscale_to_display_with(&img, viewport, resample);
                let intrinsic = (scaled.width as f32, scaled.height as f32);
                (to_render_image(&Decoded::Still(scaled.clone())), intrinsic, Some(scaled))
            }
            (_, Some(img)) => {
                (to_render_image(decoded), (img.width as f32, img.height as f32), None)
            }
            (_, None) => (to_render_image(decoded), (0.0, 0.0), None),
        };

        Prepared {
            path: path.to_path_buf(),
            render,
            intrinsic,
            decoded: Arc::new(decoded.clone()),
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
                &entry.output.decoded,
                entry.output.orientation,
                self.viewport,
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
        let entries: Vec<PathBuf> = self.directory.entries().to_vec();
        let mut rng = rand::rng();
        if let Some(path) = entries.choose(&mut rng).cloned() {
            self.directory.jump_to(&path);
            self.show(&path);
        }
    }

    fn show(&mut self, path: &Path) {
        let index = self.directory.current_index();
        match self.cache.block_on(path, index) {
            Ok(_) => self.present(path),
            Err(e) => {
                self.current = None;
                self.status = Some(format!("{}: {e}", path.display()));
            }
        }
        self.cache.prefetch_neighbours(&self.directory);
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

    pub fn set_fit(&mut self, fit: FitMode) {
        let intrinsic = self.current_intrinsic();
        self.transform.set_fit(fit, intrinsic, self.viewport);
    }

    pub fn pan(&mut self, delta: (f32, f32)) {
        self.transform.pan(delta);
    }

    pub fn zoom_at(&mut self, factor: f32, cursor: (f32, f32)) {
        self.transform.zoom_at(factor, cursor, self.viewport);
    }

    pub fn tick(&mut self, now: Instant) -> bool {
        let mut redraw = false;
        if !self.cache.pump(self.directory.current_index()).is_empty() {
            redraw = true;
        }
        self.prepare_neighbours();
        if let Some(current) = &self.current {
            let decoded = current.decoded.clone();
            if self.playback.advance(&decoded, now) {
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

    pub fn frame_index(&self) -> usize {
        self.playback.frame_index()
    }
}

impl Viewer {
    pub fn current_decoded(&self) -> Option<Arc<Decoded>> {
        self.current.as_ref().map(|c| c.decoded.clone())
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
