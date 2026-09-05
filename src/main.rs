#![cfg_attr(all(target_os = "windows", not(debug_assertions)), windows_subsystem = "windows")]

mod app;

use app::{AppInit, RevealApp};
use clap::Parser;
use gpui::{App, AppContext, Application, Bounds, WindowBounds, WindowOptions, px, size};
use reveal::config::Configuration;
use reveal::input::Bindings;
use reveal::viewer::Viewer;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "Reveal")]
struct Args {
    path: Option<PathBuf>,
}

fn main() {
    reveal::panic_report::install();
    env_logger::init();
    let args = Args::parse();

    std::thread::spawn(reveal::decode::warm_font_database);

    let config = Configuration::load();
    let mut bindings = Bindings::default();
    if let Some(overrides) = config.bindings.as_ref() {
        bindings.apply_overrides(overrides);
    }

    let mut viewer = Viewer::new();
    viewer.set_viewport(1280.0, 800.0);
    viewer.set_antialias(config.window.antialias);
    if let Some(path) = args.path.as_deref()
        && let Err(e) = viewer.open(path)
    {
        eprintln!("failed to open {}: {e}", path.display());
    }

    let cache = reveal::config::Cache::load();
    let init = AppInit { config, bindings, viewer, cache };

    Application::with_platform(gpui_platform::current_platform(false)).run(move |cx: &mut App| {
        let saved = init.cache.window.clone();
        let bounds = Bounds {
            origin: gpui::point(px(saved.x as f32), px(saved.y as f32)),
            size: size(px(saved.w.max(1) as f32), px(saved.h.max(1) as f32)),
        };
        let window_bounds = if saved.fullscreen || init.config.window.start_fullscreen {
            WindowBounds::Fullscreen(bounds)
        } else if saved.maximized {
            WindowBounds::Maximized(bounds)
        } else {
            WindowBounds::Windowed(bounds)
        };
        let view = cx
            .open_window(
                WindowOptions { window_bounds: Some(window_bounds), ..Default::default() },
                |window, cx| {
                    let view = cx.new(|cx| RevealApp::new(init, cx));
                    window.focus(&view.read(cx).focus.clone(), cx);
                    view
                },
            )
            .unwrap();

        if let Ok(view) = view.entity(cx) {
            cx.on_app_quit(move |cx: &mut App| {
                view.read(cx).save_cache();
                async {}
            })
            .detach();
        }

        cx.activate(true);
    });
}
