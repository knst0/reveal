use std::path::{Path, PathBuf};

use reveal::decode::{DecodeRequest, Decoded, Orientation, decode};
use reveal::render::Resample;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/avif").join(name)
}

fn request<'a>(path: &'a Path, bytes: &'a [u8]) -> DecodeRequest<'a> {
    DecodeRequest {
        path,
        bytes,
        target_width: 256,
        target_height: 256,
        resample: Resample::Filtered,
    }
}

#[test]
fn identity_matrix_avif_preserves_rgb() {
    let expected = [200u8, 100, 50, 255];
    let pixels = vec![rgb::RGBA::new(200, 100, 50, 255); 48 * 24];
    let encoded = ravif::Encoder::new()
        .with_internal_color_model(ravif::ColorModel::RGB)
        .with_quality(100.0)
        .with_speed(10)
        .encode_rgba(ravif::Img::new(pixels.as_slice(), 48, 24))
        .unwrap();

    let out = decode(&request(Path::new("identity.avif"), &encoded.avif_file)).unwrap();
    let Decoded::Still(image) = out.decoded else { panic!("expected a still image") };

    for (actual, expected) in image.rgba[..4].iter().zip(expected) {
        assert!(
            (i32::from(*actual) - i32::from(expected)).abs() <= 8,
            "identity matrix must preserve RGB; expected {expected}, got {actual}"
        );
    }
}

#[test]
fn animated_avif_with_inter_frames_decodes() {
    let path = fixture("colors-animated-12bpc-keyframes-0-2-3.avif");
    let bytes = std::fs::read(&path).unwrap();

    let out = decode(&request(&path, &bytes)).expect("an inter-frame sequence must decode");
    let Decoded::Animation(frames) = out.decoded else { panic!("expected an animation") };
    assert_eq!(frames.len(), 5);
}

#[test]
fn animated_avif_fixtures_decode_every_frame() {
    for name in ["colors-animated-8bpc.avif", "colors-animated-8bpc-alpha-exif-xmp.avif"] {
        let path = fixture(name);
        let bytes = std::fs::read(&path).unwrap();
        let out = decode(&request(&path, &bytes)).unwrap_or_else(|e| panic!("{name}: {e}"));
        let Decoded::Animation(frames) = out.decoded else { panic!("{name}: expected animation") };
        assert!(frames.len() > 1, "{name} decoded {} frames", frames.len());
    }
}

#[test]
fn container_rotation_is_reported_as_orientation() {
    let path = fixture("abc_color_irot_alpha_irot.avif");
    let bytes = std::fs::read(&path).unwrap();

    let out = decode(&request(&path, &bytes)).expect("the fixture must decode");
    assert_eq!(
        out.orientation,
        Orientation::Rotate90,
        "an irot property must drive the presented orientation"
    );
}
