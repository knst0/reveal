use std::io::Write;

use resvg::tiny_skia;
use usvg::Transform;

const ICNS_TYPES: &[(&[u8; 4], u32)] = &[
    (b"icp4", 16),
    (b"icp5", 32),
    (b"icp6", 64),
    (b"ic07", 128),
    (b"ic08", 256),
    (b"ic09", 512),
    (b"ic10", 1024),
    (b"ic11", 32),
    (b"ic12", 64),
    (b"ic13", 256),
    (b"ic14", 512),
];

fn render(tree: &usvg::Tree, size: u32) -> Vec<u8> {
    let mut pixmap = tiny_skia::Pixmap::new(size, size).expect("pixmap");
    let scale = size as f32 / tree.size().width();
    resvg::render(tree, Transform::from_scale(scale, scale), &mut pixmap.as_mut());
    pixmap.encode_png().expect("encode png")
}

fn main() {
    let svg = std::fs::read("resource/reveal.svg").expect("read resource/reveal.svg");
    let tree = usvg::Tree::from_data(&svg, &usvg::Options::default()).expect("parse svg");

    let mut body: Vec<u8> = Vec::new();
    for (kind, size) in ICNS_TYPES {
        let png = render(&tree, *size);
        body.extend_from_slice(*kind);
        body.extend_from_slice(&(png.len() as u32 + 8).to_be_bytes());
        body.extend_from_slice(&png);
    }

    let mut icns: Vec<u8> = Vec::new();
    icns.extend_from_slice(b"icns");
    icns.extend_from_slice(&(body.len() as u32 + 8).to_be_bytes());
    icns.extend_from_slice(&body);

    std::fs::create_dir_all("resource/macos").expect("create resource/macos");
    let mut out = std::fs::File::create("resource/macos/reveal.icns").expect("create reveal.icns");
    out.write_all(&icns).expect("write reveal.icns");

    println!("wrote resource/macos/reveal.icns ({} bytes)", icns.len());
}
