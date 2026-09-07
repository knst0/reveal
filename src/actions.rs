use std::path::Path;
use std::sync::{Mutex, OnceLock};

use crate::decode::{Decoded, DecodedImage, Orientation};

#[derive(Debug, thiserror::Error)]
pub enum ActionError {
    #[error("no image is open")]
    NoImage,
    #[error("clipboard: {0}")]
    Clipboard(String),
    #[error("trash: {0}")]
    Trash(String),
}

pub fn first_frame(decoded: &Decoded) -> Option<&DecodedImage> {
    match decoded {
        Decoded::Still(img) => Some(img),
        Decoded::Animation(frames) => frames.first().map(|f| &f.image),
    }
}

static CLIPBOARD: OnceLock<Mutex<Option<arboard::Clipboard>>> = OnceLock::new();

fn with_clipboard<T>(
    f: impl FnOnce(&mut arboard::Clipboard) -> Result<T, ActionError>,
) -> Result<T, ActionError> {
    let cell = CLIPBOARD.get_or_init(|| Mutex::new(None));
    let mut guard = cell.lock().unwrap_or_else(|e| e.into_inner());
    if guard.is_none() {
        *guard =
            Some(arboard::Clipboard::new().map_err(|e| ActionError::Clipboard(e.to_string()))?);
    }
    let clipboard = guard.as_mut().expect("clipboard was just initialised");
    f(clipboard)
}

pub fn copy_to_clipboard(decoded: &Decoded, orientation: Orientation) -> Result<(), ActionError> {
    let image = first_frame(decoded).ok_or(ActionError::NoImage)?;
    let image = crate::render::oriented(image, orientation);
    let data = arboard::ImageData {
        width: image.width as usize,
        height: image.height as usize,
        bytes: std::borrow::Cow::Borrowed(&image.rgba),
    };
    with_clipboard(|clipboard| {
        clipboard.set_image(data).map_err(|e| ActionError::Clipboard(e.to_string()))
    })
}

pub fn paste_from_clipboard() -> Result<DecodedImage, ActionError> {
    let image = with_clipboard(|clipboard| {
        clipboard.get_image().map_err(|e| ActionError::Clipboard(e.to_string()))
    })?;
    let (width, height) = (image.width as u32, image.height as u32);
    if width == 0 || height == 0 {
        return Err(ActionError::NoImage);
    }
    let expected = width as usize * height as usize * 4;
    let mut rgba = image.bytes.into_owned();
    rgba.resize(expected, 0);
    Ok(DecodedImage { width, height, rgba })
}

pub fn move_to_trash(path: &Path) -> Result<(), ActionError> {
    trash::delete(path).map_err(|e| ActionError::Trash(e.to_string()))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    Light,
    Dark,
}

impl Theme {
    pub fn toggled(self) -> Theme {
        match self {
            Theme::Light => Theme::Dark,
            Theme::Dark => Theme::Light,
        }
    }

    pub fn from_dark(dark: bool) -> Theme {
        if dark { Theme::Dark } else { Theme::Light }
    }

    pub fn is_dark(self) -> bool {
        self == Theme::Dark
    }

    pub fn background(self) -> u32 {
        match self {
            Theme::Dark => 0x101010,
            Theme::Light => 0xf2f2f2,
        }
    }

    pub fn text(self) -> u32 {
        match self {
            Theme::Dark => 0xd0d0d0,
            Theme::Light => 0x202020,
        }
    }
}
