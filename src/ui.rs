use gpui::prelude::FluentBuilder;
use gpui::{
    Div, Hsla, InteractiveElement, ParentElement, Rems, StatefulInteractiveElement, Styled, div,
    rems, rgb,
};

use crate::actions::Theme;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Palette {
    pub background: u32,
    pub bar: u32,
    pub surface: u32,
    pub elevated: u32,
    pub border: u32,
    pub border_variant: u32,
    pub text: u32,
    pub text_muted: u32,
    pub text_accent: u32,
    pub element_hover: u32,
    pub element_active: u32,
    pub danger: u32,
    pub close_hover: u32,
}

pub const DARK: Palette = Palette {
    background: 0x1a1a1a,
    bar: 0x222222,
    surface: 0x222222,
    elevated: 0x2a2a2a,
    border: 0x3a3a3a,
    border_variant: 0x303030,
    text: 0xcccccc,
    text_muted: 0x888888,
    text_accent: 0xe6e6e6,
    element_hover: 0x323232,
    element_active: 0x3c3c3c,
    danger: 0xcc7a72,
    close_hover: 0xc42b1c,
};

pub const LIGHT: Palette = Palette {
    background: 0xfafafa,
    bar: 0xf0f0ee,
    surface: 0xf0f0ee,
    elevated: 0xffffff,
    border: 0xd8d8d5,
    border_variant: 0xe4e4e1,
    text: 0x383a41,
    text_muted: 0x7f8188,
    text_accent: 0x5c79e2,
    element_hover: 0xe8e8e6,
    element_active: 0xdfdfdc,
    danger: 0xd36151,
    close_hover: 0xc42b1c,
};

pub fn palette(theme: Theme) -> Palette {
    if theme.is_dark() { DARK } else { LIGHT }
}

pub const BASE_REM: f32 = 16.0;

pub fn rem(design_px: f32) -> Rems {
    rems(design_px / BASE_REM)
}

pub const TOOLBAR_HEIGHT: f32 = 40.0;
pub const STATUS_BAR_HEIGHT: f32 = 34.0;
pub const BUTTON_SIZE: f32 = 24.0;
pub const CAPTION_BUTTON_WIDTH: f32 = 46.0;

pub fn color(value: u32) -> Hsla {
    rgb(value).into()
}

pub fn toolbar(p: Palette) -> Div {
    let pad_right = cfg!(target_os = "macos");
    div()
        .h(rem(TOOLBAR_HEIGHT))
        .flex_shrink_0()
        .pl_3()
        .when(pad_right, |s| s.pr_3())
        .flex()
        .items_center()
        .gap_1()
        .bg(color(p.bar))
        .border_b_1()
        .border_color(color(p.border))
        .text_color(color(p.text))
        .text_size(rem(13.))
}

pub fn status_bar(p: Palette) -> Div {
    div()
        .h(rem(STATUS_BAR_HEIGHT))
        .flex_shrink_0()
        .px_3()
        .flex()
        .items_center()
        .gap_3()
        .bg(color(p.bar))
        .border_t_1()
        .border_color(color(p.border))
        .text_color(color(p.text_muted))
        .text_size(rem(13.))
}

pub fn tool_button(id: &'static str, p: Palette, active: bool) -> gpui::Stateful<Div> {
    tool_button_dyn(id, p, active)
}

pub fn tool_button_dyn(
    id: impl Into<gpui::ElementId>,
    p: Palette,
    active: bool,
) -> gpui::Stateful<Div> {
    div()
        .id(id)
        .px_1()
        .h(rem(BUTTON_SIZE))
        .min_w(rem(BUTTON_SIZE))
        .flex()
        .items_center()
        .justify_center()
        .gap_1()
        .rounded(rem(6.))
        .text_color(if active { color(p.text_accent) } else { color(p.text) })
        .when(active, |s| s.bg(color(p.element_active)))
        .hover(|s| s.bg(color(p.element_hover)))
        .active(|s| s.bg(color(p.element_active)))
}

pub fn separator(p: Palette) -> Div {
    div().w(rem(1.)).h(rem(18.)).mx_1().bg(color(p.border_variant))
}

pub fn menu_surface(p: Palette) -> Div {
    div()
        .py_1()
        .min_w(rem(220.))
        .flex()
        .flex_col()
        .bg(color(p.elevated))
        .border_1()
        .border_color(color(p.border))
        .rounded(rem(8.))
        .shadow_lg()
        .text_size(rem(13.))
        .text_color(color(p.text))
}

pub fn menu_separator(p: Palette) -> Div {
    div().my_1().h(rem(1.)).bg(color(p.border_variant))
}

pub struct MenuItem {
    id: gpui::ElementId,
    label: gpui::SharedString,
    keybinding: Option<gpui::SharedString>,
    disabled: bool,
    danger: bool,
}

impl MenuItem {
    pub fn new(id: impl Into<gpui::ElementId>, label: impl Into<gpui::SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            keybinding: None,
            disabled: false,
            danger: false,
        }
    }

    pub fn keybinding(mut self, keys: Option<String>) -> Self {
        self.keybinding = keys.map(Into::into);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn danger(mut self, danger: bool) -> Self {
        self.danger = danger;
        self
    }

    pub fn render(self, p: Palette) -> gpui::Stateful<Div> {
        let text = if self.disabled {
            color(p.text_muted)
        } else if self.danger {
            color(p.danger)
        } else {
            color(p.text)
        };
        div()
            .id(self.id)
            .mx_1()
            .px_2()
            .h(rem(28.))
            .flex()
            .items_center()
            .gap_4()
            .rounded(rem(4.))
            .text_color(text)
            .when(!self.disabled, |s| s.hover(|s| s.bg(color(p.element_hover))))
            .child(div().flex_grow(1.).child(self.label))
            .children(
                self.keybinding.map(|keys| {
                    div().text_color(color(p.text_muted)).text_size(rem(12.)).child(keys)
                }),
            )
    }
}

pub fn overlay_panel(p: Palette) -> Div {
    div()
        .bg(color(p.elevated))
        .border_1()
        .border_color(color(p.border))
        .rounded(rem(8.))
        .shadow_lg()
        .text_color(color(p.text))
}

pub fn panel_header(p: Palette, title: impl Into<gpui::SharedString>) -> Div {
    div()
        .px_4()
        .h(rem(38.))
        .flex()
        .flex_shrink_0()
        .items_center()
        .bg(color(p.surface))
        .border_b_1()
        .border_color(color(p.border))
        .text_size(rem(14.))
        .text_color(color(p.text))
        .child(title.into())
}

pub const TOAST_WIDTH: f32 = 340.0;

pub fn toast_container() -> Div {
    div().absolute().bottom_4().right_4().flex().flex_col().items_end().gap_2()
}

pub fn toast(p: Palette) -> Div {
    overlay_panel(p).w(rem(TOAST_WIDTH)).flex().flex_col().gap_2().p_3().text_size(rem(13.))
}

pub fn toast_title(p: Palette, text: impl Into<gpui::SharedString>) -> Div {
    div().text_size(rem(14.)).text_color(color(p.text_accent)).child(text.into())
}

pub fn toast_body(p: Palette, text: impl Into<gpui::SharedString>) -> Div {
    div().text_color(color(p.text_muted)).child(text.into())
}

pub fn toast_actions() -> Div {
    div().mt_1().flex().items_center().justify_end().gap_2()
}

pub fn toast_button(
    id: &'static str,
    p: Palette,
    label: impl Into<gpui::SharedString>,
    primary: bool,
) -> gpui::Stateful<Div> {
    toast_button_dyn(id, p, label, primary)
}

pub fn toast_button_dyn(
    id: impl Into<gpui::ElementId>,
    p: Palette,
    label: impl Into<gpui::SharedString>,
    primary: bool,
) -> gpui::Stateful<Div> {
    div()
        .id(id)
        .px_2()
        .h(rem(24.))
        .flex()
        .items_center()
        .justify_center()
        .rounded(rem(6.))
        .text_size(rem(12.))
        .bg(color(if primary { p.element_active } else { p.surface }))
        .text_color(color(if primary { p.text_accent } else { p.text }))
        .hover(|s| s.bg(color(p.element_hover)))
        .active(|s| s.bg(color(p.element_active)))
        .child(label.into())
}

pub fn chip(id: impl Into<gpui::ElementId>, p: Palette, active: bool) -> gpui::Stateful<Div> {
    div()
        .id(id)
        .px_2()
        .h(rem(24.))
        .flex()
        .items_center()
        .justify_center()
        .rounded(rem(6.))
        .text_size(rem(12.))
        .bg(color(if active { p.element_active } else { p.surface }))
        .text_color(color(if active { p.text_accent } else { p.text }))
        .hover(|s| s.bg(color(p.element_hover)))
        .active(|s| s.bg(color(p.element_active)))
}

pub fn static_chip(p: Palette, active: bool) -> Div {
    div()
        .px_2()
        .h(rem(24.))
        .flex()
        .items_center()
        .justify_center()
        .rounded(rem(6.))
        .text_size(rem(12.))
        .bg(color(if active { p.element_active } else { p.surface }))
        .text_color(color(if active { p.text_accent } else { p.text }))
}
