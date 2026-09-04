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

    Application::new().run(move |cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1280.), px(800.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |window, cx| {
                let view = cx.new(|cx| RevealApp::new(init, cx));
                window.focus(&view.read(cx).focus.clone());
                view
            },
        )
        .unwrap();
        cx.activate(true);
    });
}
