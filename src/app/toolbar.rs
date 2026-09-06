use gpui::prelude::FluentBuilder;
use gpui::{Context, IntoElement, ParentElement, StatefulInteractiveElement, Styled, div};
use reveal::input::Action;
use reveal::playback::PlaybackState;
use reveal::render::FitMode;
use reveal::ui::{self, Palette};

use super::RevealApp;

impl RevealApp {
    pub fn render_toolbar(&self, p: Palette, cx: &mut Context<Self>) -> impl IntoElement {
        let has_image = self.viewer.current_path().is_some();
        let animated = self.viewer.is_animated();
        let playing = self.viewer.playback.state == PlaybackState::Playing;
        let fit = self.viewer.transform.fit;

        let action_button =
            |id: &'static str, label: &'static str, action: Action, active: bool| {
                ui::tool_button(id, p, active).child(label).on_click(cx.listener(
                    move |this, _e, window, cx| {
                        this.run(action, window, cx);
                    },
                ))
            };

        ui::toolbar(p)
            .child(action_button("tb-prev", "\u{2190}", Action::ImgPrev, false))
            .child(action_button("tb-next", "\u{2192}", Action::ImgNext, false))
            .child(ui::separator(p))
            .child(action_button("tb-zoom-out", "\u{2212}", Action::ZoomOut, false))
            .child(action_button("tb-zoom-in", "+", Action::ZoomIn, false))
            .child(action_button("tb-fit", "Fit", Action::ImgFit, fit == FitMode::Fit))
            .child(action_button(
                "tb-fit-best",
                "Fit Best",
                Action::ImgFitBest,
                fit == FitMode::FitBest,
            ))
            .child(action_button("tb-original", "1:1", Action::ImgOrig, fit == FitMode::Original))
            .children(animated.then(|| ui::separator(p)))
            .children(animated.then(|| {
                action_button(
                    "tb-play",
                    if playing { "Pause" } else { "Play" },
                    Action::PlayAnim,
                    playing,
                )
            }))
            .child(div().flex_grow(1.))
            .child(
                ui::tool_button("tb-copy", p, false)
                    .child("Copy")
                    .when(!has_image, |s| s.opacity(0.5))
                    .on_click(cx.listener(move |this, _e, window, cx| {
                        this.run(Action::ImgCopy, window, cx);
                    })),
            )
            .child(action_button(
                "tb-antialias",
                "Smooth",
                Action::ToggleAntialias,
                self.viewer.antialias(),
            ))
            .child(action_button(
                "tb-settings",
                "\u{2699}",
                Action::Settings,
                self.settings.is_some(),
            ))
    }
}
