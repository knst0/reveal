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

pub fn target(paths: &[PathBuf]) -> Option<PathBuf> {
    paths.iter().find(|p| is_droppable(p)).cloned()
}

/// Files handed over by the desktop environment (Finder, "Open With") before
/// the window exists, and while it is already running.
static REQUESTED: std::sync::Mutex<Vec<PathBuf>> = std::sync::Mutex::new(Vec::new());

pub fn request_open(paths: Vec<PathBuf>) {
    if paths.is_empty() {
        return;
    }
    let mut queue = REQUESTED.lock().unwrap_or_else(|e| e.into_inner());
    queue.extend(paths);
}

pub fn take_requested() -> Vec<PathBuf> {
    let mut queue = REQUESTED.lock().unwrap_or_else(|e| e.into_inner());
    std::mem::take(&mut *queue)
}

pub fn paths_from_urls<I, S>(urls: I) -> Vec<PathBuf>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    urls.into_iter().filter_map(|url| path_from_url(url.as_ref())).collect()
}

fn percent_decode(input: &str) -> Option<String> {
    let mut out = Vec::with_capacity(input.len());
    let mut bytes = input.bytes();
    while let Some(byte) = bytes.next() {
        if byte != b'%' {
            out.push(byte);
            continue;
        }
        let hex = [bytes.next()?, bytes.next()?];
        out.push(u8::from_str_radix(std::str::from_utf8(&hex).ok()?, 16).ok()?);
    }
    String::from_utf8(out).ok()
}

fn path_from_url(url: &str) -> Option<PathBuf> {
    let rest = match url.strip_prefix("file://") {
        Some(rest) => rest.split_once('/').map(|(_host, path)| path)?,
        None if !url.contains("://") => return Some(PathBuf::from(url)),
        None => return None,
    };

    let decoded = percent_decode(rest)?;
    if decoded.is_empty() {
        return None;
    }
    // A Windows URL carries a drive letter, elsewhere the leading slash is part of the path.
    let path = if cfg!(windows) && decoded.as_bytes().get(1) == Some(&b':') {
        decoded
    } else {
        format!("/{decoded}")
    };
    Some(PathBuf::from(path))
}
