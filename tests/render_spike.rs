use reveal::decode::DecodedImage;
use reveal::render::{
    FitMode, Resample, ViewTransform, downscale_to_display, downscale_to_display_with,
    fit_factor_to_budget, magnify_factor, magnify_nearest, magnify_nearest_crop, needs_downscale,
    to_bgra, to_render_image,
};

#[test]
fn magnify_factor_only_kicks_in_above_one_to_one() {
    assert_eq!(magnify_factor(0.25), 1);
    assert_eq!(magnify_factor(1.0), 1);
    assert_eq!(magnify_factor(1.2), 2);
    assert_eq!(magnify_factor(4.0), 4);
    assert_eq!(magnify_factor(999.0), 64);
    assert_eq!(magnify_factor(f32::NAN), 1);
}

#[test]
fn budget_clamps_the_factor_before_a_huge_allocation() {
    let viewport_crop = (0, 0, 1920, 1080);
    let clamped = fit_factor_to_budget(viewport_crop, 64);
    let bytes = 1920u64 * 1080 * (clamped as u64).pow(2) * 4;
    assert!(bytes <= 96 * 1024 * 1024, "{clamped} -> {bytes} bytes");

    let tiny = (0, 0, 32, 32);
    assert_eq!(fit_factor_to_budget(tiny, 64), 64, "small crops keep the full factor");
    assert_eq!(fit_factor_to_budget((0, 0, 0, 0), 8), 1);
}

#[test]
fn visible_rect_shrinks_as_zoom_grows() {
    let intrinsic = (1920.0, 1080.0);
    let viewport = (1920.0, 1080.0);
    let at = |zoom| ViewTransform { zoom, offset: (0.0, 0.0), fit: FitMode::Free };

    let full = at(1.0).visible_source_rect(intrinsic, viewport).unwrap();
    assert_eq!(full, (0, 0, 1920, 1080));

    let zoomed = at(8.0).visible_source_rect(intrinsic, viewport).unwrap();
    assert!(zoomed.2 <= 245 && zoomed.3 <= 140, "crop should be ~viewport/zoom: {zoomed:?}");

    let fitted = at(0.25).visible_source_rect(intrinsic, viewport).unwrap();
    assert_eq!(fitted, (0, 0, 1920, 1080), "minified image is fully visible");
}

#[test]
fn magnify_crop_matches_the_same_region_of_a_full_magnify() {
    let src = concentric_rings(64, 48);
    let crop = (10, 7, 21, 13);
    let factor = 5;

    let full = magnify_nearest(&src, factor);
    let cropped = magnify_nearest_crop(&src, crop, factor);

    assert_eq!((cropped.width, cropped.height), (21 * 5, 13 * 5));
    for y in 0..cropped.height {
        for x in 0..cropped.width {
            let c = ((y * cropped.width + x) * 4) as usize;
            let fx = crop.0 * factor + x;
            let fy = crop.1 * factor + y;
            let f = ((fy * full.width + fx) * 4) as usize;
            assert_eq!(&cropped.rgba[c..c + 4], &full.rgba[f..f + 4], "at {x},{y}");
        }
    }
}

#[test]
fn magnify_crop_clamps_out_of_bounds_regions() {
    let src = concentric_rings(16, 16);
    let clipped = magnify_nearest_crop(&src, (12, 12, 100, 100), 2);
    assert_eq!((clipped.width, clipped.height), (8, 8));

    let empty = magnify_nearest_crop(&src, (16, 16, 4, 4), 2);
    assert_eq!((empty.width, empty.height), (0, 0));
}

#[test]
fn crop_magnify_allocates_far_less_than_full_magnify() {
    let src = concentric_rings(1920, 1080);
    let intrinsic = (1920.0, 1080.0);
    let viewport = (1920.0, 1080.0);
    let transform = ViewTransform { zoom: 16.0, offset: (0.0, 0.0), fit: FitMode::Free };
    let crop = transform.visible_source_rect(intrinsic, viewport).unwrap();

    let cropped = magnify_nearest_crop(&src, crop, 16);
    let full_bytes = 1920u64 * 1080 * 16 * 16 * 4;
    let crop_bytes = cropped.rgba.len() as u64;

    assert!(crop_bytes * 100 < full_bytes, "crop {crop_bytes} vs full {full_bytes}");
    assert!(crop_bytes < 16 * 1024 * 1024, "must stay well under 16 MiB: {crop_bytes}");
}

#[test]
fn magnify_nearest_replicates_pixels_into_hard_blocks() {
    let src = DecodedImage {
        rgba: vec![0, 0, 0, 255, 255, 255, 255, 255, 255, 0, 0, 255, 0, 0, 255, 255],
        width: 2,
        height: 2,
    };
    let out = magnify_nearest(&src, 3);

    assert_eq!((out.width, out.height), (6, 6));
    for y in 0..6u32 {
        for x in 0..6u32 {
            let dst = ((y * 6 + x) * 4) as usize;
            let s = (((y / 3) * 2 + x / 3) * 4) as usize;
            assert_eq!(&out.rgba[dst..dst + 4], &src.rgba[s..s + 4], "at {x},{y}");
        }
    }
    assert_eq!(magnify_nearest(&src, 1).rgba, src.rgba);
}

fn concentric_rings(w: u32, h: u32) -> DecodedImage {
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    let (cx, cy) = (w as f32 / 2.0, h as f32 / 2.0);
    for y in 0..h {
        for x in 0..w {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let r = (dx * dx + dy * dy).sqrt();
            let v = if ((r / 3.0) as u32).is_multiple_of(2) { 255u8 } else { 0 };
            rgba.extend_from_slice(&[v, v, v, 255]);
        }
    }
    DecodedImage { rgba, width: w, height: h }
}

fn nearest_downscale(src: &DecodedImage, w: u32, h: u32) -> DecodedImage {
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let sx = x * src.width / w;
            let sy = y * src.height / h;
            let i = ((sy * src.width + sx) * 4) as usize;
            rgba.extend_from_slice(&src.rgba[i..i + 4]);
        }
    }
    DecodedImage { rgba, width: w, height: h }
}

fn local_variance(img: &DecodedImage) -> f64 {
    let mut total = 0.0;
    let mut count = 0.0;
    for y in 1..img.height - 1 {
        for x in 1..img.width - 1 {
            let at = |xx: u32, yy: u32| img.rgba[((yy * img.width + xx) * 4) as usize] as f64;
            let c = at(x, y);
            let d = (at(x + 1, y) - c).abs() + (at(x, y + 1) - c).abs();
            total += d;
            count += 1.0;
        }
    }
    total / count
}

#[test]
fn lanczos_downscale_beats_nearest_on_aliasing() {
    let src = concentric_rings(1200, 800);
    let display = (300.0, 200.0);

    let good = downscale_to_display(&src, display);
    let bad = nearest_downscale(&src, good.width, good.height);

    let good_v = local_variance(&good);
    let bad_v = local_variance(&bad);

    assert!(
        good_v < bad_v * 0.85,
        "filtered downscale should reduce high-frequency noise: lanczos={good_v:.2} nearest={bad_v:.2}"
    );
    println!("aliasing energy: lanczos={good_v:.2} nearest={bad_v:.2}");
}

#[test]
fn antialias_toggle_picks_distinguishable_resample_paths() {
    let src = concentric_rings(1200, 800);
    let display = (300.0, 200.0);

    let filtered = downscale_to_display_with(&src, display, Resample::Filtered);
    let nearest = downscale_to_display_with(&src, display, Resample::Nearest);

    assert_eq!((filtered.width, filtered.height), (nearest.width, nearest.height));
    assert_ne!(filtered.rgba, nearest.rgba, "filtered and nearest must differ");
    assert!(
        local_variance(&filtered) < local_variance(&nearest) * 0.85,
        "filtered path should suppress aliasing relative to nearest"
    );
    assert_eq!(Resample::from_antialias(true), Resample::Filtered);
    assert_eq!(Resample::from_antialias(false), Resample::Nearest);
}

#[test]
fn downscales_a_large_photo_to_around_display_size() {
    let src = concentric_rings(6000, 4000);
    let out = downscale_to_display(&src, (1920.0, 1080.0));

    assert!(out.width <= 1920 * 2 && out.height <= 1080 * 2);
    assert!(out.width >= 1600, "should still fill the window: {}", out.width);
    let ratio_in = 6000.0 / 4000.0;
    let ratio_out = out.width as f32 / out.height as f32;
    assert!((ratio_in - ratio_out).abs() < 0.01, "aspect preserved");

    let before = src.rgba.len();
    let after = out.rgba.len();
    println!("upload bytes: {before} -> {after} ({:.1}x less)", before as f64 / after as f64);
    assert!(after * 4 < before, "should cut atlas pressure substantially");
}

#[test]
fn small_images_are_not_resampled() {
    let src = concentric_rings(100, 80);
    assert!(!needs_downscale((100, 80), (1920.0, 1080.0)));
    let out = downscale_to_display(&src, (1920.0, 1080.0));
    assert_eq!((out.width, out.height), (100, 80));
    assert_eq!(out.rgba, src.rgba);
}

#[test]
fn converts_rgba_to_bgra_for_the_atlas() {
    let img = DecodedImage { rgba: vec![10, 20, 30, 255], width: 1, height: 1 };
    let out = to_bgra(&img);
    assert_eq!(out.as_raw(), &[30, 20, 10, 255]);
}

#[test]
fn builds_animation_render_image_with_all_frames() {
    use reveal::decode::{Decoded, Frame};
    use std::time::Duration;

    let frames = (0..3)
        .map(|_| Frame {
            image: DecodedImage { rgba: vec![1, 2, 3, 255], width: 1, height: 1 },
            delay: Duration::from_millis(80),
        })
        .collect();
    let render = to_render_image(&Decoded::Animation(frames));
    assert_eq!(render.frame_count(), 3);
}

#[test]
fn fit_modes_compute_expected_zoom() {
    let image = (4000.0, 2000.0);
    let viewport = (1000.0, 1000.0);

    assert_eq!(ViewTransform::fit_zoom(image, viewport, FitMode::Fit), 0.25);
    assert_eq!(ViewTransform::fit_zoom(image, viewport, FitMode::Original), 1.0);

    let small = (100.0, 100.0);
    assert_eq!(ViewTransform::fit_zoom(small, viewport, FitMode::FitBest), 1.0);
    assert_eq!(ViewTransform::fit_zoom(small, viewport, FitMode::Fit), 10.0);
}

#[test]
fn image_contains_only_true_inside_drawn_bounds() {
    let t = ViewTransform { zoom: 1.0, offset: (0.0, 0.0), fit: FitMode::Free };
    let image = (200.0, 100.0);
    let viewport = (600.0, 400.0);

    assert!(t.image_contains(image, viewport, (300.0, 200.0)), "centre pixel is inside");
    assert!(!t.image_contains(image, viewport, (10.0, 10.0)), "background corner is outside");
    assert!(!t.image_contains(image, viewport, (300.0, 390.0)), "toolbar strip is outside");
    assert!(!t.image_contains((0.0, 0.0), viewport, (300.0, 200.0)), "no image means no hit");
}

#[test]
fn zoom_at_cursor_keeps_that_point_stationary() {
    let image = (1000.0, 1000.0);
    let viewport = (500.0, 500.0);
    let cursor = (100.0, 100.0);

    let mut t = ViewTransform { zoom: 1.0, offset: (0.0, 0.0), fit: FitMode::Free };

    let point_before = {
        let (x, y, w, h) = t.image_bounds(image, viewport);
        ((cursor.0 - x) / w, (cursor.1 - y) / h)
    };

    t.zoom_at(2.0, cursor, viewport);

    let point_after = {
        let (x, y, w, h) = t.image_bounds(image, viewport);
        ((cursor.0 - x) / w, (cursor.1 - y) / h)
    };

    assert!(
        (point_before.0 - point_after.0).abs() < 0.001
            && (point_before.1 - point_after.1).abs() < 0.001,
        "cursor anchor drifted: {point_before:?} -> {point_after:?}"
    );
}

#[test]
fn image_is_centred_when_it_fits() {
    let t = ViewTransform { zoom: 1.0, offset: (0.0, 0.0), fit: FitMode::Fit };
    let (x, y, w, h) = t.image_bounds((200.0, 100.0), (600.0, 400.0));
    assert_eq!((x, y, w, h), (200.0, 150.0, 200.0, 100.0));
}

fn corner_marked(w: u32, h: u32) -> DecodedImage {
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    for i in 0..(w * h) as usize {
        rgba[i * 4 + 3] = 255;
    }
    rgba[0] = 255;
    DecodedImage { rgba, width: w, height: h }
}

#[test]
fn rotate90_moves_the_marker_and_swaps_dimensions() {
    use reveal::decode::Orientation;
    use reveal::render::apply_orientation;

    let src = corner_marked(4, 2);
    let out = apply_orientation(&src, Orientation::Rotate90);

    assert_eq!((out.width, out.height), (2, 4), "dimensions must swap");
    let top_right = ((out.width - 1) * 4) as usize;
    assert_eq!(out.rgba[top_right], 255, "top-left should rotate to top-right");
    assert_eq!(out.rgba[0], 0);
}

#[test]
fn horizontal_flip_mirrors_without_resizing() {
    use reveal::decode::Orientation;
    use reveal::render::apply_orientation;

    let src = corner_marked(4, 2);
    let out = apply_orientation(&src, Orientation::FlipH);

    assert_eq!((out.width, out.height), (4, 2));
    assert_eq!(out.rgba[(3 * 4) as usize], 255, "marker moves to the right edge");
    assert_eq!(out.rgba[0], 0);
}

#[test]
fn normal_orientation_is_a_passthrough() {
    use reveal::decode::Orientation;
    use reveal::render::apply_orientation;

    let src = corner_marked(3, 5);
    let out = apply_orientation(&src, Orientation::Normal);
    assert_eq!(out.rgba, src.rgba);
    assert_eq!((out.width, out.height), (3, 5));
}
