use gpui::{
    Context, InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement, anchored,
    point, px,
};
use reveal::input::Action;
use reveal::ui::{self, MenuItem, Palette};

use super::labels::action_label;

use super::RevealApp;

impl RevealApp {
    pub fn render_context_menu(
        &self,
        at: (f32, f32),
        p: Palette,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let viewer = self.active_viewer();
        let has_image = viewer.has_image();
        let has_file = viewer.current_file().is_some();
        let animated = viewer.is_animated();
        let playing = viewer.is_playing();
        let comparing = self.layout().is_some();

        let item = |id: &'static str, action: Action| {
            MenuItem::new(id, action_label(action))
                .keybinding(self.keys_for(action))
                .disabled(!has_image)
                .render(p)
                .on_click(cx.listener(move |this, _e, window, cx| {
                    this.context_menu = None;
                    this.run(action, window, cx);
                }))
        };

        let item_always = |id: &'static str, action: Action| {
            MenuItem::new(id, action_label(action))
                .keybinding(self.keys_for(action))
                .render(p)
                .on_click(cx.listener(move |this, _e, window, cx| {
                    this.context_menu = None;
                    this.run(action, window, cx);
                }))
        };
        // Flips/snaps so the menu never leaves the window near an edge.
        anchored().position(point(px(at.0), px(at.1))).snap_to_window_with_margin(px(4.)).child(
            ui::menu_surface(p)
                .occlude()
                .child(item_always("cm-open-file", Action::FileOpen))
                .child(item_always("cm-open-folder", Action::FolderOpen))
                .child(ui::menu_separator(p))
                .child(item("cm-next", Action::ImgNext))
                .child(item("cm-prev", Action::ImgPrev))
                .child(ui::menu_separator(p))
                .child(item("cm-fit", Action::ImgFit))
                .child(item("cm-fit-best", Action::ImgFitBest))
                .child(item("cm-orig", Action::ImgOrig))
                .child(item("cm-zoom-in", Action::ZoomIn))
                .child(item("cm-zoom-out", Action::ZoomOut))
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
                .child(item_always("cm-compare-tiles", Action::CompareTiles))
                .child(item_always("cm-compare-slider", Action::CompareSlider))
                .children(comparing.then(|| item_always("cm-compare-flip", Action::CompareFlip)))
                .children(comparing.then(|| item_always("cm-compare-align", Action::CompareAlign)))
                .children(
                    comparing
                        .then(|| item_always("cm-compare-align-px", Action::CompareAlignPixels)),
                )
                .child(ui::menu_separator(p))
                .child(item("cm-copy", Action::ImgCopy))
                .child(
                    MenuItem::new("cm-reveal", "Reveal in Folder")
                        .disabled(!has_file)
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
                    MenuItem::new("cm-settings", "Settings\u{2026}")
                        .keybinding(self.keys_for(Action::Settings))
                        .render(p)
                        .on_click(cx.listener(|this, _e, _window, cx| {
                            this.open_settings(cx);
                            cx.notify();
                        })),
                )
                .child(ui::menu_separator(p))
                .child(
                    MenuItem::new("cm-delete", "Move to Trash")
                        .keybinding(self.keys_for(Action::ImgDel))
                        .disabled(!has_file)
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
