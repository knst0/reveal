use gpui::{Hsla, Styled, Svg, svg};

use crate::ui::rem;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    ArrowLeft,
    ArrowRight,
    Maximize,
    Scan,
    Ratio,
    Play,
    Pause,
    Copy,
    Sparkles,
    Settings,
    Minus,
    Plus,
    Square,
    Close,
}

impl Icon {
    fn data(self) -> &'static [u8] {
        match self {
            Icon::ArrowLeft => include_bytes!("../resource/icons/arrow-left.svg"),
            Icon::ArrowRight => include_bytes!("../resource/icons/arrow-right.svg"),
            Icon::Maximize => include_bytes!("../resource/icons/maximize.svg"),
            Icon::Scan => include_bytes!("../resource/icons/scan.svg"),
            Icon::Ratio => include_bytes!("../resource/icons/ratio.svg"),
            Icon::Play => include_bytes!("../resource/icons/play.svg"),
            Icon::Pause => include_bytes!("../resource/icons/pause.svg"),
            Icon::Copy => include_bytes!("../resource/icons/copy.svg"),
            Icon::Sparkles => include_bytes!("../resource/icons/sparkles.svg"),
            Icon::Settings => include_bytes!("../resource/icons/settings.svg"),
            Icon::Minus => include_bytes!("../resource/icons/minus.svg"),
            Icon::Plus => include_bytes!("../resource/icons/plus.svg"),
            Icon::Square => include_bytes!("../resource/icons/square.svg"),
            Icon::Close => include_bytes!("../resource/icons/x.svg"),
        }
    }
}

pub const ICON_SIZE: f32 = 16.0;

pub fn icon(kind: Icon, tint: Hsla) -> Svg {
    svg().size(rem(ICON_SIZE)).flex_none().text_color(tint).data(kind.data())
}
