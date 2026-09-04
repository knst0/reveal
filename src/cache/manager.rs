use std::path::{Path, PathBuf};

use super::{CacheStore, CachedImage, DEFAULT_CAPACITY_BYTES, Loader, RequestId};
use crate::decode::DecodeError;
use crate::directory::Directory;

pub struct ImageCache {
    store: CacheStore,
    loader: Loader,
    inflight: Vec<(PathBuf, RequestId)>,
    target: (u32, u32),
}

impl Default for ImageCache {
    fn default() -> Self {
        Self::new(DEFAULT_CAPACITY_BYTES, 2)
    }
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
        for offset in [1isize, -1] {
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
