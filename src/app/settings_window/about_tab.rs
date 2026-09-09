use gpui::{Context, IntoElement, ParentElement, Styled, div, px};
use reveal::about;
use reveal::ui::{self, Palette};

use super::SettingsWindow;
use super::widgets::{credit_row, info_row, link_chip, path_label, section_label};

impl SettingsWindow {
    pub(super) fn render_about_tab(&self, p: Palette, cx: &mut Context<Self>) -> impl IntoElement {
        self.settings_body()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .text_size(px(18.))
                            .text_color(ui::color(p.text_accent))
                            .child(about::NAME),
                    )
                    .child(div().text_color(ui::color(p.text_muted)).child(about::DESCRIPTION))
                    .child(div().child(about::version_line())),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(link_chip(
                        "about-repo",
                        p,
                        "Repository",
                        about::REPOSITORY.to_owned(),
                        cx,
                    ))
                    .child(link_chip(
                        "about-issues",
                        p,
                        "Report an issue",
                        about::issues_url(),
                        cx,
                    )),
            )
            .child(section_label(p, "Locations"))
            .child(info_row(p, "Configuration", path_label(about::config_location())))
            .child(info_row(p, "Cache", path_label(about::cache_location())))
            .child(section_label(p, "Powered by open-sourced software"))
            .children(about::DEPENDENCIES.iter().enumerate().map(|(index, dep)| {
                credit_row(
                    ("dep", index),
                    p,
                    format!("{} {}", dep.name, dep.version),
                    dep.repository,
                    dep.license,
                    cx,
                )
            }))
            .child(section_label(p, "Assets"))
            .children(about::ASSETS.iter().enumerate().map(|(index, asset)| {
                credit_row(
                    ("asset", index),
                    p,
                    format!("{} \u{2014} {}", asset.name, asset.detail),
                    asset.repository,
                    asset.license,
                    cx,
                )
            }))
            .child(div().text_color(ui::color(p.text_muted)).child(format!(
                "{} itself is distributed under {}.",
                about::NAME,
                about::LICENSE
            )))
    }
}
