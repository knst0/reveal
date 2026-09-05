use std::fs;
use std::path::PathBuf;

use reveal::directory::Navigation;
use reveal::render::FitMode;
use reveal::viewer::Viewer;

fn fixture(tag: &str, count: u32) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("reveal-viewer-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    for i in 0..count {
        let img = image::RgbaImage::from_pixel(300, 200, image::Rgba([i as u8, 5, 5, 255]));
        image::DynamicImage::ImageRgba8(img).save(dir.join(format!("{i}.png"))).unwrap();
    }
    dir
}

#[test]
fn opens_an_image_and_indexes_its_directory() {
    let dir = fixture("open", 3);
    let mut v = Viewer::new();
    v.set_viewport(800.0, 600.0);
    v.open(&dir.join("1.png")).unwrap();
    v.settle();

    assert_eq!(v.current_path().unwrap(), dir.join("1.png"));
    assert_eq!(v.directory.len(), 3);
    assert_eq!(v.directory.current_index(), 1);
    assert!(v.render_image().is_some());
    assert!(v.status().is_none());
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn navigates_forward_and_wraps() {
    let dir = fixture("nav", 3);
    let mut v = Viewer::new();
    v.set_viewport(800.0, 600.0);
    v.open(&dir.join("2.png")).unwrap();
    v.settle();

    v.navigate(Navigation::Next);
    v.settle();
    assert_eq!(v.current_path().unwrap(), dir.join("0.png"));

    v.navigate(Navigation::Prev);
    v.settle();
    assert_eq!(v.current_path().unwrap(), dir.join("2.png"));
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn fit_modes_change_the_zoom() {
    let dir = fixture("fit", 1);
    let mut v = Viewer::new();
    v.set_viewport(900.0, 600.0);
    v.open(&dir.join("0.png")).unwrap();
    v.settle();

    v.set_fit(FitMode::Original);
    assert_eq!(v.transform.zoom, 1.0);

    v.set_fit(FitMode::Fit);
    assert!(v.transform.zoom > 1.0, "300x200 should scale up to fill 900x600");
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_broken_image_reports_status_instead_of_crashing() {
    let dir = fixture("broken", 1);
    let bad = dir.join("bad.png");
    fs::write(&bad, b"not an image at all").unwrap();

    let mut v = Viewer::new();
    v.set_viewport(800.0, 600.0);
    v.open(&bad).unwrap();
    v.settle();

    assert!(v.status().is_some(), "should surface an error");
    assert!(v.render_image().is_none());
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn antialias_toggle_persists_across_navigation() {
    let dir = fixture("aa", 3);
    let mut v = Viewer::new();
    v.set_viewport(800.0, 600.0);
    v.set_antialias(false);
    v.open(&dir.join("0.png")).unwrap();
    v.settle();
    assert!(!v.antialias());

    v.navigate(Navigation::Next);
    assert!(!v.antialias(), "navigation keeps the chosen resample mode");

    v.toggle_antialias();
    assert!(v.antialias());
    v.navigate(Navigation::Next);
    assert!(v.antialias());
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn panning_switches_to_free_mode() {
    let dir = fixture("pan", 1);
    let mut v = Viewer::new();
    v.set_viewport(800.0, 600.0);
    v.open(&dir.join("0.png")).unwrap();
    v.settle();

    v.pan((25.0, -10.0));
    assert_eq!(v.transform.fit, FitMode::Free);
    assert_eq!(v.transform.offset, (25.0, -10.0));
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn navigation_does_not_block_on_decode() {
    let dir = fixture("nonblock", 6);
    let mut v = Viewer::new();
    v.set_viewport(800.0, 600.0);
    v.open(&dir.join("0.png")).unwrap();
    v.settle();

    let start = std::time::Instant::now();
    for _ in 0..5 {
        v.navigate(Navigation::Next);
    }
    let elapsed = start.elapsed();

    assert!(
        elapsed < std::time::Duration::from_millis(50),
        "navigation should return without waiting for decode, took {elapsed:?}"
    );

    v.settle();
    assert_eq!(v.current_path().unwrap(), dir.join("5.png"));
    fs::remove_dir_all(&dir).unwrap();
}

fn large_fixture(tag: &str, w: u32, h: u32) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("reveal-viewer-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let img = image::RgbaImage::from_pixel(w, h, image::Rgba([7, 7, 7, 255]));
    image::DynamicImage::ImageRgba8(img).save(dir.join("big.png")).unwrap();
    dir
}

#[test]
fn the_reported_size_is_the_original_not_the_downscaled_copy() {
    let dir = large_fixture("truesize", 1920, 1080);
    let mut v = Viewer::new();
    v.set_viewport(640.0, 400.0);
    v.open(&dir.join("big.png")).unwrap();
    v.settle();

    assert_eq!(v.current_source_size(), (1920, 1080));
    let (iw, ih) = v.current_intrinsic();
    assert!(iw < 1920.0 && ih < 1080.0, "a downscaled copy should back the render");

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn original_shows_true_pixels_even_when_a_downscaled_copy_is_rendered() {
    let dir = large_fixture("original", 1920, 1080);
    let mut v = Viewer::new();
    v.set_viewport(640.0, 400.0);
    v.open(&dir.join("big.png")).unwrap();
    v.settle();

    v.set_fit(FitMode::Original);
    let (iw, _) = v.current_intrinsic();
    let displayed = iw * v.transform.zoom;
    assert!(
        (displayed - 1920.0).abs() < 2.0,
        "Original should display {} source pixels, got {displayed}",
        1920
    );

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn opening_a_directory_shows_its_first_file() {
    let dir = fixture("dirarg", 3);
    let mut v = Viewer::new();
    v.set_viewport(800.0, 600.0);
    v.open(&dir).unwrap();
    v.settle();

    assert_eq!(v.current_path().unwrap(), dir.join("0.png"));
    assert_eq!(v.directory.len(), 3);
    assert!(v.render_image().is_some());
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn open_returns_before_the_scan_and_the_first_decode() {
    let dir = fixture("fastopen", 400);
    let mut v = Viewer::new();
    v.set_viewport(800.0, 600.0);

    let start = std::time::Instant::now();
    v.open(&dir.join("7.png")).unwrap();
    let elapsed = start.elapsed();

    assert!(
        elapsed < std::time::Duration::from_millis(30),
        "open should not block on scan or decode, took {elapsed:?}"
    );
    assert!(v.needs_ticking(), "the viewer must keep ticking while startup is in flight");

    v.settle();
    assert_eq!(v.current_path().unwrap(), dir.join("7.png"));
    assert_eq!(v.directory.len(), 400);
    assert_eq!(v.directory.path_at(v.directory.current_index()).unwrap(), dir.join("7.png"));
    fs::remove_dir_all(&dir).unwrap();
}
