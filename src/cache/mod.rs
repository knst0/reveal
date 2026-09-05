mod manager;
mod store;
mod worker;

use std::path::{Path, PathBuf};
use std::sync::Arc;

pub use manager::ImageCache;
pub use store::CacheStore;
pub use worker::{LoadResult, Loader};

use crate::decode::{DecodeOutput, Orientation};

pub const DEFAULT_CAPACITY_BYTES: usize = 512 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct CachedImage {
    pub path: PathBuf,
    pub output: Arc<DecodeOutput>,
    pub bytes: usize,
}

impl CachedImage {
    pub fn orientation(&self) -> Orientation {
        self.output.orientation
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RequestId(pub u64);

#[derive(Debug, Clone)]
pub struct LoadRequest {
    pub id: RequestId,
    pub path: PathBuf,
    pub index: usize,
    pub target_width: u32,
    pub target_height: u32,
    pub resample: crate::render::Resample,
}

pub fn measure(output: &DecodeOutput) -> usize {
    use crate::decode::Decoded;
    match &output.decoded {
        Decoded::Still(img) => img.rgba.len(),
        Decoded::Animation(frames) => frames.iter().map(|f| f.image.rgba.len()).sum(),
    }
}

pub fn is_same_file(a: &Path, b: &Path) -> bool {
    a == b
}
