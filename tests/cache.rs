use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use reveal::cache::{CacheStore, CachedImage, ImageCache, Loader};
use reveal::decode::{DecodeOutput, Decoded, DecodedImage, Orientation};
use reveal::directory::Directory;

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("reveal-cachetest-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_png(path: &Path, size: u32) {
    let img = image::RgbaImage::from_pixel(size, size, image::Rgba([9, 9, 9, 255]));
    image::DynamicImage::ImageRgba8(img).save(path).unwrap();
}

fn fake(path: &str, bytes: usize) -> CachedImage {
    CachedImage {
        path: PathBuf::from(path),
        bytes,
        output: Arc::new(DecodeOutput {
            decoded: Decoded::Still(DecodedImage { rgba: Vec::new(), width: 1, height: 1 }),
            orientation: Orientation::Normal,
        }),
    }
}

#[test]
fn evicts_by_bytes_choosing_the_farthest_image() {
    let mut store = CacheStore::new(250);
    store.insert(fake("near.png", 100), 5, 5);
    store.insert(fake("mid.png", 100), 7, 5);
    store.insert(fake("far.png", 100), 40, 5);

    assert!(store.used_bytes() <= 250);
    assert!(store.contains(Path::new("near.png")));
    assert!(!store.contains(Path::new("far.png")), "farthest should go first");
}

#[test]
fn never_evicts_the_current_image_even_when_oversized() {
    let mut store = CacheStore::new(10);
    store.insert(fake("current.png", 5000), 3, 3);
    assert!(store.contains(Path::new("current.png")));
    assert_eq!(store.len(), 1);
}

#[test]
fn loader_decodes_off_the_calling_thread() {
    let dir = temp_dir("worker");
    let path = dir.join("a.png");
    write_png(&path, 32);

    let loader = Loader::new(2);
    loader.request(path.clone(), 0, (64, 64));
    let result = loader.recv().expect("a result");

    assert_eq!(result.path, path);
    let image = result.outcome.expect("decoded");
    assert_eq!(image.bytes, 32 * 32 * 4);
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn cancelled_requests_do_not_deliver_results() {
    let loader = Loader::new(1);
    let id = loader.request(PathBuf::from("nonexistent-abc.png"), 0, (1, 1));
    loader.cancel(id);

    let deadline = Instant::now() + std::time::Duration::from_millis(300);
    while Instant::now() < deadline {
        if let Some(r) = loader.try_recv() {
            assert_ne!(r.id, id, "cancelled request should not be delivered");
        }
    }
    assert_eq!(loader.pending_len(), 0);
}

#[test]
fn prefetches_both_neighbours_of_the_current_image() {
    let dir = temp_dir("prefetch");
    for n in ["1.png", "2.png", "3.png"] {
        write_png(&dir.join(n), 8);
    }
    let d = Directory::open_at(&dir.join("2.png")).unwrap();

    let mut cache = ImageCache::new(64 * 1024 * 1024, 2);
    cache.set_target_size(100, 100);
    cache.prefetch_neighbours(&d);

    let deadline = Instant::now() + std::time::Duration::from_secs(5);
    while cache.store().len() < 2 && Instant::now() < deadline {
        cache.pump(d.current_index());
    }

    assert!(cache.get(&dir.join("1.png")).is_some(), "prev prefetched");
    assert!(cache.get(&dir.join("3.png")).is_some(), "next prefetched");
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn block_on_returns_the_requested_image() {
    let dir = temp_dir("blockon");
    let path = dir.join("only.png");
    write_png(&path, 16);

    let mut cache = ImageCache::new(64 * 1024 * 1024, 2);
    cache.set_target_size(50, 50);
    let image = cache.block_on(&path, 0).expect("loaded");
    assert_eq!(image.bytes, 16 * 16 * 4);
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn block_on_reports_decode_failure() {
    let dir = temp_dir("bad");
    let path = dir.join("broken.png");
    fs::write(&path, b"this is not a png").unwrap();

    let mut cache = ImageCache::new(1024 * 1024, 1);
    assert!(cache.block_on(&path, 0).is_err());
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn first_image_is_ready_before_the_directory_is_scanned() {
    use reveal::startup;

    let dir = temp_dir("coldstart");
    let target = dir.join("000-target.png");
    write_png(&target, 24);
    for i in 0..300 {
        write_png(&dir.join(format!("filler-{i:04}.png")), 4);
    }

    let mut cache = ImageCache::new(64 * 1024 * 1024, 2);

    let t0 = Instant::now();
    let first = startup::load_first_image(&mut cache, &target, (100, 100));
    let after_first = t0.elapsed();
    assert!(first.result.is_ok(), "first image must decode");
    assert!(cache.get(&target).is_some());

    let d = startup::scan_directory(&mut cache, &target).unwrap();
    let after_scan = t0.elapsed();

    assert_eq!(d.len(), 301);
    assert!(cache.get(&target).is_some(), "scan must not drop the already-loaded image");
    assert!(
        after_first < after_scan,
        "first image ({after_first:?}) must be ready before scan completes ({after_scan:?})"
    );
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_nearest_request_is_decoded_before_its_farther_neighbours() {
    let dir = temp_dir("priority");
    let mut paths = Vec::new();
    for n in 0..7 {
        let p = dir.join(format!("{n}.png"));
        write_png(&p, 8);
        paths.push(p);
    }

    let loader = Loader::new(1);
    loader.set_current_index(3);
    for (index, path) in paths.iter().enumerate() {
        if index != 3 {
            loader.request(path.clone(), index, (16, 16));
        }
    }
    loader.request(paths[3].clone(), 3, (16, 16));

    let first = loader.recv().expect("a result");
    assert_eq!(first.index, 3, "the current image must be decoded first");

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn requests_outside_the_prefetch_window_are_cancelled() {
    let dir = temp_dir("window");
    for n in 0..40 {
        write_png(&dir.join(format!("{n:02}.png")), 8);
    }

    let mut cache = ImageCache::new(64 * 1024 * 1024, 1);
    cache.set_target_size(16, 16);
    for n in 0..40 {
        cache.request(&dir.join(format!("{n:02}.png")), n);
    }
    cache.cancel_outside_window(0);

    let d = Directory::open_at(&dir.join("00.png")).unwrap();
    cache.sync_to_directory(&d);
    assert!(cache.inflight_len() <= 8, "far requests should be dropped");

    fs::remove_dir_all(&dir).unwrap();
}
