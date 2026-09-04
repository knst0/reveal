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

fn read_entries(dir: &Path) -> io::Result<Vec<PathBuf>> {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_file() && is_supported(p))
        .collect();
    sort_entries(&mut entries);
    Ok(entries)
}

fn modified_of(dir: &Path) -> Option<SystemTime> {
    std::fs::metadata(dir).ok()?.modified().ok()
}

impl Directory {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn open_at(path: &Path) -> io::Result<Self> {
        let (dir, file) = if path.is_dir() {
            (path.to_path_buf(), None)
        } else {
            let dir = path
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .map(Path::to_path_buf)
                .unwrap_or_else(|| PathBuf::from("."));
            (dir, Some(path.to_path_buf()))
        };

        let entries = read_entries(&dir)?;
        let current = file.as_ref().and_then(|f| index_of(&entries, f)).unwrap_or(0);

        Ok(Self { last_modified: modified_of(&dir), dir, entries, current })
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

    pub fn jump_to(&mut self, path: &Path) -> bool {
        match index_of(&self.entries, path) {
            Some(index) => {
                self.current = index;
                true
            }
            None => false,
        }
    }

    pub fn set_index(&mut self, index: usize) -> Option<&Path> {
        if index >= self.entries.len() {
            return None;
        }
        self.current = index;
        self.current()
    }

    pub fn changed_on_disk(&self) -> bool {
        modified_of(&self.dir) != self.last_modified
    }

    pub fn refresh(&mut self) -> io::Result<()> {
        let previous = self.current().map(Path::to_path_buf);
        self.entries = read_entries(&self.dir)?;
        self.last_modified = modified_of(&self.dir);
        self.current = previous
            .as_deref()
            .and_then(|p| index_of(&self.entries, p))
            .unwrap_or_else(|| self.current.min(self.entries.len().saturating_sub(1)));
        Ok(())
    }
}

fn index_of(entries: &[PathBuf], path: &Path) -> Option<usize> {
    let target = std::fs::canonicalize(path).ok();
    entries.iter().position(|e| {
        e == path
            || match (&target, std::fs::canonicalize(e).ok()) {
                (Some(t), Some(c)) => &c == t,
                _ => false,
            }
    })
}
