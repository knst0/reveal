use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::CachedImage;

#[derive(Debug)]
pub struct CacheStore {
    capacity_bytes: usize,
    used_bytes: usize,
    entries: HashMap<PathBuf, Entry>,
}

#[derive(Debug)]
struct Entry {
    image: CachedImage,
    index: usize,
}

impl CacheStore {
    pub fn new(capacity_bytes: usize) -> Self {
        Self { capacity_bytes, used_bytes: 0, entries: HashMap::new() }
    }

    pub fn used_bytes(&self) -> usize {
        self.used_bytes
    }

    pub fn capacity_bytes(&self) -> usize {
        self.capacity_bytes
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn contains(&self, path: &Path) -> bool {
        self.entries.contains_key(path)
    }

    pub fn get(&self, path: &Path) -> Option<&CachedImage> {
        self.entries.get(path).map(|e| &e.image)
    }

    pub fn insert(&mut self, image: CachedImage, index: usize, current_index: usize) {
        if let Some(previous) = self.entries.remove(&image.path) {
            self.used_bytes -= previous.image.bytes;
        }
        self.used_bytes += image.bytes;
        self.entries.insert(image.path.clone(), Entry { image, index });
        self.evict_to_fit(current_index);
    }

    pub fn remove(&mut self, path: &Path) {
        if let Some(entry) = self.entries.remove(path) {
            self.used_bytes -= entry.image.bytes;
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.used_bytes = 0;
    }

    pub fn reindex(&mut self, index_of: impl Fn(&Path) -> Option<usize>) {
        let stale: Vec<PathBuf> =
            self.entries.keys().filter(|p| index_of(p).is_none()).cloned().collect();
        for path in stale {
            self.remove(&path);
        }
        for entry in self.entries.values_mut() {
            if let Some(index) = index_of(&entry.image.path) {
                entry.index = index;
            }
        }
    }

    fn evict_to_fit(&mut self, current_index: usize) {
        while self.used_bytes > self.capacity_bytes && self.entries.len() > 1 {
            let victim = self
                .entries
                .values()
                .max_by_key(|e| e.index.abs_diff(current_index))
                .map(|e| e.image.path.clone());
            match victim {
                Some(path) if self.entries.get(&path).is_some_and(|e| e.index != current_index) => {
                    self.remove(&path)
                }
                _ => break,
            }
        }
    }
}
