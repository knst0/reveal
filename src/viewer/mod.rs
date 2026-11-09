mod presentation;
mod session;
mod view;

pub use presentation::{Origin, Prepared, Presentation};
pub use session::{ScanOutcome, Session};
pub use view::ViewState;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use crate::decode::{Decoded, DecodedImage};
use crate::directory::{Directory, FileStamp, Navigation};
use crate::input::PaneAction;
use crate::playback::{FRAME_RETRY, Playback, PlaybackState};
use crate::render::{FitMode, ImageElement, Resample};
use gpui::RenderImage;

pub struct Viewer {
    pub playback: Playback,
    pub view: ViewState,
    pub presentation: Presentation,
    pub session: Session,
    reprepare_pending: bool,
    watch: Option<SourceWatch>,
}

/// Tracks the on-disk identity of the shown file so edits made by other
/// programs are picked up. A change is acted on only once the same new stamp
/// is seen on two consecutive polls, so a file still being written is not
/// decoded half-way.
struct SourceWatch {
    path: PathBuf,
    seen: Option<FileStamp>,
    candidate: Option<FileStamp>,
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
            watch: None,
        }
    }

    pub fn render_image(&mut self) -> Option<Arc<RenderImage>> {
        self.presentation.render_image(&self.view)
    }

    /// The rendered image and the element that paints it, read together so
    /// the magnification crop can never be paired with a different frame.
    pub fn image_element(&mut self) -> (Option<Arc<RenderImage>>, ImageElement) {
        let image = self.render_image();
        let element = ImageElement::new(
            image.clone(),
            self.presentation.current_intrinsic(),
            self.view.transform,
            self.playback.frame_index(),
            self.presentation.render_crop(),
        );
        (image, element)
    }

    /// Runs a pane-local action. `PaneAction` is the single list shared with
    /// key dispatch, so a new pane action cannot silently do nothing.
    pub fn apply_pane_action(&mut self, action: PaneAction) {
        let centre = (self.view.viewport.0 / 2.0, self.view.viewport.1 / 2.0);
        match action {
            PaneAction::Next => self.navigate(Navigation::Next),
            PaneAction::Prev => self.navigate(Navigation::Prev),
            PaneAction::Original => self.set_fit(FitMode::Original),
            PaneAction::Fit => self.set_fit(FitMode::Fit),
            PaneAction::FitBest => self.set_fit(FitMode::FitBest),
            PaneAction::PanUp => self.view.pan((0.0, 50.0)),
            PaneAction::PanDown => self.view.pan((0.0, -50.0)),
            PaneAction::PanLeft => self.view.pan((50.0, 0.0)),
            PaneAction::PanRight => self.view.pan((-50.0, 0.0)),
            PaneAction::ZoomIn => self.view.zoom_at(1.25, centre),
            PaneAction::ZoomOut => self.view.zoom_at(0.8, centre),
            PaneAction::TogglePlay => self.toggle_play(),
            PaneAction::Present => self.playback.set_state(PlaybackState::Present),
            PaneAction::PresentRandom => self.playback.set_state(PlaybackState::PresentRandom),
        }
    }

    pub fn transform(&self) -> crate::render::ViewTransform {
        self.view.transform
    }

    pub fn intrinsic(&self) -> (f32, f32) {
        self.presentation.current_intrinsic()
    }

    pub fn source_size(&self) -> (u32, u32) {
        self.presentation.current_source_size()
    }

    pub fn antialias(&self) -> bool {
        self.view.antialias()
    }

    pub fn image_contains(&self, point: (f32, f32)) -> bool {
        self.view.transform.image_contains(self.intrinsic(), self.view.viewport, point)
    }

    pub fn pan(&mut self, delta: (f32, f32)) {
        self.view.pan(delta);
    }

    pub fn zoom_at(&mut self, factor: f32, cursor: (f32, f32)) {
        self.view.zoom_at(factor, cursor);
    }

    pub fn set_playback(&mut self, state: PlaybackState) {
        self.playback.set_state(state);
    }

    pub fn is_playing(&self) -> bool {
        self.playback.state == PlaybackState::Playing
    }

    pub fn set_transform(&mut self, transform: crate::render::ViewTransform) {
        self.view.transform = transform;
    }

    pub fn status(&self) -> Option<&str> {
        self.session.status()
    }

    pub fn current_file(&self) -> Option<&Path> {
        self.presentation.current_file()
    }

    pub fn current_origin(&self) -> Option<Origin> {
        self.presentation.current_origin()
    }

    pub fn has_image(&self) -> bool {
        self.presentation.has_current()
    }

    pub fn is_animated(&self) -> bool {
        self.presentation.is_animated()
    }

    pub fn current_output(&self) -> Option<Arc<crate::decode::DecodeOutput>> {
        self.presentation.current_output()
    }

    /// One-based position in the folder, with a non-zero length for display.
    pub fn position(&self) -> Option<(usize, usize)> {
        self.presentation.current_file().map(|_| {
            (self.session.directory.current_index() + 1, self.session.directory.len().max(1))
        })
    }

    /// Filename, `Clipboard`, or nothing when no image is shown.
    pub fn display_name(&self) -> Option<String> {
        match (self.current_file(), self.current_origin()) {
            (Some(path), _) => {
                Some(path.file_name().unwrap_or_default().to_string_lossy().into_owned())
            }
            (None, Some(Origin::Clipboard)) => Some("Clipboard".to_owned()),
            (None, _) => None,
        }
    }

    /// The directory entry the viewer is pointed at, even before it is shown.
    pub fn directory_current(&self) -> Option<&Path> {
        self.session.directory.current()
    }

    pub fn loader_handle(&self) -> std::sync::Arc<crate::cache::Loader> {
        self.session.cache.loader_handle()
    }

    pub fn scan_receiver(
        &self,
    ) -> Option<(crossbeam_channel::Receiver<std::io::Result<Directory>>, u64)> {
        self.session.scan_receiver()
    }

    pub fn scan_generation(&self) -> u64 {
        self.session.scan_generation()
    }

    /// Copies the display settings that must match across panes before the
    /// viewer has any content. One call keeps their order out of callers.
    pub fn inherit_view(&mut self, other: &Viewer) {
        self.view.resample = other.view.resample;
        self.view.scale_factor = other.view.scale_factor;
        self.view.viewport = other.view.viewport;
        self.sync_render_config();
        self.reprepare_pending = false;
    }

    pub fn set_viewport(&mut self, width: f32, height: f32) -> bool {
        let changed = (self.view.viewport.0 - width).abs() >= f32::EPSILON
            || (self.view.viewport.1 - height).abs() >= f32::EPSILON;
        self.view.viewport = (width, height);
        self.sync_render_config();
        if changed {
            self.reprepare_pending = true;
            if self.presentation.has_current() {
                self.apply_current_fit();
                let intrinsic = self.presentation.current_intrinsic();
                self.view.clamp_view(intrinsic);
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

    fn sync_render_config(&mut self) {
        self.session.set_render_config(self.view.render_config());
    }

    pub fn set_scale_factor(&mut self, scale_factor: f32) -> bool {
        if (scale_factor - self.view.scale_factor).abs() < f32::EPSILON {
            return false;
        }
        self.view.scale_factor = scale_factor;
        self.sync_render_config();
        self.reprepare_pending = true;
        if self.presentation.has_current() {
            self.apply_current_fit();
        }
        true
    }

    /// Points the viewer at a file or folder. The directory scan and first
    /// decode are asynchronous, so this only changes the requested selection.
    pub fn open(&mut self, path: &Path) {
        self.presentation.clear_prepared();
        self.presentation.clear_current();
        self.playback.forget_paused();
        if let Some(target) = self.session.begin_scan(path) {
            self.show(&target);
        }
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
        let current = self.presentation.current_file().map(Path::to_path_buf);
        let Some(outcome) = self.session.adopt_scan(result, current.as_deref()) else {
            return true;
        };
        let mut redraw = false;
        if let Some(path) = outcome.realigned {
            self.show(&path);
            redraw = true;
        } else if self.resolve_pending() {
            redraw = true;
        } else if !self.presentation.has_current()
            && let Some(first) = outcome.first
        {
            self.show(&first);
            redraw = true;
        }
        self.session.prefetch();
        redraw || self.presentation.has_current()
    }

    fn make_current(&mut self, path: &Path) -> bool {
        let entry = self.session.cached(path);
        if !self.presentation.present(path, entry.as_ref(), Origin::File, &self.view) {
            return false;
        }
        self.playback.reset();
        self.session.set_status(None);
        self.apply_current_fit();
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
        let path = self.presentation.current_file().map(Path::to_path_buf);
        self.playback.toggle_play_for(path.as_deref());
    }

    fn neighbours_settled(&self) -> bool {
        if !self.session.neighbour_paths().iter().all(|p| self.presentation.is_ready(p, &self.view))
        {
            return false;
        }
        let window = self.session.window_paths();
        self.presentation.staged_within(&window)
    }

    pub fn prepare_neighbours(&mut self) {
        if self.neighbours_settled() {
            return;
        }
        for path in self.session.neighbour_paths() {
            if self.presentation.is_ready(&path, &self.view) {
                continue;
            }
            let Some(entry) = self.session.cached(&path) else {
                continue;
            };
            let prepared = Presentation::prepare(
                &path,
                entry.output.clone(),
                entry.display.clone(),
                Origin::File,
                &self.view,
            );
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
            self.make_current(path);
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

    fn apply_event(&mut self, (path, result): crate::cache::LoadOutcome) -> bool {
        let Err(e) = result else {
            return false;
        };
        if self.session.pending() != Some(path.as_path()) {
            return false;
        }
        self.session.set_pending(None);
        self.presentation.clear_current();
        self.session.set_status(Some(format!("{}: {e}", path.display())));
        true
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
            self.make_current(&pending);
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
        self.sync_render_config();
        if self.presentation.has_current() {
            self.presentation.reprepare_current(&mut self.view);
        } else {
            self.presentation.clear_prepared();
            self.presentation.drop_magnification();
        }
    }

    pub fn show_pasted(&mut self, image: DecodedImage) {
        let path = PathBuf::from("(clipboard)");
        let output = Arc::new(crate::decode::DecodeOutput {
            decoded: Decoded::Still(image),
            orientation: crate::decode::Orientation::Normal,
        });
        let prepared = Presentation::prepare(&path, output, None, Origin::Clipboard, &self.view);
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
        self.view.transform.zoom / self.original_zoom()
    }

    pub fn set_fit(&mut self, fit: FitMode) {
        let intrinsic = self.presentation.current_intrinsic();
        let original = self.presentation.original_zoom(&self.view);
        self.view.set_fit(fit, intrinsic, original);
    }

    fn tick_playback(&mut self, now: Instant) -> bool {
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

    /// Advances state driven by timers and by results the app already fed in.
    /// Decode and scan results arrive through `apply_load`/`apply_scan`; this
    /// never steals them from the forwarding threads.
    pub fn tick(&mut self, now: Instant) -> bool {
        let mut redraw = self.resolve_pending();
        self.prepare_neighbours();
        redraw |= self.reprepare_if_pending();
        redraw |= self.tick_playback(now);
        redraw
    }

    pub fn apply_load(&mut self, result: crate::cache::LoadResult) -> bool {
        let outcome = self.session.absorb_load(result);
        self.apply_event(outcome)
    }

    pub fn next_time_based_delay(&self) -> Option<std::time::Duration> {
        if matches!(self.playback.state, PlaybackState::Present | PlaybackState::PresentRandom) {
            return Some(self.playback.present_interval());
        }
        if self.presentation.is_animated() && self.playback.state == PlaybackState::Playing {
            return Some(
                self.presentation
                    .current_output()
                    .map(|output| self.playback.next_frame_delay(&output.decoded))
                    .unwrap_or(FRAME_RETRY),
            );
        }
        None
    }

    /// Polls the shown file and re-decodes it when its contents changed on disk.
    /// Returns true when a reload was requested.
    pub fn reload_if_source_changed(&mut self) -> bool {
        let Some(path) = self.presentation.current_file().map(Path::to_path_buf) else {
            self.watch = None;
            return false;
        };
        if self.session.pending().is_some() || self.session.directory.current() != Some(&path) {
            return false;
        }
        let now = FileStamp::of(&path);
        if self.watch.as_ref().is_none_or(|watch| watch.path != path) {
            let seen = self.session.cache.get(&path).and_then(|c| c.stamp).or(now);
            self.watch = Some(SourceWatch { path: path.clone(), seen, candidate: None });
        }
        let Some(watch) = self.watch.as_mut() else {
            return false;
        };
        if now.is_none() || now == watch.seen {
            watch.candidate = None;
            return false;
        }
        if watch.candidate != now {
            watch.candidate = now;
            return false;
        }
        watch.seen = now;
        watch.candidate = None;
        self.session.forget(&path);
        self.presentation.unstage(&path);
        self.session.request(&path);
        true
    }

    pub fn delete_current(&mut self) {
        let Some(path) = self.presentation.current_file().map(Path::to_path_buf) else {
            self.session.set_status(Some("the pasted image has no file to delete".to_owned()));
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
