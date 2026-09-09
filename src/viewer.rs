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

pub struct ViewState {
    pub transform: ViewTransform,
    pub viewport: (f32, f32),
    pub scale_factor: f32,
    pub resample: Resample,
}

impl Default for ViewState {
    fn default() -> Self {
        Self {
            transform: ViewTransform::default(),
            viewport: (0.0, 0.0),
            scale_factor: 1.0,
            resample: Resample::Filtered,
        }
    }
}

impl ViewState {
    pub fn dpr(&self) -> f32 {
        self.scale_factor.max(1.0)
    }

    pub fn physical_viewport(&self) -> (f32, f32) {
        let dpr = self.dpr();
        (self.viewport.0 * dpr, self.viewport.1 * dpr)
    }

    pub fn antialias(&self) -> bool {
        self.resample == Resample::Filtered
    }

    pub fn apply_fit(&mut self, intrinsic: (f32, f32), original_zoom: f32) {
        self.transform.apply_fit_with(intrinsic, self.viewport, original_zoom);
    }

    pub fn set_fit(&mut self, fit: FitMode, intrinsic: (f32, f32), original_zoom: f32) {
        self.transform.set_fit_with(fit, intrinsic, self.viewport, original_zoom);
    }

    pub fn pan(&mut self, delta: (f32, f32)) {
        self.transform.pan(delta);
    }

    pub fn zoom_at(&mut self, factor: f32, cursor: (f32, f32)) {
        self.transform.zoom_at(factor, cursor, self.viewport);
    }
}

pub struct Prepared {
    pub path: PathBuf,
    pub render: Arc<RenderImage>,
    pub intrinsic: (f32, f32),
    pub source: (u32, u32),
    pub output: Arc<crate::decode::DecodeOutput>,
    pub resample: Resample,
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

    pub fn current(&self) -> Option<&Prepared> {
        self.current.as_ref()
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
        self.prepared.get(path).is_some_and(|p| p.resample == view.resample)
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
        output: Arc<crate::decode::DecodeOutput>,
        view: &ViewState,
    ) -> Prepared {
        let dpr = view.dpr();
        let physical = view.physical_viewport();
        let resample = view.resample;

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
            output,
            resample,
            base,
            magnified: None,
        }
    }

    pub fn present(
        &mut self,
        path: &Path,
        output: Option<Arc<crate::decode::DecodeOutput>>,
        view: &ViewState,
    ) -> bool {
        if let Some(ready) = self.prepared.remove(path)
            && ready.resample == view.resample
        {
            self.current = Some(ready);
            return true;
        }
        let Some(output) = output else {
            return false;
        };
        self.current = Some(Self::prepare(path, output, view));
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

    pub fn staged_paths(&self) -> Vec<PathBuf> {
        self.prepared.keys().cloned().collect()
    }

    pub fn reprepare_current(&mut self, view: &mut ViewState) {
        self.prepared.clear();
        self.drop_magnification();
        let Some((path, output)) =
            self.current.as_ref().map(|c| (c.path.clone(), c.output.clone()))
        else {
            return;
        };
        let offset = view.transform.offset;
        let zoom = view.transform.zoom;
        let fit = view.transform.fit;
        let previous = self.current_intrinsic();
        self.current = Some(Self::prepare(&path, output, view));
        view.apply_fit(self.current_intrinsic(), self.original_zoom(view));
        if fit == FitMode::Free {
            let intrinsic = self.current_intrinsic();
            let ratio =
                if previous.0 > 0.0 && intrinsic.0 > 0.0 { intrinsic.0 / previous.0 } else { 1.0 };
            view.transform.fit = FitMode::Free;
            view.transform.zoom = zoom / ratio;
            view.transform.offset = offset;
        }
    }

    fn sync_magnification(&mut self, view: &ViewState) {
        let zoom = view.transform.zoom * view.dpr();
        let transform = view.transform;
        let viewport = view.viewport;
        let resample = view.resample;
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
}

pub enum SessionEvent {
    Loaded(PathBuf),
    Failed(PathBuf, crate::decode::DecodeError),
    ScanComplete,
}

pub struct ScanOutcome {
    pub anchored: bool,
    pub realigned: Option<PathBuf>,
    pub first: Option<PathBuf>,
}

#[derive(Default)]
pub struct Session {
    pub directory: Directory,
    pub cache: ImageCache,
    status: Option<String>,
    pending: Option<PathBuf>,
    scan_generation: u64,
    scan: Option<crate::directory::PendingScan>,
}

impl Session {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn status(&self) -> Option<&str> {
        self.status.as_deref()
    }

    pub fn set_status(&mut self, status: Option<String>) {
        self.status = status;
    }

    pub fn pending(&self) -> Option<&Path> {
        self.pending.as_deref()
    }

    pub fn set_pending(&mut self, pending: Option<PathBuf>) {
        self.pending = pending;
    }

    pub fn scan_generation(&self) -> u64 {
        self.scan_generation
    }

    pub fn scan_pending(&self) -> bool {
        self.scan.is_some()
    }

    pub fn scan_receiver(
        &self,
    ) -> Option<(crossbeam_channel::Receiver<std::io::Result<Directory>>, u64)> {
        self.scan.as_ref().map(|scan| (scan.results(), self.scan_generation))
    }

    pub fn current_index(&self) -> usize {
        self.directory.current_index()
    }

    pub fn begin_scan(&mut self, path: &Path) -> Option<PathBuf> {
        self.cache.cancel_all_inflight();
        self.pending = None;
        self.status = None;
        self.scan_generation = self.scan_generation.wrapping_add(1);
        let scan = crate::directory::PendingScan::spawn(path);
        let target = scan.target().map(Path::to_path_buf);
        match target.as_deref() {
            Some(target) => {
                self.directory = Directory::single(target);
                self.cache.sync_to_directory(&self.directory);
            }
            None => {
                self.directory = Directory::empty();
                self.cache.sync_to_directory(&self.directory);
                self.cache.set_current_index(0);
            }
        }
        self.scan = Some(scan);
        target
    }

    pub fn take_scan(&mut self, blocking: bool) -> Option<std::io::Result<Directory>> {
        let scan = self.scan.as_mut()?;
        let result = if blocking { Some(scan.wait()) } else { scan.take() };
        if result.is_some() {
            self.scan = None;
        }
        result
    }

    pub fn discard_scan(&mut self, generation: u64) -> bool {
        if generation != self.scan_generation {
            return false;
        }
        self.scan = None;
        true
    }

    pub fn request(&mut self, path: &Path) {
        let index = self.directory.current_index();
        self.cache.set_current_index(index);
        self.pending = Some(path.to_path_buf());
        self.cache.request(path, index);
    }

    pub fn focus(&mut self) {
        let index = self.directory.current_index();
        self.cache.set_current_index(index);
        self.cache.cancel_outside_window(index);
    }

    pub fn prefetch(&mut self) {
        self.cache.prefetch_neighbours(&self.directory);
    }

    pub fn output(&self, path: &Path) -> Option<Arc<crate::decode::DecodeOutput>> {
        self.cache.get(path).map(|entry| entry.output.clone())
    }

    pub fn is_cached(&self, path: &Path) -> bool {
        self.cache.get(path).is_some()
    }

    pub fn neighbour_paths(&self) -> Vec<PathBuf> {
        self.paths_at(&[1, -1])
    }

    pub fn window_paths(&self) -> Vec<PathBuf> {
        self.paths_at(&[1, -1, 0])
    }

    fn paths_at(&self, offsets: &[isize]) -> Vec<PathBuf> {
        offsets
            .iter()
            .filter_map(|o| self.directory.offset_index(*o))
            .filter_map(|i| self.directory.path_at(i).map(Path::to_path_buf))
            .collect()
    }

    pub fn drain(&mut self) -> Vec<SessionEvent> {
        let index = self.directory.current_index();
        self.cache
            .pump(index)
            .into_iter()
            .map(|(path, outcome)| match outcome {
                Ok(()) => SessionEvent::Loaded(path),
                Err(e) => SessionEvent::Failed(path, e),
            })
            .collect()
    }

    pub fn drain_one_blocking(&mut self) -> Option<SessionEvent> {
        let index = self.directory.current_index();
        self.cache.drain_one(index).map(|(path, outcome)| match outcome {
            Ok(()) => SessionEvent::Loaded(path),
            Err(e) => SessionEvent::Failed(path, e),
        })
    }

    pub fn absorb_load(&mut self, result: crate::cache::LoadResult) -> SessionEvent {
        let index = self.directory.current_index();
        match self.cache.absorb(result, index) {
            (path, Ok(())) => SessionEvent::Loaded(path),
            (path, Err(e)) => SessionEvent::Failed(path, e),
        }
    }

    fn realign_pending(&mut self, scanned: &Directory) -> Option<PathBuf> {
        let pending = self.pending.as_deref()?;
        let scanned_path = scanned.current()?;
        if pending == scanned_path || !crate::directory::same_file(pending, scanned_path) {
            return None;
        }
        let realigned = scanned_path.to_path_buf();
        if let Some(stale) = self.pending.take() {
            self.cache.forget(&stale);
        }
        Some(realigned)
    }

    pub fn adopt_scan(
        &mut self,
        result: std::io::Result<Directory>,
        current: Option<&Path>,
        has_current: bool,
    ) -> Option<ScanOutcome> {
        let mut scanned = match result {
            Ok(scanned) => scanned,
            Err(e) => {
                if !has_current {
                    self.status = Some(format!("failed to read folder: {e}"));
                }
                return None;
            }
        };
        if scanned.is_empty() && !has_current {
            self.status = Some(format!("no supported images in {}", scanned.dir().display()));
        }
        let anchor = current
            .or(self.pending.as_deref())
            .or_else(|| self.directory.current())
            .map(Path::to_path_buf);
        if let Some(anchor) = anchor.as_deref() {
            scanned.jump_to(anchor);
        }
        let realigned = self.realign_pending(&scanned);
        self.directory = scanned;
        self.cache.sync_to_directory(&self.directory);
        let index = self.directory.current_index();
        self.cache.set_current_index(index);
        let first = self.directory.current().map(Path::to_path_buf);
        Some(ScanOutcome { anchored: anchor.is_some(), realigned, first })
    }

    pub fn forget(&mut self, path: &Path) {
        self.cache.forget(path);
    }

    pub fn refresh_directory(&mut self) -> Option<PathBuf> {
        if self.directory.refresh().is_ok() {
            self.directory.current().map(Path::to_path_buf)
        } else {
            None
        }
    }

    pub fn navigate(&mut self, nav: Navigation) -> Option<PathBuf> {
        self.directory.navigate(nav).map(Path::to_path_buf)
    }

    pub fn jump_random(&mut self) -> Option<PathBuf> {
        let mut rng = rand::rng();
        let path = self.directory.entries().choose(&mut rng).cloned()?;
        self.directory.jump_to(&path);
        Some(path)
    }

    pub fn set_target_size(&mut self, width: u32, height: u32) {
        self.cache.set_target_size(width, height);
    }

    pub fn set_resample(&mut self, resample: Resample) {
        self.cache.set_resample(resample);
    }

    pub fn inflight_len(&self) -> usize {
        self.cache.inflight_len()
    }
}

pub struct Viewer {
    pub playback: Playback,
    pub view: ViewState,
    pub presentation: Presentation,
    pub session: Session,
    reprepare_pending: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Fit {
    Apply,
    Keep,
}

impl Default for Viewer {
    fn default() -> Self {
        Self::new()
    }
}

impl Viewer {
    pub fn new() -> Self {
        Self {
            playback: Playback::default(),
            view: ViewState::default(),
            presentation: Presentation::new(),
            session: Session::new(),
            reprepare_pending: false,
        }
    }

    pub fn render_image(&mut self) -> Option<Arc<RenderImage>> {
        self.presentation.render_image(&self.view)
    }

    pub fn set_viewport(&mut self, width: f32, height: f32) -> bool {
        let changed = (self.view.viewport.0 - width).abs() >= f32::EPSILON
            || (self.view.viewport.1 - height).abs() >= f32::EPSILON;
        self.view.viewport = (width, height);
        self.sync_target_size();
        if changed {
            self.reprepare_pending = true;
            if self.presentation.has_current() {
                self.apply_current_fit();
            }
        }
        changed
    }

    pub fn reprepare_if_pending(&mut self) -> bool {
        if !self.reprepare_pending {
            return false;
        }
        self.reprepare_pending = false;
        self.presentation.reprepare_current(&mut self.view);
        true
    }

    fn sync_target_size(&mut self) {
        let physical = self.view.physical_viewport();
        self.session.set_target_size(physical.0.max(1.0) as u32, physical.1.max(1.0) as u32);
    }

    pub fn set_scale_factor(&mut self, scale_factor: f32) -> bool {
        if (scale_factor - self.view.scale_factor).abs() < f32::EPSILON {
            return false;
        }
        self.view.scale_factor = scale_factor;
        self.sync_target_size();
        self.reprepare_pending = true;
        if self.presentation.has_current() {
            self.apply_current_fit();
        }
        true
    }

    pub fn open(&mut self, path: &Path) -> std::io::Result<()> {
        self.presentation.clear_prepared();
        self.presentation.clear_current();
        self.playback.forget_paused();
        if let Some(target) = self.session.begin_scan(path) {
            self.show(&target);
        }
        Ok(())
    }

    pub fn wait_for_scan(&mut self) {
        self.adopt_scan(true);
    }

    pub fn apply_scan(&mut self, generation: u64, result: std::io::Result<Directory>) -> bool {
        if !self.session.discard_scan(generation) {
            return false;
        }
        self.adopt_scan_result(result)
    }

    fn adopt_scan(&mut self, blocking: bool) -> bool {
        let Some(result) = self.session.take_scan(blocking) else {
            return false;
        };
        self.adopt_scan_result(result)
    }

    fn adopt_scan_result(&mut self, result: std::io::Result<Directory>) -> bool {
        let current = self.presentation.current_path().map(Path::to_path_buf);
        let has_current = self.presentation.has_current();
        let Some(outcome) = self.session.adopt_scan(result, current.as_deref(), has_current) else {
            return true;
        };
        let mut redraw = false;
        if outcome.anchored {
            if let Some(path) = outcome.realigned {
                self.session.set_pending(None);
                self.show(&path);
                redraw = true;
            } else if self.resolve_pending() {
                redraw = true;
            }
        } else if let Some(first) = outcome.first {
            self.show(&first);
            redraw = true;
        }
        self.session.prefetch();
        redraw || self.presentation.has_current()
    }

    fn make_current(&mut self, path: &Path, fit: Fit) -> bool {
        let output = self.session.output(path);
        if !self.presentation.present(path, output, &self.view) {
            return false;
        }
        self.playback.reset();
        self.session.set_status(None);
        if fit == Fit::Apply {
            self.apply_current_fit();
        }
        self.playback.restore_for(path);
        true
    }

    fn available(&self, path: &Path) -> bool {
        self.presentation.is_ready(path, &self.view) || self.session.is_cached(path)
    }

    fn apply_current_fit(&mut self) {
        let intrinsic = self.presentation.current_intrinsic();
        let original = self.presentation.original_zoom(&self.view);
        self.view.apply_fit(intrinsic, original);
    }

    pub fn toggle_play(&mut self) {
        let path = self.presentation.current_path().map(Path::to_path_buf);
        self.playback.toggle_play_for(path.as_deref());
    }

    fn neighbours_settled(&self) -> bool {
        if !self.session.neighbour_paths().iter().all(|p| self.presentation.is_ready(p, &self.view))
        {
            return false;
        }
        let window = self.session.window_paths();
        self.presentation.staged_paths().iter().all(|key| window.contains(key))
    }

    pub fn prepare_neighbours(&mut self) {
        if self.neighbours_settled() {
            return;
        }
        for path in self.session.neighbour_paths() {
            if self.presentation.is_ready(&path, &self.view) {
                continue;
            }
            let Some(output) = self.session.output(&path) else {
                continue;
            };
            let prepared = Presentation::prepare(&path, output, &self.view);
            self.presentation.stage(path, prepared);
        }
        let keep = self.session.window_paths();
        self.presentation.retain_prepared(&keep);
    }

    pub fn navigate(&mut self, nav: Navigation) {
        let Some(path) = self.session.navigate(nav) else {
            return;
        };
        self.show(&path);
    }

    pub fn jump_random(&mut self) {
        let Some(path) = self.session.jump_random() else {
            return;
        };
        self.show(&path);
    }

    fn show(&mut self, path: &Path) {
        self.session.focus();
        if self.available(path) {
            self.session.set_pending(None);
            self.make_current(path, Fit::Apply);
        } else {
            self.session.request(path);
        }
        self.session.prefetch();
    }

    pub fn settle(&mut self) {
        self.wait_for_scan();
        while self.session.pending().is_some() {
            let Some(event) = self.session.drain_one_blocking() else {
                break;
            };
            self.apply_event(event);
            self.resolve_pending();
        }
    }

    fn apply_event(&mut self, event: SessionEvent) {
        let SessionEvent::Failed(path, e) = event else {
            return;
        };
        if self.session.pending() == Some(path.as_path()) {
            self.session.set_pending(None);
            self.presentation.clear_current();
            self.session.set_status(Some(format!("{}: {e}", path.display())));
        }
    }

    fn resolve_pending(&mut self) -> bool {
        let Some(pending) = self.session.pending().map(Path::to_path_buf) else {
            return false;
        };
        if self.session.directory.current().is_none_or(|c| c != pending) {
            self.session.set_pending(None);
            return false;
        }
        if self.available(&pending) {
            self.session.set_pending(None);
            self.make_current(&pending, Fit::Apply);
            return true;
        }
        false
    }

    pub fn set_antialias(&mut self, antialias: bool) {
        let resample = Resample::from_antialias(antialias);
        if resample == self.view.resample {
            return;
        }
        self.view.resample = resample;
        self.session.set_resample(resample);
        self.presentation.clear_prepared();
        self.presentation.drop_magnification();
        if let Some(path) = self.presentation.current_path().map(Path::to_path_buf) {
            self.make_current(&path, Fit::Keep);
        }
    }

    pub fn toggle_antialias(&mut self) {
        self.set_antialias(!self.view.antialias());
    }

    pub fn show_pasted(&mut self, image: DecodedImage) {
        let output = Arc::new(crate::decode::DecodeOutput {
            decoded: Decoded::Still(image),
            orientation: crate::decode::Orientation::Normal,
            display: None,
        });
        let path = PathBuf::from("(clipboard)");
        let prepared = Presentation::prepare(&path, output, &self.view);
        self.session.set_pending(None);
        self.presentation.clear_prepared();
        self.presentation.adopt(prepared);
        self.playback.reset();
        self.session.set_status(None);
        self.view.transform.fit = FitMode::Fit;
        self.apply_current_fit();
    }

    pub fn original_zoom(&self) -> f32 {
        self.presentation.original_zoom(&self.view)
    }

    pub fn source_zoom(&self) -> f32 {
        let original = self.original_zoom();
        if original <= 0.0 { self.view.transform.zoom } else { self.view.transform.zoom / original }
    }

    pub fn set_fit(&mut self, fit: FitMode) {
        let intrinsic = self.presentation.current_intrinsic();
        let original = self.presentation.original_zoom(&self.view);
        self.view.set_fit(fit, intrinsic, original);
    }

    pub fn tick_scan(&mut self) -> bool {
        self.adopt_scan(false)
    }

    pub fn tick_cache(&mut self) -> bool {
        let events = self.session.drain();
        let mut redraw = !events.is_empty();
        for event in events {
            self.apply_event(event);
        }
        if self.resolve_pending() {
            redraw = true;
        }
        self.prepare_neighbours();
        redraw
    }

    pub fn tick_playback(&mut self, now: Instant) -> bool {
        let mut redraw = false;
        if let Some(output) = self.presentation.current_output()
            && self.playback.advance(&output.decoded, now)
        {
            redraw = true;
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

    pub fn tick(&mut self, now: Instant) -> bool {
        let mut redraw = self.tick_scan();
        redraw |= self.tick_cache();
        redraw |= self.reprepare_if_pending();
        redraw |= self.tick_playback(now);
        redraw
    }

    pub fn apply_load(&mut self, result: crate::cache::LoadResult) -> bool {
        let event = self.session.absorb_load(result);
        self.apply_event(event);
        self.tick_cache();
        true
    }

    pub fn needs_ticking(&self) -> bool {
        if self.session.scan_pending()
            || self.session.pending().is_some()
            || self.session.inflight_len() > 0
        {
            return true;
        }
        if matches!(self.playback.state, PlaybackState::Present | PlaybackState::PresentRandom) {
            return true;
        }
        self.presentation.is_animated() && self.playback.state == PlaybackState::Playing
    }

    pub fn next_time_based_delay(&self) -> Option<std::time::Duration> {
        if matches!(self.playback.state, PlaybackState::Present | PlaybackState::PresentRandom) {
            return Some(self.playback.present_interval());
        }
        if self.presentation.is_animated() && self.playback.state == PlaybackState::Playing {
            let frame = self.playback.frame_index();
            return Some(
                self.presentation
                    .current_output()
                    .and_then(|output| Playback::frame_delay(&output.decoded, frame))
                    .unwrap_or(std::time::Duration::from_millis(16)),
            );
        }
        None
    }

    pub fn delete_current(&mut self) {
        let Some(path) = self.presentation.current_path().map(Path::to_path_buf) else {
            return;
        };
        if let Err(e) = crate::actions::move_to_trash(&path) {
            self.session.set_status(Some(format!("delete failed: {e}")));
            return;
        }
        self.session.forget(&path);
        match self.session.refresh_directory() {
            Some(next) => self.show(&next),
            None => self.presentation.clear_current(),
        }
    }
}
