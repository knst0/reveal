use gpui::{Context, KeyDownEvent, Window};
use reveal::directory::Navigation;
use reveal::input::{Action, Modifiers};
use reveal::playback::PlaybackState;
use reveal::render::FitMode;

use super::RevealApp;

impl RevealApp {
    pub fn apply_action(
        &mut self,
        action: Action,
        centre: (f32, f32),
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if action != Action::ImgDel {
            self.confirm_delete = false;
        }
        if !matches!(action, Action::Escape) {
            self.context_menu = None;
        }
        match action {
            Action::FileOpen => self.open_via_dialog(false, cx),
            Action::FolderOpen => self.open_via_dialog(true, cx),
            Action::ImgNext => self.viewer.navigate(Navigation::Next),
            Action::ImgPrev => self.viewer.navigate(Navigation::Prev),
            Action::ImgOrig => self.viewer.set_fit(FitMode::Original),
            Action::ImgFit => self.viewer.set_fit(FitMode::Fit),
            Action::ImgFitBest => self.viewer.set_fit(FitMode::FitBest),
            Action::PanUp => self.viewer.pan((0.0, 50.0)),
            Action::PanDown => self.viewer.pan((0.0, -50.0)),
            Action::PanLeft => self.viewer.pan((50.0, 0.0)),
            Action::PanRight => self.viewer.pan((-50.0, 0.0)),
            Action::ZoomIn => self.viewer.zoom_at(1.25, centre),
            Action::ZoomOut => self.viewer.zoom_at(0.8, centre),
            Action::PlayAnim => self.viewer.toggle_play(),
            Action::PlayPresent => self.viewer.playback.set_state(PlaybackState::Present),
            Action::PlayPresentRandom => {
                self.viewer.playback.set_state(PlaybackState::PresentRandom)
            }
            Action::ToggleFullscreen => window.toggle_fullscreen(),
            Action::ImgCopy => self.copy_current(),
            Action::ImgDel => {
                if self.confirm_delete {
                    self.confirm_delete = false;
                    self.viewer.delete_current();
                } else {
                    self.confirm_delete = true;
                }
            }
            Action::ToggleAntialias => self.viewer.toggle_antialias(),
            Action::ToggleTheme => self.theme = self.theme.toggled(),
            Action::ToggleBottomBar => self.show_bottom_bar = !self.show_bottom_bar,
            Action::Settings => {
                if self.settings.is_some() {
                    self.close_settings();
                } else {
                    self.open_settings();
                }
            }
            Action::Escape => {
                if self.context_menu.is_some() {
                    self.context_menu = None;
                } else if self.zoom_menu_open {
                    self.zoom_menu_open = false;
                } else if self.settings.is_some() {
                    self.close_settings();
                } else {
                    self.viewer.playback.set_state(PlaybackState::Paused);
                }
            }
            _ => return false,
        }
        true
    }

    pub fn open_via_dialog(&mut self, folder: bool, cx: &mut Context<Self>) {
        if self.dialog_open {
            return;
        }
        self.dialog_open = true;
        let start_in = reveal::dialog::start_directory(self.viewer.current_path());
        cx.spawn(async move |this, cx| {
            let picked = if folder {
                reveal::dialog::pick_folder(start_in).await
            } else {
                reveal::dialog::pick_image(start_in).await
            };
            this.update(cx, |this, cx| {
                this.dialog_open = false;
                if let Some(picked) = picked {
                    this.open_dropped(std::slice::from_ref(&picked));
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    pub fn copy_current(&mut self) {
        if let Some(output) = self.viewer.current_output()
            && let Err(e) = reveal::actions::copy_to_clipboard(&output.decoded)
        {
            log::error!("copy failed: {e}");
        }
    }

    pub fn reveal_current(&self) {
        if let Some(path) = self.viewer.current_path()
            && let Err(e) = open::that_detached(path.parent().unwrap_or(path))
        {
            log::error!("reveal failed: {e}");
        }
    }

    pub fn on_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let modifiers = Modifiers {
            alt: event.keystroke.modifiers.alt,
            cmd_ctrl: event.keystroke.modifiers.control || event.keystroke.modifiers.platform,
            shift: event.keystroke.modifiers.shift,
        };

        if let Some(state) = self.settings.as_mut()
            && state.capturing.is_some()
        {
            if state.capture_key(&event.keystroke.key, modifiers) {
                cx.notify();
            }
            return;
        }

        let Some(action) = self.bindings.action_for(&event.keystroke.key, modifiers) else {
            return;
        };

        if self.settings.is_some() && !matches!(action, Action::Escape | Action::Settings) {
            return;
        }

        let centre = (self.viewer.viewport.0 / 2.0, self.viewer.viewport.1 / 2.0);
        if self.apply_action(action, centre, window, cx) {
            cx.notify();
        }
    }

    pub fn run(&mut self, action: Action, window: &mut Window, cx: &mut Context<Self>) {
        let centre = (self.viewer.viewport.0 / 2.0, self.viewer.viewport.1 / 2.0);
        self.apply_action(action, centre, window, cx);
        cx.notify();
    }

    pub fn set_fit_and_close_menu(&mut self, fit: FitMode) {
        self.viewer.set_fit(fit);
        self.zoom_menu_open = false;
        self.context_menu = None;
    }

    pub fn keys_for(&self, action: Action) -> Option<String> {
        self.bindings.keys_for(action).first().map(|b| super::labels::format_binding(b))
    }

    pub fn left_status(&self) -> String {
        if self.confirm_delete {
            return "Delete to trash? Press Delete again to confirm, Esc to cancel".to_owned();
        }
        if let Some(status) = self.viewer.status() {
            return status.to_owned();
        }
        match self.viewer.current_path() {
            Some(path) => path.file_name().unwrap_or_default().to_string_lossy().into_owned(),
            None => "No image".to_owned(),
        }
    }

    pub fn position_label(&self) -> Option<String> {
        self.viewer.current_path().map(|_| {
            format!(
                "{} of {}",
                self.viewer.directory.current_index() + 1,
                self.viewer.directory.len().max(1)
            )
        })
    }

    pub fn dimensions_label(&self) -> Option<String> {
        let (w, h) = self.viewer.current_source_size();
        (w > 0 && h > 0).then(|| format!("{w} \u{00d7} {h}"))
    }
}
