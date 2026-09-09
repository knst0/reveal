use std::cmp::Ordering;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::CachedImage;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NavigationDirection {
    Forward,
    Backward,
    #[default]
    Unknown,
}

impl NavigationDirection {
    pub fn from_transition(previous: Option<usize>, current: usize, length: usize) -> Self {
        let Some(prev) = previous else {
            return Self::Unknown;
        };
        if length > 1 && prev == length - 1 && current == 0 {
            return Self::Forward;
        }
        if length > 1 && prev == 0 && current == length - 1 {
            return Self::Backward;
        }
        match prev.cmp(&current) {
            Ordering::Less => Self::Forward,
            Ordering::Greater => Self::Backward,
            Ordering::Equal => Self::Unknown,
        }
    }
}

const AHEAD_BASE: i64 = 1 << 40;

fn keep_cost(index: usize, current: usize, direction: NavigationDirection) -> i64 {
    let d = index as i64 - current as i64;
    match direction {
        NavigationDirection::Forward => {
            if d < 0 {
                d
            } else {
                AHEAD_BASE - d
            }
        }
        NavigationDirection::Backward => {
            if d > 0 {
                -d
            } else {
                AHEAD_BASE + d
            }
        }
        NavigationDirection::Unknown => -d.abs(),
    }
}

#[derive(Debug)]
pub struct CacheStore {
    capacity_bytes: usize,
    used_bytes: usize,
    direction: NavigationDirection,
    entries: HashMap<PathBuf, Entry>,
}

#[derive(Debug)]
struct Entry {
    image: CachedImage,
    index: usize,
}

impl CacheStore {
    pub fn new(capacity_bytes: usize) -> Self {
        Self {
            capacity_bytes,
            used_bytes: 0,
            direction: NavigationDirection::Unknown,
            entries: HashMap::new(),
        }
    }

    pub fn set_direction(&mut self, direction: NavigationDirection) {
        self.direction = direction;
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
        if self.used_bytes <= self.capacity_bytes {
            return;
        }
        let mut candidates: Vec<(i64, PathBuf, usize)> = self
            .entries
            .values()
            .filter(|e| e.index != current_index)
            .map(|e| {
                (keep_cost(e.index, current_index, self.direction), e.image.path.clone(), e.image.bytes)
            })
            .collect();
        candidates.sort_by(|a, b| b.0.cmp(&a.0));
        while self.used_bytes > self.capacity_bytes {
            let Some((_, path, bytes)) = candidates.pop() else {
                break;
            };
            if self.entries.remove(&path).is_some() {
                self.used_bytes -= bytes;
            }
        }
    }
}
