use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use reveal::decode::DecodedImage;
use reveal::render::{Resample, downscale_to_display_with};

fn synthetic(width: u32, height: u32) -> DecodedImage {
    let mut rgba = vec![0u8; (width as usize) * (height as usize) * 4];
    for (i, px) in rgba.chunks_exact_mut(4).enumerate() {
        let x = (i % width as usize) as u32;
        let y = (i / width as usize) as u32;
        px[0] = (x ^ y) as u8;
        px[1] = (x.wrapping_mul(3) ^ y.wrapping_mul(7)) as u8;
        px[2] = (x.wrapping_add(y)) as u8;
        px[3] = 255;
    }
    DecodedImage { width, height, rgba }
}

const SOURCES: &[(&str, u32, u32)] = &[
    ("2mp_1920x1080", 1920, 1080),
    ("12mp_4000x3000", 4000, 3000),
    ("45mp_8256x5504", 8256, 5504),
];

const VIEWPORT: (f32, f32) = (2560.0, 1440.0);

fn bench_downscale(c: &mut Criterion) {
    let mut group = c.benchmark_group("downscale_to_display");
    group.sample_size(20);

    for (name, w, h) in SOURCES {
        let src = synthetic(*w, *h);
        group.throughput(Throughput::Elements((*w as u64) * (*h as u64)));

        for resample in [Resample::Filtered, Resample::Nearest] {
            let label = match resample {
                Resample::Filtered => "filtered",
                Resample::Nearest => "nearest",
            };
            group.bench_with_input(BenchmarkId::new(label, name), &src, |b, src| {
                b.iter(|| {
                    black_box(downscale_to_display_with(
                        black_box(src),
                        black_box(VIEWPORT),
                        resample,
                    ))
                })
            });
        }
    }
    group.finish();
}

criterion_group!(benches, bench_downscale);
criterion_main!(benches);
