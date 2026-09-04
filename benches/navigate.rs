use std::hint::black_box;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use criterion::{Criterion, criterion_group, criterion_main};
use reveal::directory::Navigation;
use reveal::viewer::Viewer;

const COUNT: usize = 24;
const VIEWPORT: (f32, f32) = (2560.0, 1440.0);

fn corpus(dir: &std::path::Path, width: u32, height: u32) -> Vec<PathBuf> {
    std::fs::create_dir_all(dir).unwrap();
    (0..COUNT)
        .map(|i| {
            let path = dir.join(format!("img_{i:03}.png"));
            if !path.exists() {
                let mut buf = image::RgbaImage::new(width, height);
                for (x, y, px) in buf.enumerate_pixels_mut() {
                    let t = i as u32;
                    *px = image::Rgba([
                        (x ^ y).wrapping_add(t) as u8,
                        (x.wrapping_mul(3) ^ y).wrapping_add(t) as u8,
                        (x.wrapping_add(y)).wrapping_add(t) as u8,
                        255,
                    ]);
                }
                buf.save(&path).unwrap();
            }
            path
        })
        .collect()
}

fn open_at(files: &[PathBuf]) -> Viewer {
    let mut viewer = Viewer::new();
    viewer.set_viewport(VIEWPORT.0, VIEWPORT.1);
    viewer.open(&files[0]).unwrap();
    viewer
}

fn bench_navigation(c: &mut Criterion) {
    let root = std::env::temp_dir().join("reveal_bench_nav");
    let files = corpus(&root, 4000, 3000);

    let mut group = c.benchmark_group("navigate");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(20));

    // Total UI-thread time consumed by a fast burst of key presses.
    // This is what the user feels as a freeze.
    group.bench_function("burst_next_12mp", |b| {
        b.iter_custom(|iters| {
            let mut total = Duration::ZERO;
            for _ in 0..iters {
                let mut viewer = open_at(&files);
                let start = Instant::now();
                for _ in 0..COUNT - 1 {
                    viewer.navigate(black_box(Navigation::Next));
                    black_box(viewer.render_image());
                }
                total += start.elapsed();
            }
            total
        })
    });

    // Worst-case single-keypress latency during a burst: the metric that
    // decides whether a frame is dropped.
    group.bench_function("worst_keypress_12mp", |b| {
        b.iter_custom(|iters| {
            let mut worst = Duration::ZERO;
            for _ in 0..iters {
                let mut viewer = open_at(&files);
                for _ in 0..COUNT - 1 {
                    let start = Instant::now();
                    viewer.navigate(black_box(Navigation::Next));
                    black_box(viewer.render_image());
                    worst = worst.max(start.elapsed());
                }
            }
            worst * iters as u32
        })
    });

    // Steady paging that lets the loader keep up, including tick() work.
    group.bench_function("paced_next_12mp", |b| {
        b.iter_custom(|iters| {
            let mut total = Duration::ZERO;
            for _ in 0..iters {
                let mut viewer = open_at(&files);
                let start = Instant::now();
                for _ in 0..COUNT - 1 {
                    viewer.navigate(black_box(Navigation::Next));
                    viewer.settle();
                    black_box(viewer.tick(Instant::now()));
                    black_box(viewer.render_image());
                }
                total += start.elapsed();
            }
            total
        })
    });

    group.finish();
}

criterion_group!(benches, bench_navigation);
criterion_main!(benches);
