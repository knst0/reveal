mod actions;
mod compare;
mod context_menu;
pub mod labels;
mod settings_window;
mod status_bar;
pub mod titlebar;
mod toolbar;
mod update_toast;

use std::sync::Arc;
use std::time::{Duration, Instant};

use futures::StreamExt;
use gpui::{
    App, AppContext, Bounds, Context, ExternalPaths, InteractiveElement, IntoElement, MouseButton,
    MouseDownEvent, ParentElement, Render, RenderImage, Styled, Subscription, Window, WindowBounds,
    WindowDecorations, WindowHandle, WindowOptions, div, px, size,
};
use reveal::actions::Theme;
use reveal::compare::Layout;
use reveal::config::{Cache, Configuration, Updates};
use reveal::input::Bindings;
use reveal::render::{FitMode, ImageElement};
use reveal::settings::SettingsState;
use reveal::ui;
use reveal::update::UpdateNotice;
use reveal::viewer::Viewer;

use labels::title_for;
use settings_window::{SettingsWindow, WINDOW_HEIGHT, WINDOW_WIDTH};

/// Background results, tagged with the pane (0 main, 1 compared) they belong to.
enum Wake {
    Cache(usize, reveal::cache::LoadResult),
    Scan(usize, u64, std::io::Result<reveal::directory::Directory>),
    Poll,
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
    /// Pane a left-button drag started in; Ctrl+drag moves only that one.
    drag_pane: usize,
    compare: Option<compare::Compare>,
    /// Size of the area below the toolbar that holds the panes.
    image_area: (f32, f32),
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
    scan_forwarded: [u64; 2],
    /// Last image handed to the image element. gpui keeps every painted
    /// `RenderImage` in the window's sprite atlas until `drop_image`, so the
    /// previous one is evicted whenever it is replaced.
    painted: Option<Arc<RenderImage>>,
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
            drag_pane: 0,
            compare: None,
            image_area: (0.0, 0.0),
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
            scan_forwarded: [0; 2],
            painted: None,
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
        if let Err(e) = self.active_viewer_mut().open(&target) {
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
        config.window.antialias = self.viewer.view.antialias();
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
        for pane in 0..self.existing_panes() {
            self.pane_mut(pane).set_antialias(config.window.antialias);
        }
        self.update_settings = config.updates.clone();
        self.bindings = bindings;
        self.config = config;
    }

    pub fn compute_title(&self) -> String {
        title_for(self.active_viewer().presentation.current_path())
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
                        if tx.unbounded_send(Wake::Poll).is_err() {
                            break;
                        }
                    }
                })
                .detach();
        }

        self.forward_loader(0, cx);
        self.sync_scan_forwarder(cx);

        cx.spawn(async move |this, cx| {
            loop {
                let timeout =
                    this.update(cx, |this, _| this.next_time_based_delay()).ok().flatten();
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
                            if this.tick_playback() {
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
            Wake::Cache(pane, result) => {
                if self.pane_mut(pane).apply_load(result) {
                    cx.notify();
                }
            }
            Wake::Scan(pane, generation, result) => {
                if self.pane_mut(pane).apply_scan(generation, result) {
                    cx.notify();
                }
            }
            Wake::Poll => {
                let requested = reveal::drop::take_requested();
                if !requested.is_empty() {
                    self.open_dropped(&requested);
                    cx.notify();
                } else {
                    let mut changed = false;
                    for pane in 0..self.existing_panes() {
                        changed |= self.pane_mut(pane).reload_if_source_changed();
                    }
                    if changed {
                        cx.notify();
                    }
                }
            }
            Wake::Refresh => {
                let mut changed = false;
                for pane in 0..self.existing_panes() {
                    changed |= self.pane_mut(pane).reprepare_if_pending();
                }
                if changed {
                    cx.notify();
                }
            }
        }
        if self.tick_playback() {
            cx.notify();
        }
    }

    fn tick_playback(&mut self) -> bool {
        let now = Instant::now();
        let mut redraw = false;
        for pane in 0..self.visible_panes() {
            redraw |= self.pane_mut(pane).tick_playback(now);
        }
        redraw
    }

    fn next_time_based_delay(&self) -> Option<Duration> {
        (0..self.visible_panes()).filter_map(|pane| self.pane(pane).next_time_based_delay()).min()
    }

    /// Forwards a pane's decode results into the wake channel.
    fn forward_loader(&self, pane: usize, cx: &mut Context<Self>) {
        let Some(tx) = self.wake.clone() else {
            return;
        };
        let loader = self.pane(pane).session.cache.loader_handle();
        cx.background_executor()
            .spawn_dedicated(move |_| async move {
                while let Some(result) = loader.recv() {
                    if tx.unbounded_send(Wake::Cache(pane, result)).is_err() {
                        break;
                    }
                }
            })
            .detach();
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

    fn start_scan_forwarder(&mut self, pane: usize, cx: &mut Context<Self>) {
        let Some((receiver, generation)) = self.pane(pane).session.scan_receiver() else {
            return;
        };
        let Some(tx) = self.wake.clone() else {
            return;
        };
        cx.background_executor()
            .spawn_dedicated(move |_| async move {
                while let Ok(result) = receiver.recv() {
                    if tx.unbounded_send(Wake::Scan(pane, generation, result)).is_err() {
                        break;
                    }
                }
            })
            .detach();
    }

    fn sync_scan_forwarder(&mut self, cx: &mut Context<Self>) {
        for pane in 0..self.existing_panes() {
            let generation = self.pane(pane).session.scan_generation();
            if generation != self.scan_forwarded[pane] {
                self.scan_forwarded[pane] = generation;
                self.start_scan_forwarder(pane, cx);
            }
        }
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
        let timer_active = self.next_time_based_delay().is_some();
        if timer_active != self.timer_active {
            self.timer_active = timer_active;
            self.wake();
        }

        let scale = self.config.window.ui_scale.factor();
        window.set_rem_size(px(ui::BASE_REM * scale));

        let chrome = (ui::TOOLBAR_HEIGHT + ui::STATUS_BAR_HEIGHT) * scale;
        self.image_area = (f32::from(size.width), (f32::from(size.height) - chrome).max(1.0));
        // Without compare this is one full-area rect.
        let rects = self.pane_rects();
        let mut resized = false;
        for (pane, rect) in rects.iter().enumerate().take(self.visible_panes()) {
            let viewer = self.pane_mut(pane);
            resized |= viewer.set_viewport(rect.w, rect.h);
            resized |= viewer.set_scale_factor(window.scale_factor());
        }
        if resized {
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
                    let point = this.to_image_area(event.event.position);
                    let retargeted = hovering && this.hover_pane(point);
                    if this.drop_hover != hovering || retargeted {
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
                    if let Some(c) = this.compare.as_mut() {
                        c.dragging_split = false;
                    }
                }),
            )
            .on_mouse_move(cx.listener(|this, event: &gpui::MouseMoveEvent, _window, cx| {
                let point = this.to_image_area(event.position);
                if event.pressed_button != Some(MouseButton::Left) {
                    let mut changed = this.drag_from.take().is_some();
                    if let Some(c) = this.compare.as_mut() {
                        changed |= std::mem::take(&mut c.dragging_split);
                    }
                    changed |= this.hover_pane(point);
                    if changed {
                        cx.notify();
                    }
                    return;
                }
                if this.compare.as_ref().is_some_and(|c| c.dragging_split) {
                    this.move_split(point);
                    cx.notify();
                    return;
                }
                let Some((px_, py)) = this.drag_from else {
                    return;
                };
                let now = (f32::from(event.position.x), f32::from(event.position.y));
                let only = held_ctrl(&event.modifiers).then_some(this.drag_pane);
                for pane in this.targets(only) {
                    this.pane_mut(pane).view.pan((now.0 - px_, now.1 - py));
                }
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
                let point = this.to_image_area(event.position);
                this.hover_pane(point);
                let pane = this.pane_at(point);
                let cursor = this.pane_local(pane, point);
                let factor = 1.1f32.powf(delta.clamp(-5.0, 5.0));
                // Every pane zooms at the same pane-local point; Ctrl zooms only this one.
                for target in this.targets(held_ctrl(&event.modifiers).then_some(pane)) {
                    this.pane_mut(target).view.zoom_at(factor, cursor);
                }
                cx.notify();
            }))
            .child(self.render_toolbar(p, window, cx))
            .child(self.render_image_area(p, window, cx))
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

    fn render_image_area(
        &mut self,
        p: ui::Palette,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let area = div().flex_grow(1.).relative().overflow_hidden();
        let area = match self.layout() {
            Some(layout) => area
                .bg(ui::color(p.border))
                .children(self.render_compare_area(layout, p, window, cx)),
            None => {
                if let Some(old) = self.compare.as_mut().and_then(|c| c.painted.take()) {
                    cx.drop_image(old, Some(window));
                }
                let image = self.viewer.render_image();
                compare::track_painted(&mut self.painted, &image, window, cx);
                area.child(ImageElement::new(
                    image,
                    self.viewer.presentation.current_intrinsic(),
                    self.viewer.view.transform,
                    self.viewer.playback.frame_index(),
                    self.viewer.presentation.render_crop(),
                ))
            }
        };
        area.on_mouse_down(
            MouseButton::Left,
            cx.listener(|this, event: &MouseDownEvent, _window, cx| {
                let point = this.to_image_area(event.position);
                let shift = event.modifiers.shift;
                if this.layout() == Some(Layout::Slider) && (shift || this.near_split(point)) {
                    // Shift grabs the cut from anywhere; a double click on it recentres it.
                    this.drag_from = None;
                    if event.click_count >= 2 && !shift {
                        if let Some(c) = this.compare.as_mut() {
                            c.split = 0.5;
                        }
                    } else {
                        this.move_split(point);
                        if let Some(c) = this.compare.as_mut() {
                            c.dragging_split = true;
                        }
                    }
                    cx.notify();
                    return;
                }
                let pane = this.pane_at(point);
                if event.click_count < 2 {
                    this.drag_from =
                        Some((f32::from(event.position.x), f32::from(event.position.y)));
                    this.drag_pane = pane;
                    return;
                }
                let local = this.pane_local(pane, point);
                let viewer = this.pane(pane);
                if viewer.view.transform.image_contains(
                    viewer.presentation.current_intrinsic(),
                    viewer.view.viewport,
                    local,
                ) {
                    this.drag_from = None;
                    for target in this.targets(held_ctrl(&event.modifiers).then_some(pane)) {
                        this.pane_mut(target).set_fit(FitMode::Fit);
                    }
                    cx.notify();
                }
            }),
        )
    }
}

fn held_ctrl(modifiers: &gpui::Modifiers) -> bool {
    modifiers.control || modifiers.platform
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
