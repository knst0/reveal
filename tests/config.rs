use std::fs;
use std::path::PathBuf;

use reveal::config::{Cache, Configuration};

fn temp(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("reveal-cfg-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn missing_config_falls_back_to_defaults() {
    let dir = temp("missing");
    let cfg = Configuration::load_from(&dir.join("nope.json"));
    assert_eq!(cfg, Configuration::default());
    assert!(cfg.window.dark);
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn corrupt_config_falls_back_instead_of_failing() {
    let dir = temp("corrupt");
    let path = dir.join("cfg.json");
    fs::write(&path, b"this is [not valid json ===").unwrap();

    let cfg = Configuration::load_from(&path);
    assert_eq!(cfg, Configuration::default());
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn partial_config_keeps_defaults_for_absent_fields() {
    let dir = temp("partial");
    let path = dir.join("cfg.json");
    fs::write(&path, br#"{"window": {"dark": false}}"#).unwrap();

    let cfg = Configuration::load_from(&path);
    assert!(!cfg.window.dark, "explicit value honoured");
    assert!(cfg.window.show_bottom_bar, "absent field keeps its default");
    assert!(cfg.window.antialias);
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn config_round_trips_through_json() {
    let dir = temp("roundtrip");
    let path = dir.join("cfg.json");

    let mut cfg = Configuration::default();
    cfg.window.antialias = false;
    cfg.title.displayed_folders = Some(3);
    cfg.save_to(&path).unwrap();

    assert_eq!(Configuration::load_from(&path), cfg);
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn cache_round_trips_window_state() {
    let dir = temp("cache");
    let path = dir.join("cache.json");

    let mut cache = Cache::default();
    cache.window.w = 1600;
    cache.window.h = 900;
    cache.window.maximized = true;
    cache.current_dir = Some(PathBuf::from("/some/dir"));
    cache.save_to(&path).unwrap();

    let loaded = Cache::load_from(&path);
    assert_eq!(loaded.window.w, 1600);
    assert!(loaded.window.maximized);
    assert_eq!(loaded.current_dir, Some(PathBuf::from("/some/dir")));
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn config_and_cache_are_separate_files() {
    assert_ne!(
        reveal::config::CONFIG_FILE,
        reveal::config::CACHE_FILE,
        "user intent and persisted state must not share a file"
    );
}
#[test]
fn updates_settings_round_trip_through_toml() {
    let mut cfg = reveal::config::Configuration::default();
    assert!(cfg.updates.check);
    assert!(!cfg.updates.auto_install);

    cfg.updates.auto_install = true;
    let text = serde_json::to_string_pretty(&cfg).unwrap();
    let back: reveal::config::Configuration = serde_json::from_str(&text).unwrap();
    assert_eq!(back.updates, cfg.updates);
    assert!(back.updates.should_auto_install());
}

#[test]
fn a_config_without_an_updates_section_gets_the_defaults() {
    let cfg: reveal::config::Configuration =
        serde_json::from_str(r#"{"window": {"dark": false}}"#).unwrap();
    assert!(cfg.updates.check);
    assert!(!cfg.updates.auto_install);
}

#[test]
fn the_channel_round_trips_and_defaults_to_stable() {
    let mut cfg = reveal::config::Configuration::default();
    assert_eq!(cfg.updates.channel, reveal::config::Channel::Stable);

    cfg.updates.channel = reveal::config::Channel::Beta;
    let text = serde_json::to_string_pretty(&cfg).unwrap();
    assert!(text.contains("\"channel\": \"beta\""), "unexpected serialisation: {text}");

    let back: reveal::config::Configuration = serde_json::from_str(&text).unwrap();
    assert_eq!(back.updates.channel, reveal::config::Channel::Beta);
}

#[test]
fn an_unknown_channel_falls_back_to_the_defaults() {
    let cfg: reveal::config::Configuration =
        reveal::config::Configuration::load_from(std::path::Path::new("does-not-exist.json"));
    assert_eq!(cfg.updates.channel, reveal::config::Channel::Stable);
}
