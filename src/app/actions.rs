use gpui::{Context, KeyDownEvent, Window};
use reveal::compare::{Align, Layout};
use reveal::directory::Navigation;
use reveal::input::{Action, Modifiers};
use reveal::playback::PlaybackState;
use reveal::render::FitMode;
use reveal::viewer::Viewer;

use super::RevealApp;
use super::compare::per_pane;

fn apply_to_pane(viewer: &mut Viewer, action: Action) {
    let centre = (viewer.view.viewport.0 / 2.0, viewer.view.viewport.1 / 2.0);
    match action {
        Action::ImgNext => viewer.navigate(Navigation::Next),
        Action::ImgPrev => viewer.navigate(Navigation::Prev),
        Action::ImgOrig => viewer.set_fit(FitMode::Original),
        Action::ImgFit => viewer.set_fit(FitMode::Fit),
        Action::ImgFitBest => viewer.set_fit(FitMode::FitBest),
        Action::PanUp => viewer.view.pan((0.0, 50.0)),
        Action::PanDown => viewer.view.pan((0.0, -50.0)),
        Action::PanLeft => viewer.view.pan((50.0, 0.0)),
        Action::PanRight => viewer.view.pan((-50.0, 0.0)),
        Action::ZoomIn => viewer.view.zoom_at(1.25, centre),
        Action::ZoomOut => viewer.view.zoom_at(0.8, centre),
        Action::PlayAnim => viewer.toggle_play(),
        Action::PlayPresent => viewer.playback.set_state(PlaybackState::Present),
        Action::PlayPresentRandom => viewer.playback.set_state(PlaybackState::PresentRandom),
        _ => {}
    }
}

impl RevealApp {
    /// Runs `action`. Pane actions go to `only` that pane, or to every
    /// visible pane; file actions go to the active pane.
    pub fn apply_action(
        &mut self,
        action: Action,
        only: Option<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if action != Action::ImgDel {
            self.confirm_delete = None;
        }
        if !matches!(action, Action::Escape) {
            self.context_menu = None;
        }
        if per_pane(action) {
            for pane in self.targets(only) {
                apply_to_pane(self.pane_mut(pane), action);
            }
            return true;
        }
        match action {
            Action::FileOpen => self.open_via_dialog(false, cx),
            Action::FolderOpen => self.open_via_dialog(true, cx),
            Action::ToggleFullscreen => window.toggle_fullscreen(),
            Action::ImgCopy => self.copy_current(),
            Action::ImgPaste => self.paste_image(),
            Action::ImgDel => {
                if self.delete_confirmed() {
                    self.confirm_delete = None;
                    self.active_viewer_mut().delete_current();
                } else {
                    self.confirm_delete = self
                        .active_viewer()
                        .presentation
                        .current_path()
                        .map(std::path::Path::to_path_buf);
                }
            }
            Action::ToggleAntialias => {
                let antialias = !self.viewer.view.antialias();
                for pane in 0..self.existing_panes() {
                    self.pane_mut(pane).set_antialias(antialias);
                }
            }
            Action::ToggleTheme => self.theme = self.theme.toggled(),
            Action::CompareTiles => self.toggle_compare(Layout::Tiles, cx),
            Action::CompareSlider => self.toggle_compare(Layout::Slider, cx),
            Action::CompareFlip => self.flip_compare(),
            Action::CompareAlign => self.align_compare(Align::Frame),
            Action::CompareAlignPixels => self.align_compare(Align::Pixels),
            Action::Settings => {
                if self.settings_window.is_some() {
                    self.close_settings(cx);
                } else {
                    self.open_settings(cx);
                }
            }
            Action::Escape => {
                if self.context_menu.is_some() {
                    self.context_menu = None;
                } else if self.zoom_menu_open {
                    self.zoom_menu_open = false;
                } else {
                    for pane in self.targets(None) {
                        self.pane_mut(pane).playback.set_state(PlaybackState::Paused);
                    }
                }
            }
            _ => {}
        }
        true
    }

    pub fn open_via_dialog(&mut self, folder: bool, cx: &mut Context<Self>) {
        if self.dialog_open {
            return;
        }
        self.dialog_open = true;
        let pane = self.active_pane();
        let start_in =
            reveal::dialog::start_directory(self.active_viewer().presentation.current_path());
        cx.spawn(async move |this, cx| {
            let picked = if folder {
                reveal::dialog::pick_folder(start_in).await
            } else {
                reveal::dialog::pick_image(start_in).await
            };
            this.update(cx, |this, cx| {
                this.dialog_open = false;
                if let Some(picked) = picked {
                    if let Some(c) = this.compare.as_mut() {
                        c.active = pane;
                    }
                    this.open_dropped(std::slice::from_ref(&picked));
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    pub fn copy_current(&mut self) {
        if let Some(output) = self.active_viewer().presentation.current_output()
            && let Err(e) = reveal::actions::copy_to_clipboard(&output.decoded, output.orientation)
        {
            log::error!("copy failed: {e}");
        }
    }

    pub fn paste_image(&mut self) {
        match reveal::actions::paste_from_clipboard() {
            Ok(image) => self.active_viewer_mut().show_pasted(image),
            Err(e) => log::warn!("paste failed: {e}"),
        }
    }

    pub fn reveal_current(&self) {
        if let Some(path) = self.active_viewer().presentation.current_path()
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

        let key = &event.keystroke.key;
        let (action, only) = match self.bindings.action_for(key, modifiers) {
            Some(action) => (action, None),
            // In compare, Ctrl + a pane key acts on the active pane only.
            None if modifiers.cmd_ctrl && self.layout().is_some() => {
                let plain = Modifiers { cmd_ctrl: false, ..modifiers };
                match self.bindings.action_for(key, plain) {
                    Some(action) if per_pane(action) => (action, Some(self.active_pane())),
                    _ => return,
                }
            }
            None => return,
        };
        if self.apply_action(action, only, window, cx) {
            cx.notify();
        }
    }

    pub fn run(&mut self, action: Action, window: &mut Window, cx: &mut Context<Self>) {
        self.apply_action(action, None, window, cx);
        cx.notify();
    }

    pub fn set_fit_and_close_menu(&mut self, fit: FitMode) {
        for pane in self.targets(None) {
            self.pane_mut(pane).set_fit(fit);
        }
        self.zoom_menu_open = false;
        self.context_menu = None;
    }

    pub fn keys_for(&self, action: Action) -> Option<String> {
        self.bindings.keys_for(action).first().map(|b| super::labels::format_binding(b))
    }

    pub fn delete_confirmed(&self) -> bool {
        match (self.confirm_delete.as_deref(), self.active_viewer().presentation.current_path()) {
            (Some(pending), Some(current)) => pending == current,
            _ => false,
        }
    }

    pub fn left_status(&self) -> String {
        if self.delete_confirmed() {
            return "Delete to trash? Press Delete again to confirm, Esc to cancel".to_owned();
        }
        let viewer = self.active_viewer();
        if let Some(status) = viewer.session.status() {
            return status.to_owned();
        }
        match viewer.presentation.current_path() {
            Some(path) => path.file_name().unwrap_or_default().to_string_lossy().into_owned(),
            None => "No image".to_owned(),
        }
    }

    pub fn position_label(&self) -> Option<String> {
        let viewer = self.active_viewer();
        viewer.presentation.current_path().map(|_| {
            format!(
                "{} of {}",
                viewer.session.directory.current_index() + 1,
                viewer.session.directory.len().max(1)
            )
        })
    }

    pub fn dimensions_label(&self) -> Option<String> {
        let (w, h) = self.active_viewer().presentation.current_source_size();
        (w > 0 && h > 0).then(|| format!("{w} \u{00d7} {h}"))
    }
}
