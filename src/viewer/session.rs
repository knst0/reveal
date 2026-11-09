use std::path::{Path, PathBuf};

use rand::seq::IndexedRandom;

use crate::cache::{CachedImage, ImageCache};
use crate::directory::{Directory, Navigation};
use crate::render::RenderConfig;

/// Decoded neighbours staged for instant navigation. Cache prefetching warms
/// a wider window than this; staging only needs the images a keypress reaches.
const STAGE_RADIUS: isize = 1;

pub struct ScanOutcome {
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

    pub fn begin_scan(&mut self, path: &Path) -> Option<PathBuf> {
        self.cache.cancel_all_inflight();
        self.pending = None;
        self.status = None;
        self.scan_generation = self.scan_generation.wrapping_add(1);
        let scan = crate::directory::PendingScan::spawn(path);
        let target = scan.target().map(Path::to_path_buf);
        match target.as_deref() {
            Some(target) => self.directory = Directory::single(target),
            None => self.directory = Directory::empty(),
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
        self.pending = Some(path.to_path_buf());
        self.cache.request(path);
    }

    pub fn focus(&mut self) {
        self.cache.focus(&self.directory);
    }

    pub fn prefetch(&mut self) {
        self.cache.prefetch_neighbours(&self.directory);
    }

    pub fn cached(&self, path: &Path) -> Option<CachedImage> {
        self.cache.get(path).cloned()
    }

    pub fn is_cached(&self, path: &Path) -> bool {
        self.cache.get(path).is_some()
    }

    pub fn neighbour_paths(&self) -> Vec<PathBuf> {
        self.paths_at(&[STAGE_RADIUS, -STAGE_RADIUS])
    }

    pub fn window_paths(&self) -> Vec<PathBuf> {
        self.paths_at(&[STAGE_RADIUS, -STAGE_RADIUS, 0])
    }

    fn paths_at(&self, offsets: &[isize]) -> Vec<PathBuf> {
        offsets
            .iter()
            .filter_map(|o| self.directory.offset_index(*o))
            .filter_map(|i| self.directory.path_at(i).map(Path::to_path_buf))
            .collect()
    }

    pub fn drain_one_blocking(&mut self) -> Option<crate::cache::LoadOutcome> {
        self.cache.drain_one()
    }

    pub fn absorb_load(&mut self, result: crate::cache::LoadResult) -> crate::cache::LoadOutcome {
        self.cache.absorb(result)
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
    ) -> Option<ScanOutcome> {
        let has_current = current.is_some();
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
        self.cache.focus(&self.directory);
        let first = self.directory.current().map(Path::to_path_buf);
        Some(ScanOutcome { realigned, first })
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

    pub fn set_render_config(&mut self, config: RenderConfig) {
        self.cache.set_render_config(config);
    }
}
