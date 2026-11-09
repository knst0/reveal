use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use reveal::decode::{DecodeRequest, decode};

fn encoded_png(width: u32, height: u32) -> Vec<u8> {
    let mut buf = image::RgbaImage::new(width, height);
    for (x, y, px) in buf.enumerate_pixels_mut() {
        *px = image::Rgba([(x ^ y) as u8, (x.wrapping_mul(3) ^ y) as u8, (x + y) as u8, 255]);
    }
    let mut out = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(buf).write_to(&mut out, image::ImageFormat::Png).unwrap();
    out.into_inner()
}

fn bench_decode(c: &mut Criterion) {
    let mut group = c.benchmark_group("decode");
    group.sample_size(10);

    let bytes = encoded_png(4000, 3000);
    let path = std::path::Path::new("bench.png");

    group.bench_function("png_12mp", |b| {
        b.iter(|| {
            black_box(
                decode(&DecodeRequest {
                    path: black_box(path),
                    bytes: black_box(&bytes),
                    target_width: 2560,
                    target_height: 1440,
                })
                .unwrap(),
            )
        })
    });

    group.finish();
}

criterion_group!(benches, bench_decode);
criterion_main!(benches);
