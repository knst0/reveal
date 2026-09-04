use std::path::PathBuf;

use reveal::decode::is_supported;
use reveal::formats::{FORMATS, extensions, mime_list};

#[test]
fn every_advertised_format_is_actually_supported() {
    for f in FORMATS {
        let path = PathBuf::from(format!("sample.{}", f.extension));
        assert!(
            is_supported(&path),
            "advertised .{} but the decoder layer rejects it",
            f.extension
        );
    }
}

#[test]
fn mime_list_is_sorted_and_deduplicated() {
    let mimes = mime_list();
    let mut sorted = mimes.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(mimes, sorted);
    assert!(mimes.contains(&"image/png"));
    assert!(mimes.contains(&"image/jxl"));
    assert!(mimes.contains(&"image/svg+xml"));
}

#[test]
fn jpeg_variants_share_one_mime_type() {
    let jpegs: Vec<_> =
        FORMATS.iter().filter(|f| f.extension == "jpg" || f.extension == "jpeg").collect();
    assert_eq!(jpegs.len(), 2);
    assert_eq!(jpegs[0].mime, jpegs[1].mime);
    assert_eq!(mime_list().iter().filter(|m| **m == "image/jpeg").count(), 1);
}

#[test]
fn extensions_are_lowercase_and_unique() {
    let exts = extensions();
    let mut seen = exts.clone();
    seen.sort_unstable();
    seen.dedup();
    assert_eq!(exts.len(), seen.len(), "duplicate extension entry");
    for e in exts {
        assert_eq!(e, e.to_ascii_lowercase());
        assert!(!e.starts_with('.'), "store bare extensions: {e}");
    }
}

#[test]
fn the_macos_icon_is_a_valid_icns_container() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/resource/macos/reveal.icns");
    let data = std::fs::read(path).expect("resource/macos/reveal.icns exists");

    assert_eq!(&data[0..4], b"icns");
    let declared = u32::from_be_bytes(data[4..8].try_into().unwrap()) as usize;
    assert_eq!(declared, data.len(), "header length must match file size");

    let mut offset = 8;
    let mut chunks = 0;
    while offset < data.len() {
        let len = u32::from_be_bytes(data[offset + 4..offset + 8].try_into().unwrap()) as usize;
        assert!(len >= 8 && offset + len <= data.len(), "chunk overruns the file");
        assert_eq!(&data[offset + 8..offset + 12], b"\x89PNG", "chunk payload must be a PNG");
        offset += len;
        chunks += 1;
    }
    assert_eq!(offset, data.len(), "chunks must tile the file exactly");
    assert!(chunks >= 6, "expected the full size ladder, got {chunks}");
}
