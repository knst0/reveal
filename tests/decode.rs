use std::path::Path;

use reveal::decode::{self, DecodeRequest, Decoded, Orientation};

fn req<'a>(path: &'a Path, bytes: &'a [u8]) -> DecodeRequest<'a> {
    DecodeRequest { path, bytes, target_width: 256, target_height: 256 }
}

fn png_bytes(w: u32, h: u32) -> Vec<u8> {
    let img = image::RgbaImage::from_pixel(w, h, image::Rgba([10, 20, 30, 255]));
    let mut out = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(img).write_to(&mut out, image::ImageFormat::Png).unwrap();
    out.into_inner()
}

#[test]
fn decodes_png_still() {
    let bytes = png_bytes(64, 32);
    let path = Path::new("sample.png");
    let out = decode::decode(&req(path, &bytes)).unwrap();
    assert_eq!(out.orientation, Orientation::Normal);
    match out.decoded {
        Decoded::Still(img) => {
            assert_eq!((img.width, img.height), (64, 32));
            assert_eq!(img.rgba.len(), 64 * 32 * 4);
            assert_eq!(&img.rgba[..4], &[10, 20, 30, 255]);
        }
        _ => panic!("expected still"),
    }
}

#[test]
fn sniffs_content_over_wrong_extension() {
    let bytes = png_bytes(8, 8);
    let path = Path::new("mislabeled.jpg");
    let out = decode::decode(&req(path, &bytes)).unwrap();
    assert!(matches!(out.decoded, Decoded::Still(_)));
}

#[test]
fn rasterizes_svg_at_target_size() {
    let svg = br#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><rect width="10" height="10" fill="red"/></svg>"#;
    let path = Path::new("sample.svg");
    let out = decode::decode(&req(path, svg)).unwrap();
    match out.decoded {
        Decoded::Still(img) => {
            assert_eq!((img.width, img.height), (256, 256));
            assert_eq!(&img.rgba[..4], &[255, 0, 0, 255]);
        }
        _ => panic!("expected still"),
    }
}

#[test]
fn decodes_animated_gif_frames() {
    let mut bytes = Vec::new();
    {
        let mut enc = image::codecs::gif::GifEncoder::new(&mut bytes);
        enc.set_repeat(image::codecs::gif::Repeat::Infinite).unwrap();
        for _ in 0..3 {
            let buf = image::RgbaImage::from_pixel(8, 8, image::Rgba([0, 0, 255, 255]));
            enc.encode_frame(image::Frame::from_parts(
                buf,
                0,
                0,
                image::Delay::from_numer_denom_ms(100, 1),
            ))
            .unwrap();
        }
    }
    let path = Path::new("anim.gif");
    let out = decode::decode(&req(path, &bytes)).unwrap();
    match out.decoded {
        Decoded::Animation(frames) => {
            assert_eq!(frames.len(), 3);
            assert!(frames[0].delay.as_millis() > 0);
        }
        _ => panic!("expected animation"),
    }
}

#[test]
fn supported_extensions() {
    assert!(decode::is_supported(Path::new("a.png")));
    assert!(decode::is_supported(Path::new("a.svg")));
    assert!(!decode::is_supported(Path::new("a.txt")));
}

#[test]
fn decodes_jpeg_xl_still() {
    use zune_core::bit_depth::BitDepth;
    use zune_core::colorspace::ColorSpace;
    use zune_core::options::EncoderOptions;
    use zune_jpegxl::JxlSimpleEncoder;

    let (w, h) = (16usize, 8usize);
    let pixels: Vec<u8> = std::iter::repeat_n([200u8, 100, 50], w * h).flatten().collect();
    let options = EncoderOptions::new(w, h, ColorSpace::RGB, BitDepth::Eight);
    let mut bytes = Vec::new();
    JxlSimpleEncoder::new(&pixels, options).encode(&mut bytes).expect("jxl encode");

    let path = Path::new("sample.jxl");
    let out = decode::decode(&req(path, &bytes)).unwrap();
    match out.decoded {
        Decoded::Still(img) => {
            assert_eq!((img.width, img.height), (w as u32, h as u32));
            assert_eq!(img.rgba.len(), w * h * 4);
            assert_eq!(img.rgba[3], 255);
            let (r, g, b) = (img.rgba[0], img.rgba[1], img.rgba[2]);
            assert!(r > g && g > b, "unexpected color {r},{g},{b}");
        }
        _ => panic!("expected still"),
    }
}

fn avif_bytes(w: u32, h: u32, rgba: [u8; 4]) -> Vec<u8> {
    let px = rgb::RGBA::new(rgba[0], rgba[1], rgba[2], rgba[3]);
    let pixels = vec![px; w as usize * h as usize];
    let img = ravif::Img::new(pixels.as_slice(), w as usize, h as usize);
    ravif::Encoder::new()
        .with_quality(100.0)
        .with_speed(10)
        .encode_rgba(img)
        .expect("avif encode")
        .avif_file
}

#[test]
fn decodes_avif_still() {
    let bytes = avif_bytes(48, 24, [200, 100, 50, 255]);
    let path = Path::new("sample.avif");
    let out = decode::decode(&req(path, &bytes)).unwrap();
    match out.decoded {
        Decoded::Still(img) => {
            assert_eq!((img.width, img.height), (48, 24));
            assert_eq!(img.rgba.len(), 48 * 24 * 4);
            for px in img.rgba.chunks_exact(4) {
                assert_eq!(px[3], 255);
                let (r, g, b) = (i32::from(px[0]), i32::from(px[1]), i32::from(px[2]));
                assert!((r - 200).abs() <= 8, "red {r} is not close to 200");
                assert!((g - 100).abs() <= 8, "green {g} is not close to 100");
                assert!((b - 50).abs() <= 8, "blue {b} is not close to 50");
            }
        }
        _ => panic!("expected still"),
    }
}

#[test]
fn avif_is_a_supported_extension() {
    assert!(decode::is_supported(Path::new("photo.avif")));
    assert!(decode::supported_extensions().contains(&"avif"));
}

#[test]
fn renders_svg_text_with_system_fonts() {
    let svg = br#"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="60"><rect width="200" height="60" fill="white"/><text x="10" y="40" font-family="sans-serif" font-size="36" fill="black">Hello</text></svg>"#;
    let path = Path::new("text.svg");
    let out = decode::decode(&req(path, svg)).unwrap();
    let Decoded::Still(img) = out.decoded else { panic!("expected still") };
    let dark = img.rgba.chunks_exact(4).filter(|p| p[0] < 128 && p[3] > 0).count();
    assert!(dark > 0, "svg <text> rendered no glyph pixels");
}
