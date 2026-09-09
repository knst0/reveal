use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::decode::is_supported;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Navigation {
    Next,
    Prev,
}

#[derive(Debug, Default)]
pub struct Directory {
    dir: PathBuf,
    entries: Vec<PathBuf>,
    index: HashMap<PathBuf, usize>,
    sizes: Vec<Option<u64>>,
    current: usize,
    last_modified: Option<SystemTime>,
}

fn sort_entries(entries: &mut [PathBuf]) {
    entries.sort_by(|a, b| {
        let a = a.file_name().unwrap_or_default().to_string_lossy();
        let b = b.file_name().unwrap_or_default().to_string_lossy();
        lexical_sort::natural_lexical_cmp(&a, &b)
    });
}

fn build_index(entries: &[PathBuf]) -> HashMap<PathBuf, usize> {
    let mut index = HashMap::with_capacity(entries.len());
    for (i, path) in entries.iter().enumerate() {
        index.entry(path.clone()).or_insert(i);
    }
    index
}

fn build_sizes(entries: &[PathBuf]) -> Vec<Option<u64>> {
    entries
        .iter()
        .map(|path| std::fs::metadata(path).ok().map(|meta| meta.len()))
        .collect()
}

fn read_entries(dir: &Path) -> io::Result<Vec<PathBuf>> {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)?
        .filter_map(Result::ok)
        .filter(|e| match e.file_type() {
            Ok(t) => t.is_file() || (t.is_symlink() && e.path().is_file()),
            Err(_) => false,
        })
        .map(|e| e.path())
        .filter(|p| is_supported(p))
        .collect();
    sort_entries(&mut entries);
    Ok(entries)
}

pub fn split_target(path: &Path) -> (PathBuf, Option<PathBuf>) {
    if path.is_dir() {
        (path.to_path_buf(), None)
    } else {
        let dir = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        (dir, Some(path.to_path_buf()))
    }
}

fn modified_of(dir: &Path) -> Option<SystemTime> {
    std::fs::metadata(dir).ok()?.modified().ok()
}

impl Directory {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn single(path: &Path) -> Self {
        let (dir, file) = split_target(path);
        let entries: Vec<PathBuf> = file.into_iter().collect();
        let index = build_index(&entries);
        let sizes = build_sizes(&entries);
        Self { last_modified: None, dir, entries, index, sizes, current: 0 }
    }

    pub fn open_at(path: &Path) -> io::Result<Self> {
        let (dir, file) = split_target(path);

        let entries = read_entries(&dir)?;
        let current = file.as_ref().and_then(|f| index_of(&entries, f)).unwrap_or(0);
        let index = build_index(&entries);
        let sizes = build_sizes(&entries);

        Ok(Self { last_modified: modified_of(&dir), dir, entries, index, sizes, current })
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn entries(&self) -> &[PathBuf] {
        &self.entries
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn current_index(&self) -> usize {
        self.current
    }

    pub fn current(&self) -> Option<&Path> {
        self.entries.get(self.current).map(PathBuf::as_path)
    }

    pub fn path_at(&self, index: usize) -> Option<&Path> {
        self.entries.get(index).map(PathBuf::as_path)
    }

    pub fn offset_index(&self, offset: isize) -> Option<usize> {
        let len = self.entries.len();
        if len == 0 {
            return None;
        }
        let len_i = len as isize;
        let raw = self.current as isize + offset;
        Some(raw.rem_euclid(len_i) as usize)
    }

    pub fn navigate(&mut self, nav: Navigation) -> Option<&Path> {
        let offset = match nav {
            Navigation::Next => 1,
            Navigation::Prev => -1,
        };
        self.current = self.offset_index(offset)?;
        self.current()
    }

    pub fn size_at(&self, index: usize) -> Option<u64> {
        self.sizes.get(index).copied().flatten()
    }

    pub fn index_of_path(&self, path: &Path) -> Option<usize> {
        match self.index.get(path) {
            Some(index) => Some(*index),
            None => index_of(&self.entries, path),
        }
    }

    pub fn jump_to(&mut self, path: &Path) -> bool {
        match self.index_of_path(path) {
            Some(index) => {
                self.current = index;
                true
            }
            None => false,
        }
    }

    pub fn refresh(&mut self) -> io::Result<()> {
        let previous = self.current().map(Path::to_path_buf);
        let mut known: HashMap<PathBuf, u64> = self
            .entries
            .iter()
            .zip(self.sizes.iter())
            .filter_map(|(path, size)| size.map(|size| (path.clone(), size)))
            .collect();
        self.entries = read_entries(&self.dir)?;
        self.index = build_index(&self.entries);
        self.sizes = self
            .entries
            .iter()
            .map(|path| {
                known.remove(path).or_else(|| std::fs::metadata(path).ok().map(|m| m.len()))
            })
            .collect();
        self.last_modified = modified_of(&self.dir);
        self.current = previous
            .as_deref()
            .and_then(|p| index_of(&self.entries, p))
            .unwrap_or_else(|| self.current.min(self.entries.len().saturating_sub(1)));
        Ok(())
    }
}

pub fn same_file(a: &Path, b: &Path) -> bool {
    if a == b {
        return true;
    }
    if a.file_name() != b.file_name() {
        return false;
    }
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

fn index_of(entries: &[PathBuf], path: &Path) -> Option<usize> {
    if let Some(index) = entries.iter().position(|e| e == path) {
        return Some(index);
    }
    let name = path.file_name();
    let mut target: Option<Option<PathBuf>> = None;
    for (index, entry) in entries.iter().enumerate() {
        if entry.file_name() != name {
            continue;
        }
        let target = target.get_or_insert_with(|| std::fs::canonicalize(path).ok());
        let (Some(t), Some(c)) = (target.as_ref(), std::fs::canonicalize(entry).ok()) else {
            continue;
        };
        if &c == t {
            return Some(index);
        }
    }
    None
}

pub struct PendingScan {
    target: Option<PathBuf>,
    receiver: crossbeam_channel::Receiver<io::Result<Directory>>,
}

impl PendingScan {
    pub fn spawn(path: &Path) -> Self {
        let (_, file) = split_target(path);
        let (sender, receiver) = crossbeam_channel::unbounded();
        let request = path.to_path_buf();
        std::thread::Builder::new()
            .name("reveal-scan".into())
            .spawn(move || {
                let _ = sender.send(Directory::open_at(&request));
            })
            .ok();
        Self { target: file, receiver }
    }

    pub fn target(&self) -> Option<&Path> {
        self.target.as_deref()
    }

    pub fn take(&mut self) -> Option<io::Result<Directory>> {
        match self.receiver.try_recv() {
            Ok(result) => Some(result),
            Err(crossbeam_channel::TryRecvError::Empty) => None,
            Err(crossbeam_channel::TryRecvError::Disconnected) => {
                Some(Err(io::Error::other("directory scan failed")))
            }
        }
    }

    pub fn results(&self) -> crossbeam_channel::Receiver<io::Result<Directory>> {
        self.receiver.clone()
    }

    pub fn wait(&mut self) -> io::Result<Directory> {
        self.receiver.recv().map_err(|_| io::Error::other("directory scan failed"))?
    }
}
