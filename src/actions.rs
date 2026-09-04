use std::path::Path;

use crate::decode::{Decoded, DecodedImage};

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

pub fn copy_to_clipboard(decoded: &Decoded) -> Result<(), ActionError> {
    let image = first_frame(decoded).ok_or(ActionError::NoImage)?;
    let data = arboard::ImageData {
        width: image.width as usize,
        height: image.height as usize,
        bytes: std::borrow::Cow::Borrowed(&image.rgba),
    };
    let mut clipboard =
        arboard::Clipboard::new().map_err(|e| ActionError::Clipboard(e.to_string()))?;
    clipboard.set_image(data).map_err(|e| ActionError::Clipboard(e.to_string()))
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

    pub fn bar_background(self) -> u32 {
        match self {
            Theme::Dark => 0x1c1c1c,
            Theme::Light => 0xe2e2e2,
        }
    }

    pub fn text(self) -> u32 {
        match self {
            Theme::Dark => 0xd0d0d0,
            Theme::Light => 0x202020,
        }
    }
}
