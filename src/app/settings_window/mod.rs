mod about_tab;
mod general_tab;
mod keys_tab;
mod widgets;

use gpui::prelude::FluentBuilder;
use gpui::{
    Context, InteractiveElement, IntoElement, KeyDownEvent, ParentElement, Render,
    StatefulInteractiveElement, Styled, WeakEntity, Window, div, px,
};
use reveal::actions::Theme;
use reveal::input::Modifiers;
use reveal::settings::{SETTINGS_TABS, SettingsState, SettingsTab};
use reveal::ui::{self, Palette};

use super::RevealApp;
use super::titlebar;

pub const WINDOW_WIDTH: f32 = 560.0;
pub const WINDOW_HEIGHT: f32 = 460.0;

pub struct SettingsWindow {
    pub focus: gpui::FocusHandle,
    state: SettingsState,
    theme: Theme,
    owner: WeakEntity<RevealApp>,
}

impl SettingsWindow {
    pub fn new(
        state: SettingsState,
        theme: Theme,
        owner: WeakEntity<RevealApp>,
        cx: &mut Context<Self>,
    ) -> Self {
        Self { focus: cx.focus_handle(), state, theme, owner }
    }

    fn on_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let modifiers = Modifiers {
            alt: event.keystroke.modifiers.alt,
            cmd_ctrl: event.keystroke.modifiers.control || event.keystroke.modifiers.platform,
            shift: event.keystroke.modifiers.shift,
        };

        if self.state.capturing.is_some() {
            if self.state.capture_key(&event.keystroke.key, modifiers) {
                self.preview(cx);
            }
            return;
        }

        if event.keystroke.key == "escape" {
            window.remove_window();
        }
    }

    fn preview(&mut self, cx: &mut Context<Self>) {
        let config = self.state.config.clone();
        let bindings = self.state.bindings.clone();
        self.theme = Theme::from_dark(config.window.dark);
        self.owner
            .update(cx, |app, cx| {
                app.apply_config(config, bindings);
                cx.notify();
            })
            .ok();
        cx.notify();
    }

    fn save(&mut self, cx: &mut Context<Self>) {
        if let Err(e) = self.state.persist() {
            log::warn!("failed to save settings: {e}");
            self.state.notice = Some(format!("Could not save: {e}"));
            return;
        }
        self.state.notice = Some("Settings saved.".to_owned());
        let config = self.state.config.clone();
        let bindings = self.state.bindings.clone();
        self.theme = Theme::from_dark(config.window.dark);
        self.owner
            .update(cx, |app, cx| {
                app.apply_config(config.clone(), bindings.clone());
                app.settings_baseline = Some((config, bindings));
                cx.notify();
            })
            .ok();
    }
}

impl Render for SettingsWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ui::palette(self.theme);
        let tab = self.state.tab;
        let dirty = self.state.is_dirty();
        window.set_rem_size(px(ui::BASE_REM * self.state.config.window.ui_scale.factor()));

        div()
            .size_full()
            .relative()
            .flex()
            .flex_col()
            .overflow_hidden()
            .bg(ui::color(p.background))
            .text_color(ui::color(p.text))
            .text_size(px(13.))
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|this, event, window, cx| this.on_key(event, window, cx)))
            .child(titlebar::header(p, "Settings", window))
            .child(self.render_settings_tabs(tab, p, cx))
            .child(match tab {
                SettingsTab::General => self.render_general_tab(p, cx).into_any_element(),
                SettingsTab::Keys => self.render_keys_tab(p, cx).into_any_element(),
                SettingsTab::About => self.render_about_tab(p, cx).into_any_element(),
            })
            .child(self.render_settings_footer(dirty, p, cx))
            .children(titlebar::resize_handles(window))
    }
}

impl gpui::Focusable for SettingsWindow {
    fn focus_handle(&self, _cx: &gpui::App) -> gpui::FocusHandle {
        self.focus.clone()
    }
}

impl SettingsWindow {
    fn render_settings_tabs(
        &self,
        active: SettingsTab,
        p: Palette,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .flex()
            .flex_shrink_0()
            .items_center()
            .gap_1()
            .px_2()
            .py_1()
            .border_b_1()
            .border_color(ui::color(p.border_variant))
            .children(SETTINGS_TABS.iter().map(|tab| {
                let tab = *tab;
                ui::chip(
                    match tab {
                        SettingsTab::General => "settings-tab-general",
                        SettingsTab::Keys => "settings-tab-keys",
                        SettingsTab::About => "settings-tab-about",
                    },
                    p,
                    tab == active,
                )
                .child(tab.label())
                .on_click(cx.listener(move |this, _e, _w, cx| {
                    this.state.select_tab(tab);
                    cx.notify();
                }))
            }))
    }

    fn settings_body(&self) -> gpui::Stateful<gpui::Div> {
        div().id("settings-body").flex_grow(1.).flex().flex_col().p_3().gap_3().overflow_y_scroll()
    }

    fn render_settings_footer(
        &self,
        dirty: bool,
        p: Palette,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .flex()
            .flex_shrink_0()
            .items_center()
            .gap_2()
            .px_4()
            .py_2()
            .border_t_1()
            .border_color(ui::color(p.border_variant))
            .child(ui::toast_button("settings-reset", p, "Reset shortcuts", false).on_click(
                cx.listener(|this, _e, _w, cx| {
                    this.state.reset_bindings();
                    this.preview(cx);
                }),
            ))
            .child(div().flex_grow(1.))
            .child(ui::toast_button("settings-cancel", p, "Cancel", false).on_click(cx.listener(
                |_this, _e, window, _cx| {
                    window.remove_window();
                },
            )))
            .child(
                ui::toast_button("settings-save", p, "Save", true)
                    .when(!dirty, |s| s.opacity(0.5))
                    .on_click(cx.listener(|this, _e, _w, cx| {
                        this.save(cx);
                        cx.notify();
                    })),
            )
    }
}
