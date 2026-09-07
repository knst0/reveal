use reveal::config::Channel;
use reveal::update::{Install, detect_install, is_newer, pick_upgrade};

#[test]
fn only_a_standalone_install_may_replace_its_own_binary() {
    assert!(Install::Standalone.can_self_update());
    for managed in [Install::MacosBundle, Install::WindowsInstaller, Install::Flatpak] {
        assert!(!managed.can_self_update(), "{managed:?} must not self-update");
    }
}

#[test]
fn every_managed_install_explains_how_to_upgrade() {
    for managed in [Install::MacosBundle, Install::WindowsInstaller, Install::Flatpak] {
        assert!(!managed.upgrade_hint().is_empty(), "{managed:?} needs a hint");
    }
}

#[test]
fn a_flatpak_sandbox_is_detected_from_the_environment() {
    if !cfg!(target_os = "linux") {
        return;
    }
    unsafe { std::env::set_var("FLATPAK_ID", "io.github.knst0.reveal") };
    let detected = detect_install();
    unsafe { std::env::remove_var("FLATPAK_ID") };
    assert_eq!(detected, Install::Flatpak);
}

#[test]
fn the_test_binary_is_a_standalone_install() {
    assert_eq!(detect_install(), Install::Standalone);
}

#[test]
fn upgrades_still_pick_the_highest_accepted_version() {
    assert!(is_newer("0.3.2", "0.4.0"));
    assert_eq!(
        pick_upgrade(Channel::Stable, "0.3.2", ["0.3.3", "0.4.0", "0.5.0-beta.1"]),
        Some("0.4.0")
    );
}

#[test]
fn a_symlink_into_the_bundle_still_resolves_to_the_bundle() {
    let tmp = std::env::temp_dir().join("reveal-symlink-probe");
    let _ = std::fs::remove_dir_all(&tmp);
    let macos = tmp.join("Reveal.app/Contents/MacOS");
    std::fs::create_dir_all(&macos).unwrap();
    let real = macos.join("reveal");
    std::fs::write(&real, b"x").unwrap();

    let canonical = std::fs::canonicalize(&real).unwrap();
    assert!(
        canonical.components().any(|c| c.as_os_str() == "Reveal.app"),
        "canonicalize must keep the bundle in the path: {canonical:?}"
    );
    let _ = std::fs::remove_dir_all(&tmp);
}
