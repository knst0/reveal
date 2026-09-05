use std::path::{Path, PathBuf};

use super::{CacheStore, CachedImage, DEFAULT_CAPACITY_BYTES, Loader, RequestId};
use crate::decode::DecodeError;
use crate::directory::Directory;

pub const PREFETCH_RADIUS: isize = 3;

pub fn prefetch_offsets(radius: isize) -> Vec<isize> {
    let mut offsets = Vec::new();
    for step in 1..=radius.max(0) {
        offsets.push(step);
        offsets.push(-step);
    }
    offsets
}

pub struct ImageCache {
    store: CacheStore,
    loader: Loader,
    inflight: Vec<(PathBuf, RequestId)>,
    target: (u32, u32),
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
            loader: Loader::new(threads),
            inflight: Vec::new(),
            target: (0, 0),
        }
    }

    pub fn set_target_size(&mut self, width: u32, height: u32) {
        self.target = (width, height);
    }

    pub fn store(&self) -> &CacheStore {
        &self.store
    }

    pub fn get(&self, path: &Path) -> Option<&CachedImage> {
        self.store.get(path)
    }

    pub fn request(&mut self, path: &Path, index: usize) {
        if self.store.contains(path) || self.inflight.iter().any(|(p, _)| p == path) {
            return;
        }
        let id = self.loader.request(path.to_path_buf(), index, self.target);
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
                self.request(&path, index);
            }
        }
    }

    pub fn inflight_len(&self) -> usize {
        self.inflight.len()
    }

    pub fn set_current_index(&mut self, index: usize) {
        self.loader.set_current_index(index);
    }

    pub fn cancel_outside_window(&mut self, index: usize) {
        let radius = PREFETCH_RADIUS.max(0) as usize;
        let dropped = self.loader.cancel_far_from(index, radius);
        self.inflight.retain(|(_, id)| !dropped.contains(id));
    }

    pub fn cancel_all_inflight(&mut self) {
        self.loader.cancel_everything();
        self.inflight.clear();
    }

    pub fn cancel_far_from(&mut self, keep: &[PathBuf]) {
        let keep_ids: Vec<RequestId> =
            self.inflight.iter().filter(|(p, _)| keep.contains(p)).map(|(_, id)| *id).collect();
        self.loader.cancel_all_except(&keep_ids);
        self.inflight.retain(|(p, _)| keep.contains(p));
    }

    pub fn pump(&mut self, current_index: usize) -> Vec<(PathBuf, Result<(), DecodeError>)> {
        let mut events = Vec::new();
        while let Some(result) = self.loader.try_recv() {
            self.inflight.retain(|(_, id)| *id != result.id);
            match result.outcome {
                Ok(image) => {
                    let path = image.path.clone();
                    self.store.insert(image, result.index, current_index);
                    events.push((path, Ok(())));
                }
                Err(e) => events.push((result.path, Err(e))),
            }
        }
        events
    }

    pub fn drain_one(
        &mut self,
        current_index: usize,
    ) -> Option<(PathBuf, Result<(), DecodeError>)> {
        if self.inflight.is_empty() {
            return None;
        }
        let result = self.loader.recv()?;
        self.inflight.retain(|(_, id)| *id != result.id);
        match result.outcome {
            Ok(image) => {
                let path = image.path.clone();
                self.store.insert(image, result.index, current_index);
                Some((path, Ok(())))
            }
            Err(e) => Some((result.path, Err(e))),
        }
    }

    pub fn block_on(&mut self, path: &Path, index: usize) -> Result<&CachedImage, DecodeError> {
        self.request(path, index);
        while !self.store.contains(path) {
            let Some(result) = self.loader.recv() else {
                break;
            };
            self.inflight.retain(|(_, id)| *id != result.id);
            let failed_path = result.path.clone();
            match result.outcome {
                Ok(image) => self.store.insert(image, result.index, index),
                Err(e) if failed_path == path => return Err(e),
                Err(_) => continue,
            }
        }
        self.store.get(path).ok_or_else(|| DecodeError::Decode("image was not cached".into()))
    }

    pub fn sync_to_directory(&mut self, dir: &Directory) {
        self.store.reindex(|p| dir.entries().iter().position(|e| e.as_path() == p));
    }
}

impl ImageCache {
    pub fn forget(&mut self, path: &Path) {
        self.store.remove(path);
    }
}
