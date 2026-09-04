use gpui::{
    AppContext, Context, InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement,
};
use reveal::ui::{self, Palette};
use reveal::update::{self, UpdateNotice, UpdateStatus};

use super::RevealApp;

impl RevealApp {
    pub fn save_cache(&self) {
        if let Err(e) = self.cache.save() {
            log::warn!("failed to save cache: {e}");
        }
    }

    pub fn start_update_check(&mut self, cx: &mut Context<Self>) {
        if let Some(notice) = update::take_upgrade_notice(&mut self.cache) {
            self.update_notice = Some(notice);
        }
        self.save_cache();

        if !update::should_run_check(&self.update_settings, &self.cache) {
            return;
        }

        let channel = self.update_settings.channel;
        let auto = self.update_settings.should_auto_install();
        cx.spawn(async move |this, cx| {
            let status = cx
                .background_spawn(async move {
                    if auto { update::install(channel) } else { update::check(channel) }
                })
                .await;

            this.update(cx, |this, cx| {
                update::record_check(&mut this.cache);
                match status {
                    UpdateStatus::Available { version } => {
                        if !update::is_skipped(&this.cache, &version) {
                            this.update_notice = Some(UpdateNotice::Available { version });
                        }
                    }
                    UpdateStatus::Installed { version } => {
                        this.cache.update_pending_version = Some(version.clone());
                        this.update_notice = Some(UpdateNotice::Installed { version });
                    }
                    UpdateStatus::Failed { error } => log::warn!("update check failed: {error}"),
                    UpdateStatus::UpToDate | UpdateStatus::Disabled => {}
                }
                this.save_cache();
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn install_update_now(&mut self, cx: &mut Context<Self>) {
        if self.update_busy {
            return;
        }
        let channel = self.update_settings.channel;
        self.update_busy = true;
        cx.notify();

        cx.spawn(async move |this, cx| {
            let status = cx.background_spawn(async move { update::install(channel) }).await;
            this.update(cx, |this, cx| {
                this.update_busy = false;
                match status {
                    UpdateStatus::Installed { version } => {
                        this.cache.update_pending_version = Some(version.clone());
                        this.update_notice = Some(UpdateNotice::Installed { version });
                        this.save_cache();
                    }
                    UpdateStatus::Failed { error } => {
                        log::warn!("update failed: {error}");
                        this.update_notice = None;
                    }
                    _ => this.update_notice = None,
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn dismiss_update_notice(&mut self) {
        self.update_notice = None;
    }

    fn disable_update_checks(&mut self) {
        self.update_settings.check = false;
        self.config.updates.check = false;
        if let Some(UpdateNotice::Available { version }) = &self.update_notice {
            let version = version.clone();
            update::skip_version(&mut self.cache, &version);
        }
        self.save_cache();
        self.update_notice = None;
    }

    pub fn render_update_toast(
        &self,
        notice: UpdateNotice,
        p: Palette,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let body = match &notice {
            UpdateNotice::Available { version } => ui::toast(p)
                .child(ui::toast_title(p, "Update available"))
                .child(ui::toast_body(
                    p,
                    format!(
                        "Version {version} is ready to install ({} channel).",
                        self.update_settings.channel.label().to_lowercase()
                    ),
                ))
                .child(
                    ui::toast_actions()
                        .child(
                            ui::toast_button("update-never", p, "Don\u{2019}t show again", false)
                                .on_click(cx.listener(|this, _e, _w, cx| {
                                    this.disable_update_checks();
                                    cx.notify();
                                })),
                        )
                        .child(ui::toast_button("update-close", p, "Close", false).on_click(
                            cx.listener(|this, _e, _w, cx| {
                                this.dismiss_update_notice();
                                cx.notify();
                            }),
                        ))
                        .child(
                            ui::toast_button(
                                "update-now",
                                p,
                                if self.update_busy { "Updating\u{2026}" } else { "Update" },
                                true,
                            )
                            .on_click(cx.listener(
                                |this, _e, _w, cx| {
                                    this.install_update_now(cx);
                                },
                            )),
                        ),
                ),
            UpdateNotice::Installed { version } => ui::toast(p)
                .child(ui::toast_title(p, "Update installed"))
                .child(ui::toast_body(
                    p,
                    format!("Version {version} will be used the next time you start Reveal."),
                ))
                .child(ui::toast_actions().child(
                    ui::toast_button("update-ok", p, "OK", true).on_click(cx.listener(
                        |this, _e, _w, cx| {
                            this.dismiss_update_notice();
                            cx.notify();
                        },
                    )),
                )),
            UpdateNotice::Upgraded { from, to } => {
                let summary = match from {
                    Some(from) => format!("Updated from {from} to {to}."),
                    None => format!("Updated to {to}."),
                };
                let url = update::changelog_url(to);
                ui::toast(p)
                    .child(ui::toast_title(p, format!("Welcome to Reveal {to}")))
                    .child(ui::toast_body(p, summary))
                    .child(
                        ui::toast_actions()
                            .child(ui::toast_button("changelog-close", p, "Close", false).on_click(
                                cx.listener(|this, _e, _w, cx| {
                                    this.dismiss_update_notice();
                                    cx.notify();
                                }),
                            ))
                            .child(
                                ui::toast_button("changelog-open", p, "What\u{2019}s new", true)
                                    .on_click(cx.listener(move |this, _e, _w, cx| {
                                        if let Err(e) = open::that_detached(&url) {
                                            log::warn!("failed to open changelog: {e}");
                                        }
                                        this.dismiss_update_notice();
                                        cx.notify();
                                    })),
                            ),
                    )
            }
        };

        ui::toast_container().child(body.occlude())
    }
}
