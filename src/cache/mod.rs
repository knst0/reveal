mod manager;
mod store;
mod worker;

use std::path::PathBuf;
use std::sync::Arc;

pub use manager::ImageCache;
pub use store::{CacheStore, NavigationDirection};
pub use worker::{LoadResult, Loader};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestKind {
    Explicit,
    Prefetch,
}

use crate::decode::{DecodeError, DecodeOutput};
use crate::render::{Display, RenderConfig};

pub const DEFAULT_CAPACITY_BYTES: usize = 512 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct CachedImage {
    pub path: PathBuf,
    pub output: Arc<DecodeOutput>,
    /// Downscaled display texture built on the loader thread for the render
    /// config that was current when the request was made.
    pub display: Option<Arc<Display>>,
    pub bytes: usize,
    /// Source file identity taken from the same handle the bytes were read from.
    pub stamp: Option<crate::directory::FileStamp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RequestId(pub u64);

#[derive(Debug, Clone)]
pub struct LoadRequest {
    pub id: RequestId,
    pub path: PathBuf,
    pub index: usize,
    pub render: RenderConfig,
}

pub type LoadOutcome = (PathBuf, Result<(), DecodeError>);

pub fn measure(output: &DecodeOutput, display: Option<&Display>) -> usize {
    use crate::decode::Decoded;
    let decoded = match &output.decoded {
        Decoded::Still(img) => img.rgba.len(),
        Decoded::Animation(frames) => frames.iter().map(|f| f.image.rgba.len()).sum(),
    };
    let display = display
        .map_or(0, |d| (d.width as usize).saturating_mul(d.height as usize).saturating_mul(4));
    decoded.saturating_add(display)
}
