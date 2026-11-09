use std::path::{Path, PathBuf};

use super::{
    CacheStore, CachedImage, DEFAULT_CAPACITY_BYTES, LoadResult, Loader, NavigationDirection,
    RequestId, RequestKind,
};
use crate::decode::DecodeError;
use crate::directory::Directory;
use crate::render::RenderConfig;

pub const PREFETCH_RADIUS: isize = 3;

pub fn prefetch_offsets(radius: isize) -> impl Iterator<Item = isize> {
    (1..=radius.max(0)).flat_map(|step| [step, -step])
}

pub struct ImageCache {
    store: CacheStore,
    loader: std::sync::Arc<Loader>,
    inflight: Vec<(PathBuf, RequestId)>,
    render: RenderConfig,
    current: usize,
    last_index: Option<usize>,
    length: usize,
}

impl Default for ImageCache {
    fn default() -> Self {
        Self::new(DEFAULT_CAPACITY_BYTES, default_threads())
    }
}

pub fn default_threads() -> usize {
    std::thread::available_parallelism().map_or(2, |n| n.get().saturating_sub(1).clamp(2, 6))
}

impl ImageCache {
    pub fn new(capacity_bytes: usize, threads: usize) -> Self {
        Self {
            store: CacheStore::new(capacity_bytes),
            loader: std::sync::Arc::new(Loader::new(threads)),
            inflight: Vec::new(),
            render: RenderConfig::default(),
            current: 0,
            last_index: None,
            length: 0,
        }
    }

    pub fn set_render_config(&mut self, render: RenderConfig) {
        self.render = render;
    }

    pub fn store(&self) -> &CacheStore {
        &self.store
    }

    pub fn loader_handle(&self) -> std::sync::Arc<Loader> {
        std::sync::Arc::clone(&self.loader)
    }

    pub fn get(&self, path: &Path) -> Option<&CachedImage> {
        self.store.get(path)
    }

    pub fn request(&mut self, path: &Path) {
        self.request_with(path, self.current, RequestKind::Explicit, None);
    }

    pub fn request_as(&mut self, path: &Path, kind: RequestKind) {
        self.request_with(path, self.current, kind, None);
    }

    /// Requests a specific directory position. Used by prefetching and tests
    /// that exercise queue ordering independently of the focused entry.
    pub fn request_at(&mut self, path: &Path, index: usize) {
        self.request_with(path, index, RequestKind::Explicit, None);
    }

    fn request_with(&mut self, path: &Path, index: usize, kind: RequestKind, size: Option<u64>) {
        if self.store.contains(path) || self.inflight.iter().any(|(p, _)| p == path) {
            return;
        }
        if kind == RequestKind::Prefetch {
            let len = size.or_else(|| std::fs::metadata(path).ok().map(|meta| meta.len()));
            if let Some(len) = len {
                let remaining = self.store.capacity_bytes().saturating_sub(self.store.used_bytes());
                if (len as usize) > remaining {
                    return;
                }
            }
        }
        let id = self.loader.request(path.to_path_buf(), index, self.render);
        self.inflight.push((path.to_path_buf(), id));
    }

    pub fn prefetch_neighbours(&mut self, dir: &Directory) {
        self.prefetch_around(dir, PREFETCH_RADIUS);
    }

    pub fn prefetch_around(&mut self, dir: &Directory, radius: isize) {
        for offset in prefetch_offsets(radius) {
            let Some(index) = dir.offset_index(offset) else {
                continue;
            };
            if index == dir.current_index() {
                continue;
            }
            if let Some(path) = dir.path_at(index) {
                let path = path.to_path_buf();
                let size = dir.size_at(index);
                self.request_with(&path, index, RequestKind::Prefetch, size);
            }
        }
    }

    pub fn inflight_len(&self) -> usize {
        self.inflight.len()
    }

    fn set_current_index(&mut self, index: usize) {
        self.current = index;
        let direction = NavigationDirection::from_transition(self.last_index, index, self.length);
        self.last_index = Some(index);
        self.store.set_direction(direction);
        self.loader.set_current_index(index);
    }

    pub fn cancel_outside_window(&mut self) {
        let index = self.current;
        let radius = PREFETCH_RADIUS.max(0) as usize;
        let dropped = self.loader.cancel_far_from(index, radius);
        let dropped: std::collections::HashSet<RequestId> = dropped.into_iter().collect();
        self.inflight.retain(|(_, id)| !dropped.contains(id));
    }

    /// Focuses a directory position: reindex the store from the directory,
    /// update eviction direction, then drop requests outside the window.
    /// Keeping these together removes the "sync before index" caller rule.
    pub fn focus(&mut self, dir: &Directory) {
        self.sync_to_directory(dir);
        let index = dir.current_index();
        self.set_current_index(index);
        self.cancel_outside_window();
    }

    pub fn cancel_all_inflight(&mut self) {
        self.loader.cancel_everything();
        self.inflight.clear();
    }

    pub fn pump(&mut self) -> Vec<super::LoadOutcome> {
        let mut events = Vec::new();
        while let Some(result) = self.loader.try_recv() {
            events.push(self.absorb(result));
        }
        events
    }

    pub fn absorb(&mut self, result: LoadResult) -> super::LoadOutcome {
        let LoadResult { id, index, path, outcome } = result;
        self.inflight.retain(|(_, request)| *request != id);
        match outcome {
            Ok(image) => {
                let path = image.path.clone();
                self.store.insert(image, index, self.current);
                (path, Ok(()))
            }
            Err(e) => (path, Err(e)),
        }
    }

    pub fn drain_one(&mut self) -> Option<super::LoadOutcome> {
        if self.inflight.is_empty() {
            return None;
        }
        let result = self.loader.recv()?;
        Some(self.absorb(result))
    }

    pub fn block_on(&mut self, path: &Path) -> Result<&CachedImage, DecodeError> {
        self.request(path);
        while !self.store.contains(path) {
            let Some(result) = self.loader.recv() else {
                break;
            };
            let (result_path, outcome) = self.absorb(result);
            if let Err(e) = outcome
                && result_path == path
            {
                return Err(e);
            }
        }
        self.store
            .get(path)
            .ok_or_else(|| DecodeError::Decode("the image loader stopped before decoding".into()))
    }

    pub fn sync_to_directory(&mut self, dir: &Directory) {
        self.length = dir.len();
        self.store.reindex(|p| dir.index_of_path(p));
    }
}

impl ImageCache {
    pub fn forget(&mut self, path: &Path) {
        self.store.remove(path);
    }
}
