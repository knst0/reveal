use std::path::{Path, PathBuf};

use crate::decode::is_supported;
use crate::directory::Directory;

pub fn is_droppable(path: &Path) -> bool {
    path.is_dir() || is_supported(path)
}

pub fn resolve(paths: &[PathBuf]) -> Option<PathBuf> {
    let path = paths.iter().find(|p| is_droppable(p))?;
    if path.is_dir() {
        Directory::open_at(path).ok()?.current().map(Path::to_path_buf)
    } else {
        Some(path.clone())
    }
}
