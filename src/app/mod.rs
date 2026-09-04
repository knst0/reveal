mod actions;
mod context_menu;
pub mod labels;
mod settings_panel;
mod status_bar;
mod toolbar;
mod update_toast;

use std::time::{Duration, Instant};

use gpui::{
    App, Context, ExternalPaths, InteractiveElement, IntoElement, MouseButton, MouseDownEvent,
    ParentElement, Render, Styled, Timer, Window, div,
};
use reveal::actions::Theme;
use reveal::config::{Cache, Configuration, Updates};
use reveal::input::Bindings;
use reveal::render::{FitMode, ImageElement};
use reveal::settings::SettingsState;
use reveal::ui;
use reveal::update::UpdateNotice;
use reveal::viewer::Viewer;

use labels::title_for;

pub struct RevealApp {
    pub confirm_delete: bool,
    pub theme: Theme,
    pub bindings: Bindings,
    pub config: Configuration,
    pub settings: Option<SettingsState>,
    pub show_bottom_bar: bool,
    pub zoom_menu_open: bool,
    pub context_menu: Option<(f32, f32)>,
    pub drag_from: Option<(f32, f32)>,
    pub window_title: String,
    pub focus: gpui::FocusHandle,
    pub viewer: Viewer,
    pub cache: Cache,
    pub update_settings: Updates,
    pub update_notice: Option<UpdateNotice>,
    pub update_busy: bool,
    pub drop_hover: bool,
}

pub struct AppInit {
    pub config: Configuration,
    pub bindings: Bindings,
    pub viewer: Viewer,
    pub cache: Cache,
}

impl RevealApp {
    pub fn new(init: AppInit, cx: &mut Context<Self>) -> Self {
        let AppInit { config, bindings, viewer, cache } = init;
        let theme = Theme::from_dark(config.window.dark);
        let show_bottom_bar = config.window.show_bottom_bar;
        let update_settings = config.updates.clone();

        let mut app = Self {
            focus: cx.focus_handle(),
            drag_from: None,
            bindings,
            settings: None,
            show_bottom_bar,
            zoom_menu_open: false,
            context_menu: None,
            confirm_delete: false,
            window_title: String::new(),
            cache,
            config,
            update_settings,
            update_notice: None,
            update_busy: false,
            drop_hover: false,
            theme,
            viewer,
        };
        app.start_ticker(cx);
        app.start_update_check(cx);
        app
    }

    pub fn open_dropped(&mut self, paths: &[std::path::PathBuf]) {
        self.drop_hover = false;
        let Some(target) = reveal::drop::resolve(paths) else {
            return;
        };
        if let Err(e) = self.viewer.open(&target) {
            log::warn!("failed to open dropped {}: {e}", target.display());
        }
        self.context_menu = None;
        self.zoom_menu_open = false;
    }

    pub fn open_settings(&mut self) {
        let mut config = self.config.clone();
        config.window.dark = self.theme.is_dark();
        config.window.show_bottom_bar = self.show_bottom_bar;
        config.window.antialias = self.viewer.antialias();
        config.updates = self.update_settings.clone();
        self.settings = Some(SettingsState::new(config, self.bindings.clone()));
        self.context_menu = None;
        self.zoom_menu_open = false;
    }

    pub fn close_settings(&mut self) {
        self.settings = None;
    }

    pub fn dismiss_settings_backdrop(&mut self) {
        let Some(state) = self.settings.as_mut() else {
            return;
        };
        if state.cancel_capture() {
            return;
        }
        if state.is_dirty() {
            state.notice = Some("Unsaved changes \u{2014} use Save or Cancel.".to_owned());
            return;
        }
        self.settings = None;
    }

    pub fn save_settings(&mut self) {
        let Some(state) = self.settings.as_mut() else {
            return;
        };
        if let Err(e) = state.persist() {
            log::warn!("failed to save settings: {e}");
            state.notice = Some(format!("Could not save: {e}"));
            return;
        }
        state.notice = Some("Settings saved.".to_owned());
        let config = state.config.clone();
        let bindings = state.bindings.clone();
        self.apply_config(config, bindings);
    }

    fn apply_config(&mut self, config: Configuration, bindings: Bindings) {
        self.theme = Theme::from_dark(config.window.dark);
        self.show_bottom_bar = config.window.show_bottom_bar;
        self.viewer.set_antialias(config.window.antialias);
        self.update_settings = config.updates.clone();
        self.bindings = bindings;
        self.config = config;
    }

    pub fn compute_title(&self) -> String {
        title_for(self.viewer.current_path())
    }

    pub fn start_ticker(&self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            loop {
                Timer::after(Duration::from_millis(16)).await;
                let alive = this
                    .update(cx, |this, cx| {
                        if this.viewer.tick(Instant::now()) {
                            cx.notify();
                        }
                    })
                    .is_ok();
                if !alive {
                    break;
                }
            }
        })
        .detach();
    }
}

impl Render for RevealApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let size = window.viewport_size();
        if self.drop_hover && !cx.has_active_drag() {
            self.drop_hover = false;
        }
        self.viewer.set_viewport(f32::from(size.width), f32::from(size.height));
        self.viewer.scale_factor = window.scale_factor();

        let title = self.compute_title();
        if title != self.window_title {
            window.set_window_title(&title);
            self.window_title = title;
        }

        let p = ui::palette(self.theme);

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(ui::color(p.background))
            .text_color(ui::color(p.text))
            .track_focus(&self.focus)
            .can_drop(|dragged, _window, _cx| {
                dragged
                    .downcast_ref::<ExternalPaths>()
                    .is_some_and(|p| p.paths().iter().any(|path| reveal::drop::is_droppable(path)))
            })
            .on_drop(cx.listener(|this, paths: &ExternalPaths, _window, cx| {
                this.open_dropped(paths.paths());
                cx.notify();
            }))
            .on_drag_move(cx.listener(
                |this, event: &gpui::DragMoveEvent<ExternalPaths>, _window, cx| {
                    let hovering = event.bounds.contains(&event.event.position)
                        && event
                            .drag(cx)
                            .paths()
                            .iter()
                            .any(|path| reveal::drop::is_droppable(path));
                    if this.drop_hover != hovering {
                        this.drop_hover = hovering;
                        cx.notify();
                    }
                },
            ))
            .on_key_down(cx.listener(|this, event, window, cx| this.on_key(event, window, cx)))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event: &MouseDownEvent, _window, cx| {
                    if this.context_menu.take().is_some() || this.zoom_menu_open {
                        this.zoom_menu_open = false;
                        cx.notify();
                    }
                    this.drag_from =
                        Some((f32::from(event.position.x), f32::from(event.position.y)));
                }),
            )
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(|this, event: &MouseDownEvent, _window, cx| {
                    this.zoom_menu_open = false;
                    this.context_menu =
                        Some((f32::from(event.position.x), f32::from(event.position.y)));
                    cx.notify();
                }),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _event: &gpui::MouseUpEvent, _window, _cx| {
                    this.drag_from = None;
                }),
            )
            .on_mouse_move(cx.listener(|this, event: &gpui::MouseMoveEvent, _window, cx| {
                let Some((px_, py)) = this.drag_from else {
                    return;
                };
                let now = (f32::from(event.position.x), f32::from(event.position.y));
                this.viewer.pan((now.0 - px_, now.1 - py));
                this.drag_from = Some(now);
                cx.notify();
            }))
            .on_scroll_wheel(cx.listener(|this, event: &gpui::ScrollWheelEvent, _window, cx| {
                let delta = match event.delta {
                    gpui::ScrollDelta::Pixels(p) => f32::from(p.y) / 50.0,
                    gpui::ScrollDelta::Lines(l) => l.y,
                };
                if delta == 0.0 {
                    return;
                }
                let cursor = (f32::from(event.position.x), f32::from(event.position.y));
                this.viewer.zoom_at(1.0 + delta * 0.1, cursor);
                cx.notify();
            }))
            .child(self.render_toolbar(p, cx))
            .child(self.render_image_area(cx))
            .children(self.show_bottom_bar.then(|| self.render_status_bar(p, cx)))
            .children(self.context_menu.map(|at| self.render_context_menu(at, p, cx)))
            .children(self.settings.is_some().then(|| self.render_settings(p, cx)))
            .children(
                self.update_notice.clone().map(|notice| self.render_update_toast(notice, p, cx)),
            )
            .children(self.drop_hover.then(|| self.render_drop_overlay(p)))
    }
}

impl RevealApp {
    fn render_image_area(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex_grow()
            .relative()
            .overflow_hidden()
            .child(ImageElement::new(
                self.viewer.render_image(),
                self.viewer.current_intrinsic(),
                self.viewer.transform,
                self.viewer.frame_index(),
                self.viewer.render_crop(),
            ))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event: &MouseDownEvent, _window, cx| {
                    if event.click_count < 2 {
                        return;
                    }
                    let intrinsic = this.viewer.current_intrinsic();
                    let point = (f32::from(event.position.x), f32::from(event.position.y));
                    if this.viewer.transform.image_contains(intrinsic, this.viewer.viewport, point)
                    {
                        this.drag_from = None;
                        this.viewer.set_fit(FitMode::Fit);
                        cx.notify();
                    }
                }),
            )
    }
}

impl RevealApp {
    fn render_drop_overlay(&self, p: ui::Palette) -> impl IntoElement {
        let mut backdrop = ui::color(p.background);
        backdrop.a = 0.82;
        div().absolute().inset_0().flex().items_center().justify_center().bg(backdrop).child(
            div()
                .flex()
                .flex_col()
                .items_center()
                .gap_2()
                .px_8()
                .py_6()
                .rounded_lg()
                .border_2()
                .border_dashed()
                .border_color(ui::color(p.text_accent))
                .child(
                    div()
                        .text_size(gpui::px(16.))
                        .text_color(ui::color(p.text))
                        .child("Drop to open"),
                )
                .child(
                    div()
                        .text_size(gpui::px(12.))
                        .text_color(ui::color(p.text_muted))
                        .child("Image file or folder"),
                ),
        )
    }
}

impl gpui::Focusable for RevealApp {
    fn focus_handle(&self, _cx: &App) -> gpui::FocusHandle {
        self.focus.clone()
    }
}
