use gpui::{
    Context, InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement, Styled,
    div, px,
};
use reveal::input::Action;
use reveal::playback::PlaybackState;
use reveal::ui::{self, MenuItem, Palette};

use super::RevealApp;

impl RevealApp {
    pub fn render_context_menu(
        &self,
        at: (f32, f32),
        p: Palette,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let has_image = self.viewer.current_path().is_some();
        let animated = self.viewer.is_animated();
        let playing = self.viewer.playback.state == PlaybackState::Playing;

        let item = |id: &'static str, label: &'static str, action: Action| {
            MenuItem::new(id, label)
                .keybinding(self.keys_for(action))
                .disabled(!has_image)
                .render(p)
                .on_click(cx.listener(move |this, _e, window, cx| {
                    this.context_menu = None;
                    this.run(action, window, cx);
                }))
        };

        div().absolute().left(px(at.0)).top(px(at.1)).child(
            ui::menu_surface(p)
                .occlude()
                .child(item("cm-next", "Next Image", Action::ImgNext))
                .child(item("cm-prev", "Previous Image", Action::ImgPrev))
                .child(ui::menu_separator(p))
                .child(item("cm-fit", "Fit to Window", Action::ImgFit))
                .child(item("cm-fit-best", "Fit Best", Action::ImgFitBest))
                .child(item("cm-orig", "Original Size", Action::ImgOrig))
                .child(item("cm-zoom-in", "Zoom In", Action::ZoomIn))
                .child(item("cm-zoom-out", "Zoom Out", Action::ZoomOut))
                .children(animated.then(|| ui::menu_separator(p)))
                .children(animated.then(|| {
                    MenuItem::new(
                        "cm-play",
                        if playing { "Pause Animation" } else { "Play Animation" },
                    )
                    .keybinding(self.keys_for(Action::PlayAnim))
                    .render(p)
                    .on_click(cx.listener(move |this, _e, window, cx| {
                        this.context_menu = None;
                        this.run(Action::PlayAnim, window, cx);
                    }))
                }))
                .child(ui::menu_separator(p))
                .child(item("cm-copy", "Copy Image", Action::ImgCopy))
                .child(
                    MenuItem::new("cm-reveal", "Reveal in Folder")
                        .disabled(!has_image)
                        .render(p)
                        .on_click(cx.listener(|this, _e, _window, cx| {
                            this.context_menu = None;
                            this.reveal_current();
                            cx.notify();
                        })),
                )
                .child(ui::menu_separator(p))
                .child(
                    MenuItem::new("cm-fullscreen", "Toggle Fullscreen")
                        .keybinding(self.keys_for(Action::ToggleFullscreen))
                        .render(p)
                        .on_click(cx.listener(move |this, _e, window, cx| {
                            this.context_menu = None;
                            this.run(Action::ToggleFullscreen, window, cx);
                        })),
                )
                .child(
                    MenuItem::new("cm-theme", "Toggle Theme")
                        .keybinding(self.keys_for(Action::ToggleTheme))
                        .render(p)
                        .on_click(cx.listener(move |this, _e, window, cx| {
                            this.context_menu = None;
                            this.run(Action::ToggleTheme, window, cx);
                        })),
                )
                .child(
                    MenuItem::new("cm-status", "Toggle Status Bar")
                        .keybinding(self.keys_for(Action::ToggleBottomBar))
                        .render(p)
                        .on_click(cx.listener(move |this, _e, window, cx| {
                            this.context_menu = None;
                            this.run(Action::ToggleBottomBar, window, cx);
                        })),
                )
                .child(
                    MenuItem::new("cm-settings", "Settings\u{2026}")
                        .keybinding(self.keys_for(Action::Settings))
                        .render(p)
                        .on_click(cx.listener(|this, _e, _window, cx| {
                            this.open_settings();
                            cx.notify();
                        })),
                )
                .child(ui::menu_separator(p))
                .child(
                    MenuItem::new("cm-delete", "Move to Trash")
                        .keybinding(self.keys_for(Action::ImgDel))
                        .disabled(!has_image)
                        .danger(true)
                        .render(p)
                        .on_click(cx.listener(move |this, _e, window, cx| {
                            this.context_menu = None;
                            this.run(Action::ImgDel, window, cx);
                        })),
                ),
        )
    }
}
