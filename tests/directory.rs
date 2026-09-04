use std::fs;
use std::path::{Path, PathBuf};

use reveal::directory::{Directory, Navigation};

fn write_png(path: &Path) {
    let img = image::RgbaImage::from_pixel(2, 2, image::Rgba([1, 2, 3, 255]));
    image::DynamicImage::ImageRgba8(img).save(path).unwrap();
}

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("reveal-dirtest-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn names(d: &Directory) -> Vec<String> {
    d.entries().iter().map(|p| p.file_name().unwrap().to_string_lossy().into_owned()).collect()
}

#[test]
fn sorts_naturally_and_filters_unsupported() {
    let dir = temp_dir("sort");
    for n in ["a10.png", "a2.png", "a1.png"] {
        write_png(&dir.join(n));
    }
    fs::write(dir.join("notes.txt"), b"ignore me").unwrap();

    let d = Directory::open_at(&dir).unwrap();
    assert_eq!(names(&d), vec!["a1.png", "a2.png", "a10.png"]);
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn starts_at_requested_file_and_wraps_both_ways() {
    let dir = temp_dir("nav");
    for n in ["1.png", "2.png", "3.png"] {
        write_png(&dir.join(n));
    }

    let mut d = Directory::open_at(&dir.join("3.png")).unwrap();
    assert_eq!(d.current_index(), 2);

    assert_eq!(d.navigate(Navigation::Next).unwrap().file_name().unwrap(), "1.png");
    assert_eq!(d.navigate(Navigation::Prev).unwrap().file_name().unwrap(), "3.png");
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn refresh_keeps_current_file_when_others_are_added() {
    let dir = temp_dir("refresh");
    for n in ["b.png", "c.png"] {
        write_png(&dir.join(n));
    }
    let mut d = Directory::open_at(&dir.join("c.png")).unwrap();
    assert_eq!(d.current_index(), 1);

    write_png(&dir.join("a.png"));
    d.refresh().unwrap();

    assert_eq!(d.len(), 3);
    assert_eq!(d.current().unwrap().file_name().unwrap(), "c.png");
    assert_eq!(d.current_index(), 2);
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn refresh_survives_current_file_deletion() {
    let dir = temp_dir("delete");
    for n in ["x.png", "y.png"] {
        write_png(&dir.join(n));
    }
    let mut d = Directory::open_at(&dir.join("y.png")).unwrap();
    fs::remove_file(dir.join("y.png")).unwrap();
    d.refresh().unwrap();

    assert_eq!(d.len(), 1);
    assert_eq!(d.current().unwrap().file_name().unwrap(), "x.png");
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn empty_directory_navigates_to_nothing() {
    let dir = temp_dir("empty");
    let mut d = Directory::open_at(&dir).unwrap();
    assert!(d.is_empty());
    assert!(d.current().is_none());
    assert!(d.navigate(Navigation::Next).is_none());
    fs::remove_dir_all(&dir).unwrap();
}
