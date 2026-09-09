use gpui::prelude::FluentBuilder;
use gpui::{
    AppContext, Context, InteractiveElement, IntoElement, KeyDownEvent, ParentElement, Render,
    StatefulInteractiveElement, Styled, WeakEntity, Window, div, px,
};
use reveal::actions::Theme;
use reveal::config::{Channel, UI_SCALES};
use reveal::input::{ALL_ACTIONS, Action, Binding, Modifiers};
use reveal::settings::{
    APPEARANCE_FIELDS, SETTINGS_TABS, SettingsState, SettingsTab, ToggleField, UPDATE_FIELDS,
};
use reveal::ui::{self, Palette};

use super::RevealApp;
use super::labels::{action_label, format_binding};
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

    fn render_general_tab(&self, p: Palette, cx: &mut Context<Self>) -> impl IntoElement {
        let config = self.state.config.clone();
        let channel = config.updates.channel;
        let notice = self.state.notice.clone();

        let mut appearance = Vec::new();
        for field in APPEARANCE_FIELDS {
            appearance.push(toggle_row(*field, &config, p, cx));
        }
        let mut updates = Vec::new();
        for field in UPDATE_FIELDS {
            updates.push(toggle_row(*field, &config, p, cx));
        }

        self.settings_body()
            .children(notice.map(|text| div().text_color(ui::color(p.text_accent)).child(text)))
            .child(section_label(p, "Appearance"))
            .children(appearance)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(div().flex_grow(1.).child("Interface scale"))
                    .children(UI_SCALES.iter().map(|scale| {
                        let scale = *scale;
                        ui::chip(("ui-scale", scale as usize), p, scale == config.window.ui_scale)
                            .child(scale.label())
                            .on_click(cx.listener(move |this, _e, _w, cx| {
                                this.state.set_ui_scale(scale);
                                this.preview(cx);
                            }))
                    })),
            )
            .child(section_label(p, "Updates"))
            .children(updates)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(div().flex_grow(1.).child("Release channel"))
                    .children([Channel::Stable, Channel::Beta].into_iter().map(|option| {
                        ui::chip(
                            match option {
                                Channel::Stable => "channel-stable",
                                Channel::Beta => "channel-beta",
                            },
                            p,
                            option == channel,
                        )
                        .child(option.label())
                        .on_click(cx.listener(move |this, _e, _w, cx| {
                            this.state.set_channel(option);
                            this.preview(cx);
                        }))
                    })),
            )
            .child(section_label(p, "File associations"))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        div()
                            .flex_grow(1.)
                            .flex()
                            .flex_col()
                            .child(div().child("Default image viewer"))
                            .child(
                                div()
                                    .text_color(ui::color(p.text_muted))
                                    .child(associations_hint()),
                            ),
                    )
                    .child(ui::toast_button("set-defaults", p, "Set defaults", false).on_click(
                        cx.listener(|this, _e, _w, cx| {
                            this.state.notice = Some("Setting defaults\u{2026}".to_owned());
                            cx.spawn(async move |this, cx| {
                                let notice = cx
                                    .background_spawn(async {
                                        reveal::settings::association_notice()
                                    })
                                    .await;
                                let _ = this.update(cx, |this, cx| {
                                    this.state.notice = Some(notice);
                                    cx.notify();
                                });
                            })
                            .detach();
                            cx.notify();
                        }),
                    )),
            )
            .child(div().text_color(ui::color(p.text_muted)).child(config_location_hint()))
    }

    fn render_keys_tab(&self, p: Palette, cx: &mut Context<Self>) -> impl IntoElement {
        let capturing = self.state.capturing.clone();
        let rows: Vec<(Action, Vec<Binding>)> = ALL_ACTIONS
            .iter()
            .map(|action| {
                let keys: Vec<Binding> =
                    self.state.bindings.keys_for(*action).into_iter().cloned().collect();
                (*action, keys)
            })
            .collect();

        let hint = match capturing.as_ref() {
            Some(target) => {
                format!(
                    "Press a key for \u{201c}{}\u{201d}, or Esc to cancel.",
                    action_label(target.action)
                )
            }
            None => match self.state.displaced {
                Some(other) => {
                    format!("Rebound. \u{201c}{}\u{201d} lost that shortcut.", action_label(other))
                }
                None => self
                    .state
                    .notice
                    .clone()
                    .unwrap_or_else(|| "Click a shortcut to change it.".to_owned()),
            },
        };

        self.settings_body().gap_0().child(
            div()
                .flex()
                .flex_col()
                .child(
                    div()
                        .pb_2()
                        .text_color(ui::color(if capturing.is_some() {
                            p.text_accent
                        } else {
                            p.text_muted
                        }))
                        .child(hint),
                )
                .children(rows.into_iter().map(|(action, keys)| {
                    let adding = capturing
                        .as_ref()
                        .is_some_and(|t| t.action == action && t.replacing.is_none());
                    let mut chips: Vec<gpui::AnyElement> = Vec::new();
                    for (index, binding) in keys.iter().enumerate() {
                        let recapturing = capturing
                            .as_ref()
                            .is_some_and(|t| t.replacing.as_ref() == Some(binding));
                        let for_click = binding.clone();
                        let for_remove = binding.clone();
                        chips.push(
                            ui::chip(("bind", action as usize * 16 + index), p, recapturing)
                                .gap_1()
                                .child(if recapturing {
                                    "Press a key\u{2026}".to_owned()
                                } else {
                                    format_binding(binding)
                                })
                                .child(
                                    div()
                                        .id(("unbind", action as usize * 16 + index))
                                        .text_color(ui::color(p.text_muted))
                                        .hover(|s| s.text_color(ui::color(p.danger)))
                                        .child("\u{00d7}")
                                        .occlude()
                                        .on_click(cx.listener(move |this, _e, _w, cx| {
                                            this.state.remove_binding(&for_remove);
                                            this.preview(cx);
                                        })),
                                )
                                .on_click(cx.listener(move |this, _e, _w, cx| {
                                    this.state.begin_recapture(action, for_click.clone());
                                    cx.notify();
                                }))
                                .into_any_element(),
                        );
                    }
                    if keys.is_empty() && !adding {
                        chips.push(
                            div()
                                .h(px(24.))
                                .flex()
                                .items_center()
                                .text_color(ui::color(p.text_muted))
                                .child("Unassigned")
                                .into_any_element(),
                        );
                    }
                    chips.push(
                        ui::chip(("add-bind", action as usize), p, adding)
                            .child(if adding { "Press a key\u{2026}" } else { "+" })
                            .on_click(cx.listener(move |this, _e, _w, cx| {
                                this.state.begin_capture(action);
                                cx.notify();
                            }))
                            .into_any_element(),
                    );

                    div()
                        .py_1()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(div().flex_grow(1.).child(action_label(action)))
                        .child(div().flex().items_center().gap_1().flex_wrap().children(chips))
                })),
        )
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

fn section_label(p: Palette, text: &'static str) -> gpui::Div {
    div().text_color(ui::color(p.text_muted)).child(text)
}

fn toggle_row(
    field: ToggleField,
    config: &reveal::config::Configuration,
    p: Palette,
    cx: &mut Context<SettingsWindow>,
) -> gpui::AnyElement {
    let value = field.get(config);
    let enabled = field.enabled(config);

    div()
        .flex()
        .items_center()
        .gap_3()
        .when(!enabled, |s| s.opacity(0.5))
        .child(
            div()
                .flex_grow(1.)
                .flex()
                .flex_col()
                .child(div().child(field.label()))
                .child(div().text_color(ui::color(p.text_muted)).child(field.description())),
        )
        .child(if enabled {
            ui::chip(("toggle", field as usize), p, value)
                .min_w(px(48.))
                .child(if value { "On" } else { "Off" })
                .on_click(cx.listener(move |this, _e, _w, cx| {
                    this.state.toggle(field);
                    this.preview(cx);
                }))
                .into_any_element()
        } else {
            ui::static_chip(p, value)
                .min_w(px(48.))
                .child(if value { "On" } else { "Off" })
                .into_any_element()
        })
        .into_any_element()
}

fn config_location_hint() -> String {
    match reveal::config::config_path() {
        Some(path) => format!("Saved to {}", path.display()),
        None => "Configuration directory unavailable.".to_owned(),
    }
}

fn associations_hint() -> &'static str {
    if cfg!(target_os = "windows") {
        "Register Reveal for all supported formats, then confirm in Windows Settings."
    } else if cfg!(target_os = "macos") {
        "Register Reveal.app with Launch Services so it appears under Open With."
    } else {
        "Claim the supported image types via xdg-mime."
    }
}
