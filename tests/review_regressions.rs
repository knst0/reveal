//! Review-only regressions for reveal e8ae50b6927e2d2be024d18fbbe43965054a75be.
//! Copy to tests/review_regressions.rs in a disposable checkout, then run:
//! cargo test --locked --test review_regressions -- --test-threads=1 --nocapture
//! All tests assert the desired behavior; failures demonstrate review findings.
//! The aspect-ratio test calculates dimensions only and never allocates the
//! multi-gigabyte output that the production preparation path would request.

use std::path::PathBuf;

use reveal::decode::DecodedImage;
use reveal::render::{FitMode, downscaled_size, needs_downscale};
use reveal::viewer::Viewer;

fn pixels(width: u32, height: u32) -> DecodedImage {
    DecodedImage { width, height, rgba: vec![127; (width * height * 4) as usize] }
}

#[test]
fn review_downscaling_a_thin_image_must_not_enlarge_it() {
    let source = (10_000, 10);
    let display = (1_920.0, 1_080.0);
    assert!(needs_downscale(source, display));
    let output = downscaled_size(source, display);
    let allocation = output.0 as u64 * output.1 as u64 * 4;
    println!("source={source:?}, output={output:?}, planned_rgba_bytes={allocation}");
    assert!(
        output.0 <= source.0 && output.1 <= source.1,
        "A downscale must not enlarge either source dimension; output={output:?}, bytes={allocation}"
    );
}

#[test]
fn review_fit_to_window_must_follow_viewport_resize() {
    let mut viewer = Viewer::new();
    viewer.set_viewport(900.0, 600.0);
    viewer.show_pasted(pixels(300, 200));
    assert_eq!(viewer.view.transform.fit, FitMode::Fit);
    assert_eq!(
        viewer.view.transform.displayed_size(viewer.presentation.current_intrinsic()),
        (900.0, 600.0)
    );

    viewer.set_viewport(450.0, 300.0);
    let actual = viewer.view.transform.displayed_size(viewer.presentation.current_intrinsic());
    println!(
        "new_viewport={:?}, displayed={actual:?}, fit={:?}",
        viewer.view.viewport, viewer.view.transform.fit
    );
    assert_eq!(actual, (450.0, 300.0), "Fit to Window should recompute the fit after resizing");
}

#[test]
fn review_original_size_must_map_source_pixels_to_physical_pixels() {
    let mut viewer = Viewer::new();
    viewer.set_viewport(800.0, 600.0);
    viewer.set_scale_factor(2.0);
    viewer.show_pasted(pixels(300, 200));
    viewer.set_fit(FitMode::Original);

    let logical = viewer.view.transform.displayed_size(viewer.presentation.current_intrinsic());
    let physical = (logical.0 * viewer.view.scale_factor, logical.1 * viewer.view.scale_factor);
    println!(
        "source={:?}, logical={logical:?}, physical={physical:?}, dpr={}",
        viewer.presentation.current_source_size(),
        viewer.view.scale_factor
    );
    assert_eq!(
        physical,
        (300.0, 200.0),
        "Original Size should be one physical display pixel per source pixel"
    );
}

struct TemporaryImage(PathBuf);
impl Drop for TemporaryImage {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

#[test]
fn review_bare_relative_cli_filename_must_survive_directory_scan() {
    // Do not change the process cwd; a unique fixture is written into the
    // disposable checkout so this also works safely with other integration tests.
    let fixture_path = PathBuf::from(format!("reveal-review-relative-{}.png", std::process::id()));
    assert!(!fixture_path.exists(), "refusing to overwrite an existing file");
    let fixture = TemporaryImage(fixture_path);
    image::RgbaImage::from_pixel(32, 32, image::Rgba([1, 2, 3, 255])).save(&fixture.0).unwrap();
    let mut viewer = Viewer::new();
    viewer.set_viewport(800.0, 600.0);
    viewer.open(&fixture.0).unwrap();
    viewer.wait_for_scan();
    viewer.settle();
    println!(
        "argument={:?}, directory_current={:?}, current_path={:?}, status={:?}",
        fixture.0,
        viewer.session.directory.current(),
        viewer.presentation.current_path(),
        viewer.session.status()
    );
    assert!(
        viewer.presentation.current_path().is_some(),
        "A valid relative image filename must remain pending until it has been presented"
    );
    assert!(viewer.render_image().is_some());
    drop(viewer);
}
