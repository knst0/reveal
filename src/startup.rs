use std::path::{Path, PathBuf};

use crate::cache::{CachedImage, ImageCache};
use crate::decode::DecodeError;
use crate::directory::Directory;

pub struct FirstImage {
    pub path: PathBuf,
    pub result: Result<(), DecodeError>,
}

pub fn load_first_image(cache: &mut ImageCache, path: &Path, target: (u32, u32)) -> FirstImage {
    cache.set_target_size(target.0, target.1);
    let result = cache.block_on(path, 0).map(|_: &CachedImage| ());
    FirstImage { path: path.to_path_buf(), result }
}

pub fn scan_directory(cache: &mut ImageCache, path: &Path) -> std::io::Result<Directory> {
    let dir = Directory::open_at(path)?;
    cache.sync_to_directory(&dir);
    Ok(dir)
}
