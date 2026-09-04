use gpui::prelude::FluentBuilder;
use gpui::{
    Context, InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement, Styled,
    div, px,
};
use reveal::config::Channel;
use reveal::input::{ALL_ACTIONS, Action, Binding};
use reveal::settings::{APPEARANCE_FIELDS, SETTINGS_TABS, SettingsTab, ToggleField, UPDATE_FIELDS};
use reveal::ui::{self, Palette};

use super::RevealApp;
use super::labels::{action_label, format_binding};

const PANEL_WIDTH: f32 = 560.0;
const PANEL_HEIGHT: f32 = 460.0;

impl RevealApp {
    pub fn render_settings(&self, p: Palette, cx: &mut Context<Self>) -> gpui::AnyElement {
        let Some(state) = self.settings.as_ref() else {
            return div().into_any_element();
        };
        let tab = state.tab;
        let dirty = state.is_dirty();

        div()
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(gpui::rgba(0x00000066))
            .id("settings-backdrop")
            .on_click(cx.listener(|this, _e, _window, cx| {
                this.dismiss_settings_backdrop();
                cx.notify();
            }))
            .child(
                ui::overlay_panel(p)
                    .occlude()
                    .w(px(PANEL_WIDTH))
                    .h(px(PANEL_HEIGHT))
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .child(ui::panel_header(p, "Settings"))
                    .child(self.render_settings_tabs(tab, p, cx))
                    .child(match tab {
                        SettingsTab::General => self.render_general_tab(p, cx).into_any_element(),
                        SettingsTab::Keys => self.render_keys_tab(p, cx).into_any_element(),
                    })
                    .child(self.render_settings_footer(dirty, p, cx)),
            )
            .into_any_element()
    }

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
                    if let Some(state) = this.settings.as_mut() {
                        state.select_tab(tab);
                    }
                    cx.notify();
                }))
            }))
    }

    fn settings_body(&self) -> gpui::Stateful<gpui::Div> {
        div().id("settings-body").flex_grow().flex().flex_col().p_3().gap_3().overflow_y_scroll()
    }

    fn render_general_tab(&self, p: Palette, cx: &mut Context<Self>) -> impl IntoElement {
        let state = self.settings.as_ref().expect("settings open");
        let config = state.config.clone();
        let channel = config.updates.channel;
        let notice = state.notice.clone();

        let mut appearance = Vec::new();
        for field in APPEARANCE_FIELDS {
            appearance.push(toggle_row(*field, config.clone(), p, cx));
        }
        let mut updates = Vec::new();
        for field in UPDATE_FIELDS {
            updates.push(toggle_row(*field, config.clone(), p, cx));
        }

        self.settings_body()
            .children(notice.map(|text| {
                div().text_size(px(11.)).text_color(ui::color(p.text_accent)).child(text)
            }))
            .child(section_label(p, "Appearance"))
            .children(appearance)
            .child(section_label(p, "Updates"))
            .children(updates)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(div().flex_grow().text_size(px(11.)).child("Release channel"))
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
                            if let Some(state) = this.settings.as_mut() {
                                state.set_channel(option);
                            }
                            cx.notify();
                        }))
                    })),
            )
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(ui::color(p.text_muted))
                    .child(config_location_hint()),
            )
    }

    fn render_keys_tab(&self, p: Palette, cx: &mut Context<Self>) -> impl IntoElement {
        let state = self.settings.as_ref().expect("settings open");
        let capturing = state.capturing.clone();
        let rows: Vec<(Action, Vec<Binding>)> = ALL_ACTIONS
            .iter()
            .map(|action| {
                let keys: Vec<Binding> =
                    state.bindings.keys_for(*action).into_iter().cloned().collect();
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
            None => match state.displaced {
                Some(other) => {
                    format!("Rebound. \u{201c}{}\u{201d} lost that shortcut.", action_label(other))
                }
                None => state
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
                        .text_size(px(11.))
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
                                            if let Some(state) = this.settings.as_mut() {
                                                state.remove_binding(&for_remove);
                                            }
                                            cx.notify();
                                        })),
                                )
                                .on_click(cx.listener(move |this, _e, _w, cx| {
                                    if let Some(state) = this.settings.as_mut() {
                                        state.begin_recapture(action, for_click.clone());
                                    }
                                    cx.notify();
                                }))
                                .into_any_element(),
                        );
                    }
                    if keys.is_empty() && !adding {
                        chips.push(
                            div()
                                .h(px(20.))
                                .flex()
                                .items_center()
                                .text_size(px(11.))
                                .text_color(ui::color(p.text_muted))
                                .child("Unassigned")
                                .into_any_element(),
                        );
                    }
                    chips.push(
                        ui::chip(("add-bind", action as usize), p, adding)
                            .child(if adding { "Press a key\u{2026}" } else { "+" })
                            .on_click(cx.listener(move |this, _e, _w, cx| {
                                if let Some(state) = this.settings.as_mut() {
                                    state.begin_capture(action);
                                }
                                cx.notify();
                            }))
                            .into_any_element(),
                    );

                    div()
                        .py_1()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(div().flex_grow().text_size(px(11.)).child(action_label(action)))
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
            .px_3()
            .py_2()
            .border_t_1()
            .border_color(ui::color(p.border_variant))
            .child(ui::toast_button("settings-reset", p, "Reset shortcuts", false).on_click(
                cx.listener(|this, _e, _w, cx| {
                    if let Some(state) = this.settings.as_mut() {
                        state.reset_bindings();
                    }
                    cx.notify();
                }),
            ))
            .child(div().flex_grow())
            .child(ui::toast_button("settings-cancel", p, "Cancel", false).on_click(cx.listener(
                |this, _e, _w, cx| {
                    this.close_settings();
                    cx.notify();
                },
            )))
            .child(
                ui::toast_button("settings-save", p, "Save", true)
                    .when(!dirty, |s| s.opacity(0.5))
                    .on_click(cx.listener(|this, _e, _w, cx| {
                        this.save_settings();
                        cx.notify();
                    })),
            )
    }
}

fn section_label(p: Palette, text: &'static str) -> gpui::Div {
    div().text_size(px(11.)).text_color(ui::color(p.text_muted)).child(text)
}

fn toggle_row(
    field: ToggleField,
    config: reveal::config::Configuration,
    p: Palette,
    cx: &mut Context<RevealApp>,
) -> gpui::AnyElement {
    let value = field.get(&config);
    let enabled = field.enabled(&config);

    div()
        .flex()
        .items_center()
        .gap_3()
        .when(!enabled, |s| s.opacity(0.5))
        .child(
            div()
                .flex_grow()
                .flex()
                .flex_col()
                .child(div().text_size(px(11.)).child(field.label()))
                .child(
                    div()
                        .text_size(px(11.))
                        .text_color(ui::color(p.text_muted))
                        .child(field.description()),
                ),
        )
        .child(if enabled {
            ui::chip(("toggle", field as usize), p, value)
                .min_w(px(48.))
                .child(if value { "On" } else { "Off" })
                .on_click(cx.listener(move |this, _e, _w, cx| {
                    if let Some(state) = this.settings.as_mut() {
                        state.toggle(field);
                    }
                    cx.notify();
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
