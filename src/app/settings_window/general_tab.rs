use gpui::prelude::FluentBuilder;
use gpui::{
    AppContext, Context, IntoElement, ParentElement, StatefulInteractiveElement, Styled, div, px,
};
use reveal::config::{Channel, UI_SCALES};
use reveal::settings::{APPEARANCE_FIELDS, ToggleField, UPDATE_FIELDS};
use reveal::ui::{self, Palette};

use super::SettingsWindow;
use super::widgets::section_label;

impl SettingsWindow {
    pub(super) fn render_general_tab(
        &self,
        p: Palette,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
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
    }
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

fn associations_hint() -> &'static str {
    if cfg!(target_os = "windows") {
        "Register Reveal for all supported formats, then confirm in Windows Settings."
    } else if cfg!(target_os = "macos") {
        "Register Reveal.app with Launch Services so it appears under Open With."
    } else {
        "Claim the supported image types via xdg-mime."
    }
}
