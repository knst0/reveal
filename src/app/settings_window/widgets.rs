use gpui::{
    Context, InteractiveElement, ParentElement, StatefulInteractiveElement, Styled, div, px,
};
use reveal::ui::{self, Palette};

use super::SettingsWindow;

pub fn section_label(p: Palette, text: &'static str) -> gpui::Div {
    div().text_color(ui::color(p.text_muted)).child(text)
}

pub fn info_row(p: Palette, label: &'static str, value: String) -> gpui::Div {
    div()
        .flex()
        .items_start()
        .gap_3()
        .child(div().w(px(110.)).flex_shrink_0().text_color(ui::color(p.text_muted)).child(label))
        .child(div().flex_grow(1.).child(value))
}

pub fn link_chip(
    id: &'static str,
    p: Palette,
    label: &'static str,
    url: String,
    cx: &mut Context<SettingsWindow>,
) -> gpui::Stateful<gpui::Div> {
    ui::chip(id, p, false).child(label).on_click(cx.listener(move |_this, _e, _w, _cx| {
        if let Err(e) = open::that_detached(&url) {
            log::warn!("failed to open {url}: {e}");
        }
    }))
}

pub fn credit_row(
    id: impl Into<gpui::ElementId>,
    p: Palette,
    title: String,
    repository: &'static str,
    license: &'static str,
    cx: &mut Context<SettingsWindow>,
) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .flex()
        .items_center()
        .gap_3()
        .py_1()
        .px_1()
        .rounded(ui::rem(6.))
        .hover(|s| s.bg(ui::color(p.element_hover)))
        .child(
            div()
                .flex_grow(1.)
                .flex()
                .flex_col()
                .child(div().child(title))
                .child(div().text_color(ui::color(p.text_muted)).child(repository)),
        )
        .child(ui::static_chip(p, false).child(license))
        .on_click(cx.listener(move |_this, _e, _w, _cx| {
            if let Err(e) = open::that_detached(repository) {
                log::warn!("failed to open {repository}: {e}");
            }
        }))
}

pub fn path_label(path: Option<std::path::PathBuf>) -> String {
    match path {
        Some(path) => path.display().to_string(),
        None => "Unavailable".to_owned(),
    }
}
