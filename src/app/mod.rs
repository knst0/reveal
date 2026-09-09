mod actions;
mod context_menu;
pub mod labels;
mod settings_window;
mod status_bar;
pub mod titlebar;
mod toolbar;
mod update_toast;

use std::time::{Duration, Instant};

use futures::StreamExt;
use gpui::{
    App, AppContext, Bounds, Context, ExternalPaths, InteractiveElement, IntoElement, MouseButton,
    MouseDownEvent, ParentElement, Render, Styled, Subscription, Window, WindowBounds,
    WindowDecorations, WindowHandle, WindowOptions, div, px, size,
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
use settings_window::{SettingsWindow, WINDOW_HEIGHT, WINDOW_WIDTH};

enum Wake {
    Cache(reveal::cache::LoadResult),
    Scan(u64, std::io::Result<reveal::directory::Directory>),
    DropPoll,
    Refresh,
}

pub struct RevealApp {
    pub confirm_delete: Option<std::path::PathBuf>,
    pub theme: Theme,
    pub bindings: Bindings,
    pub config: Configuration,
    pub settings_window: Option<WindowHandle<SettingsWindow>>,
    settings_release: Option<Subscription>,
    settings_baseline: Option<(Configuration, Bindings)>,
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
    pub dialog_open: bool,
    wake: Option<futures::channel::mpsc::UnboundedSender<Wake>>,
    resize_generation: u64,
    resize_settling: bool,
    timer_active: bool,
    scan_forwarded: u64,
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
        let update_settings = config.updates.clone();

        let mut app = Self {
            focus: cx.focus_handle(),
            drag_from: None,
            bindings,
            settings_window: None,
            settings_release: None,
            settings_baseline: None,
            zoom_menu_open: false,
            context_menu: None,
            confirm_delete: None,
            window_title: String::new(),
            cache,
            config,
            update_settings,
            update_notice: None,
            update_busy: false,
            drop_hover: false,
            dialog_open: false,
            wake: None,
            resize_generation: 0,
            resize_settling: false,
            timer_active: false,
            scan_forwarded: 0,
            theme,
            viewer,
        };
        app.start_ticker(cx);
        app.start_update_check(cx);
        app
    }

    pub fn open_dropped(&mut self, paths: &[std::path::PathBuf]) {
        self.drop_hover = false;
        let Some(target) = reveal::drop::target(paths) else {
            return;
        };
        if let Err(e) = self.viewer.open(&target) {
            log::warn!("failed to open dropped {}: {e}", target.display());
        }
        self.context_menu = None;
        self.zoom_menu_open = false;
    }

    pub fn open_settings(&mut self, cx: &mut Context<Self>) {
        self.context_menu = None;
        self.zoom_menu_open = false;

        if let Some(handle) = self.settings_window.as_ref() {
            handle.update(cx, |_, window, _| window.activate_window()).ok();
            return;
        }

        let mut config = self.config.clone();
        config.window.dark = self.theme.is_dark();
        config.window.antialias = self.viewer.antialias();
        config.updates = self.update_settings.clone();
        self.settings_baseline = Some((config.clone(), self.bindings.clone()));
        let state = SettingsState::new(config, self.bindings.clone());

        let theme = self.theme;
        let owner = cx.weak_entity();
        let bounds = Bounds::centered(None, size(px(WINDOW_WIDTH), px(WINDOW_HEIGHT)), cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(titlebar::titlebar_options("Settings")),
            window_decorations: Some(WindowDecorations::Client),
            app_owns_titlebar_drag: true,
            window_min_size: Some(size(px(WINDOW_WIDTH), px(360.))),
            ..Default::default()
        };

        let handle = match cx.open_window(options, |window, cx| {
            let view = cx.new(|cx| SettingsWindow::new(state, theme, owner, cx));
            window.focus(&view.read(cx).focus.clone(), cx);
            view
        }) {
            Ok(handle) => handle,
            Err(e) => {
                log::warn!("failed to open settings window: {e}");
                return;
            }
        };

        if let Ok(view) = handle.entity(cx) {
            self.settings_release = Some(cx.observe_release(&view, |this, _, cx| {
                this.settings_window = None;
                if let Some((config, bindings)) = this.settings_baseline.take() {
                    this.apply_config(config, bindings);
                    cx.notify();
                }
            }));
        }
        self.settings_window = Some(handle);
    }

    pub fn close_settings(&mut self, cx: &mut Context<Self>) {
        if let Some(handle) = self.settings_window.take() {
            handle.update(cx, |_, window, _| window.remove_window()).ok();
        }
        self.settings_release = None;
    }

    pub fn apply_config(&mut self, config: Configuration, bindings: Bindings) {
        self.theme = Theme::from_dark(config.window.dark);
        self.viewer.set_antialias(config.window.antialias);
        self.update_settings = config.updates.clone();
        self.bindings = bindings;
        self.config = config;
    }

    pub fn compute_title(&self) -> String {
        title_for(self.viewer.current_path())
    }

    pub fn start_ticker(&mut self, cx: &mut Context<Self>) {
        let (tx, mut rx) = futures::channel::mpsc::unbounded::<Wake>();
        self.wake = Some(tx.clone());

        {
            let tx = tx.clone();
            cx.background_executor()
                .spawn_dedicated(move |_| async move {
                    loop {
                        std::thread::sleep(Duration::from_millis(250));
                        if tx.unbounded_send(Wake::DropPoll).is_err() {
                            break;
                        }
                    }
                })
                .detach();
        }

        {
            let loader = self.viewer.cache.loader_handle();
            let tx = tx.clone();
            cx.background_executor()
                .spawn_dedicated(move |_| async move {
                    while let Some(result) = loader.recv() {
                        if tx.unbounded_send(Wake::Cache(result)).is_err() {
                            break;
                        }
                    }
                })
                .detach();
        }

        self.sync_scan_forwarder(cx);

        cx.spawn(async move |this, cx| {
            loop {
                let timeout =
                    this.update(cx, |this, _| this.viewer.next_time_based_delay()).ok().flatten();
                let fired = match timeout {
                    Some(duration) => {
                        let next = rx.next();
                        let timer = cx.background_executor().timer(duration);
                        futures::pin_mut!(next, timer);
                        match futures::future::select(next, timer).await {
                            futures::future::Either::Left((wake, _)) => wake,
                            futures::future::Either::Right((_, _)) => None,
                        }
                    }
                    None => rx.next().await,
                };
                let alive = this
                    .update(cx, |this, cx| match fired {
                        Some(wake) => this.handle_wake(wake, cx),
                        None => {
                            if this.viewer.tick_playback(Instant::now()) {
                                cx.notify();
                            }
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

    fn handle_wake(&mut self, wake: Wake, cx: &mut Context<Self>) {
        match wake {
            Wake::Cache(result) => {
                if self.viewer.apply_load(result) {
                    cx.notify();
                }
            }
            Wake::Scan(generation, result) => {
                if self.viewer.apply_scan(generation, result) {
                    cx.notify();
                }
            }
            Wake::DropPoll => {
                let requested = reveal::drop::take_requested();
                if !requested.is_empty() {
                    self.open_dropped(&requested);
                    cx.notify();
                }
            }
            Wake::Refresh => {
                if self.viewer.reprepare_if_pending() {
                    cx.notify();
                }
            }
        }
        if self.viewer.tick_playback(Instant::now()) {
            cx.notify();
        }
    }

    fn wake(&self) {
        if let Some(tx) = self.wake.as_ref() {
            let _ = tx.unbounded_send(Wake::Refresh);
        }
    }

    fn schedule_reprepare(&mut self, cx: &mut Context<Self>) {
        self.resize_generation = self.resize_generation.wrapping_add(1);
        if self.resize_settling {
            return;
        }
        self.resize_settling = true;
        let mut seen = self.resize_generation;
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(Duration::from_millis(120)).await;
                let current = this.update(cx, |this, _| this.resize_generation).unwrap_or(seen);
                if current == seen {
                    break;
                }
                seen = current;
            }
            this.update(cx, |this, _| {
                this.resize_settling = false;
                this.wake();
            })
            .ok();
        })
        .detach();
    }

    fn start_scan_forwarder(&mut self, cx: &mut Context<Self>) {
        let Some((receiver, generation)) = self.viewer.scan_receiver() else {
            return;
        };
        let Some(tx) = self.wake.clone() else {
            return;
        };
        cx.background_executor()
            .spawn_dedicated(move |_| async move {
                while let Ok(result) = receiver.recv() {
                    if tx.unbounded_send(Wake::Scan(generation, result)).is_err() {
                        break;
                    }
                }
            })
            .detach();
    }

    fn sync_scan_forwarder(&mut self, cx: &mut Context<Self>) {
        let generation = self.viewer.scan_generation();
        if generation == self.scan_forwarded {
            return;
        }
        self.scan_forwarded = generation;
        self.start_scan_forwarder(cx);
    }
}

impl Render for RevealApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let size = window.viewport_size();
        if self.drop_hover && !cx.has_active_drag() {
            self.drop_hover = false;
        }
        self.record_window_geometry(window);
        self.sync_scan_forwarder(cx);
        let timer_active = self.viewer.next_time_based_delay().is_some();
        if timer_active != self.timer_active {
            self.timer_active = timer_active;
            self.wake();
        }

        let scale = self.config.window.ui_scale.factor();
        window.set_rem_size(px(ui::BASE_REM * scale));

        let chrome = (ui::TOOLBAR_HEIGHT + ui::STATUS_BAR_HEIGHT) * scale;
        let resized = self
            .viewer
            .set_viewport(f32::from(size.width), (f32::from(size.height) - chrome).max(1.0));
        let scaled = self.viewer.set_scale_factor(window.scale_factor());
        if resized || scaled {
            self.schedule_reprepare(cx);
        }

        let title = self.compute_title();
        if title != self.window_title {
            window.set_window_title(&title);
            self.window_title = title;
        }

        let p = ui::palette(self.theme);

        div()
            .size_full()
            .relative()
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
                cx.listener(|this, _event: &MouseDownEvent, _window, cx| {
                    if this.context_menu.take().is_some() || this.zoom_menu_open {
                        this.zoom_menu_open = false;
                        cx.notify();
                    }
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
                if event.pressed_button != Some(MouseButton::Left) {
                    if this.drag_from.take().is_some() {
                        cx.notify();
                    }
                    return;
                }
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
                let cursor = this.to_image_area(event.position);
                this.viewer.zoom_at(1.1f32.powf(delta.clamp(-5.0, 5.0)), cursor);
                cx.notify();
            }))
            .child(self.render_toolbar(p, window, cx))
            .child(self.render_image_area(cx))
            .child(self.render_status_bar(p, cx))
            .children(self.context_menu.map(|at| self.render_context_menu(at, p, cx)))
            .children(
                self.update_notice.clone().map(|notice| self.render_update_toast(notice, p, cx)),
            )
            .children(self.drop_hover.then(|| self.render_drop_overlay(p)))
            .children(titlebar::resize_handles(window))
    }
}

impl RevealApp {
    fn record_window_geometry(&mut self, window: &Window) {
        let fullscreen = window.is_fullscreen();
        let maximized = window.is_maximized();
        self.cache.window.fullscreen = fullscreen;
        self.cache.window.maximized = maximized;
        if fullscreen || maximized {
            return;
        }
        if let gpui::WindowBounds::Windowed(bounds) = window.window_bounds() {
            self.cache.window.x = f32::from(bounds.origin.x) as i32;
            self.cache.window.y = f32::from(bounds.origin.y) as i32;
            self.cache.window.w = f32::from(bounds.size.width).max(1.0) as u32;
            self.cache.window.h = f32::from(bounds.size.height).max(1.0) as u32;
        }
    }

    fn to_image_area(&self, position: gpui::Point<gpui::Pixels>) -> (f32, f32) {
        let toolbar = ui::TOOLBAR_HEIGHT * self.config.window.ui_scale.factor();
        (f32::from(position.x), f32::from(position.y) - toolbar)
    }

    fn render_image_area(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex_grow(1.)
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
                        this.drag_from =
                            Some((f32::from(event.position.x), f32::from(event.position.y)));
                        return;
                    }
                    let intrinsic = this.viewer.current_intrinsic();
                    let point = this.to_image_area(event.position);
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
                        .text_size(gpui::px(13.))
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
