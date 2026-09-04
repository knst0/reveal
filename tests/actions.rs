use std::fs;

use reveal::actions::{Theme, first_frame, move_to_trash};
use reveal::config::{Cache, Channel, Updates};
use reveal::decode::{Decoded, DecodedImage, Frame};
use reveal::panic_report::format_report;
use reveal::update::{
    CHECK_INTERVAL, UpdateNotice, accepts, changelog_url, current_version, is_due, is_newer,
    is_prerelease, is_skipped, pick_upgrade, skip_version, take_upgrade_notice,
};

fn img(v: u8) -> DecodedImage {
    DecodedImage { rgba: vec![v, v, v, 255], width: 1, height: 1 }
}

#[test]
fn first_frame_works_for_stills_and_animations() {
    assert_eq!(first_frame(&Decoded::Still(img(7))).unwrap().rgba[0], 7);

    let anim = Decoded::Animation(vec![
        Frame { image: img(1), delay: std::time::Duration::from_millis(10) },
        Frame { image: img(2), delay: std::time::Duration::from_millis(10) },
    ]);
    assert_eq!(first_frame(&anim).unwrap().rgba[0], 1);
}

#[test]
fn empty_animation_has_no_frame_to_copy() {
    assert!(first_frame(&Decoded::Animation(vec![])).is_none());
}

#[test]
fn trash_removes_the_file_from_its_directory() {
    let dir = std::env::temp_dir().join(format!("reveal-trash-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("doomed.png");
    let image = image::RgbaImage::from_pixel(4, 4, image::Rgba([1, 2, 3, 255]));
    image::DynamicImage::ImageRgba8(image).save(&path).unwrap();
    assert!(path.exists());

    move_to_trash(&path).expect("trash should accept a real file");
    assert!(!path.exists(), "file should be gone from its directory");
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn theme_toggles_and_changes_colours() {
    let dark = Theme::Dark;
    let light = dark.toggled();
    assert_eq!(light, Theme::Light);
    assert_eq!(light.toggled(), Theme::Dark);
    assert!(dark.is_dark());
    assert_ne!(dark.background(), light.background());
    assert_ne!(dark.text(), light.text());
    assert_eq!(Theme::from_dark(false), Theme::Light);
}

#[test]
fn panic_report_names_the_version_and_os() {
    let report = format_report("panicked at 'boom'", "1.2.3", "windows");
    assert!(report.contains("1.2.3"));
    assert!(report.contains("windows"));
    assert!(report.contains("boom"));
}

#[cfg(not(feature = "updates"))]
use reveal::update::UpdateStatus;

#[test]
#[cfg(not(feature = "updates"))]
fn update_check_is_off_unless_the_updates_feature_is_enabled() {
    assert_eq!(reveal::update::check(Channel::Stable), UpdateStatus::Disabled);
    assert_eq!(reveal::update::install(Channel::Stable), UpdateStatus::Disabled);
}

#[test]
fn version_comparison_ignores_prerelease_suffixes() {
    assert!(is_newer("1.2.3", "1.3.0-rc1"));
    assert!(!is_newer("1.2.3", "1.2.3-rc1"));
}

#[test]
fn auto_install_requires_checking_to_be_enabled() {
    let off = Updates { check: false, auto_install: true, ..Updates::default() };
    assert!(!off.should_check());
    assert!(!off.should_auto_install());

    let notify_only = Updates { check: true, auto_install: false, ..Updates::default() };
    assert!(notify_only.should_check());
    assert!(!notify_only.should_auto_install());

    let silent = Updates { check: true, auto_install: true, ..Updates::default() };
    assert!(silent.should_check());
    assert!(silent.should_auto_install());
}

#[test]
fn checks_are_throttled_to_the_interval() {
    let day = CHECK_INTERVAL;
    assert!(is_due(None, 0, day));
    assert!(is_due(Some(0), day.as_secs(), day));
    assert!(!is_due(Some(0), day.as_secs() - 1, day));
    assert!(!is_due(Some(500), 100, day));
}

#[test]
fn a_skipped_version_is_not_offered_again() {
    let mut cache = Cache::default();
    assert!(!is_skipped(&cache, "1.2.0"));
    skip_version(&mut cache, "1.2.0");
    assert!(is_skipped(&cache, "1.2.0"));
    assert!(!is_skipped(&cache, "1.3.0"));
}

#[test]
fn a_pending_install_becomes_a_changelog_notice_after_restart() {
    let current = current_version();
    let mut cache = Cache { update_pending_version: Some(current.to_owned()), ..Cache::default() };

    let notice = take_upgrade_notice(&mut cache);
    assert!(matches!(notice, Some(UpdateNotice::Upgraded { .. })));
    assert_eq!(cache.update_pending_version, None);
    assert_eq!(cache.last_launched_version.as_deref(), Some(current));

    assert_eq!(take_upgrade_notice(&mut cache), None);
}

#[test]
fn a_first_run_shows_no_changelog_notice() {
    let mut cache = Cache::default();
    assert_eq!(take_upgrade_notice(&mut cache), None);
    assert_eq!(cache.last_launched_version.as_deref(), Some(current_version()));
}

#[test]
fn changelog_url_points_at_the_release_tag() {
    assert!(changelog_url("1.2.3").ends_with("/releases/tag/v1.2.3"));
    assert!(changelog_url("v1.2.3").ends_with("/releases/tag/v1.2.3"));
}

#[test]
fn version_comparison_handles_multi_digit_parts() {
    assert!(is_newer("1.2.3", "1.2.4"));
    assert!(is_newer("1.9.0", "1.10.0"));
    assert!(!is_newer("2.0.0", "1.9.9"));
    assert!(!is_newer("1.2.3", "1.2.3"));
    assert!(is_newer("v1.0", "v1.0.1"));
}

#[test]
fn prereleases_are_recognised_by_their_suffix() {
    assert!(is_prerelease("1.2.0-rc1"));
    assert!(is_prerelease("v1.2.0-beta.2"));
    assert!(!is_prerelease("1.2.0"));
    assert!(!is_prerelease("v1.2.0"));
}

#[test]
fn the_stable_channel_refuses_prereleases() {
    assert!(accepts(Channel::Stable, "1.2.0"));
    assert!(!accepts(Channel::Stable, "1.3.0-rc1"));
}

#[test]
fn the_beta_channel_accepts_both() {
    assert!(accepts(Channel::Beta, "1.2.0"));
    assert!(accepts(Channel::Beta, "1.3.0-rc1"));
}

#[test]
fn stable_skips_a_newer_prerelease_for_the_newest_stable() {
    let versions = ["1.4.0-rc1", "1.3.0", "1.2.0"];
    assert_eq!(pick_upgrade(Channel::Stable, "1.2.0", versions), Some("1.3.0"));
    assert_eq!(pick_upgrade(Channel::Beta, "1.2.0", versions), Some("1.4.0-rc1"));
}

#[test]
fn stable_reports_nothing_when_only_prereleases_are_newer() {
    let versions = ["1.4.0-rc1", "1.2.0"];
    assert_eq!(pick_upgrade(Channel::Stable, "1.2.0", versions), None);
}

#[test]
fn a_release_order_from_the_api_does_not_decide_the_pick() {
    let versions = ["1.2.0", "1.9.0", "1.3.0"];
    assert_eq!(pick_upgrade(Channel::Stable, "1.1.0", versions), Some("1.9.0"));
}

#[test]
fn nothing_older_than_the_current_version_is_offered() {
    let versions = ["1.0.0", "0.9.0"];
    assert_eq!(pick_upgrade(Channel::Stable, "1.0.0", versions), None);
    assert_eq!(pick_upgrade(Channel::Beta, "1.0.0", versions), None);
}
