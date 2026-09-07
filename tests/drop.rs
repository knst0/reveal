use std::fs;
use std::path::PathBuf;

fn fixture(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("reveal-drop-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    for name in ["2.png", "10.png"] {
        let img = image::RgbaImage::from_pixel(8, 8, image::Rgba([1, 2, 3, 255]));
        image::DynamicImage::ImageRgba8(img).save(dir.join(name)).unwrap();
    }
    fs::write(dir.join("notes.txt"), "not an image").unwrap();
    dir
}

#[test]
fn accepts_images_and_folders_but_not_other_files() {
    let dir = fixture("accepts");
    assert!(reveal::drop::is_droppable(&dir));
    assert!(reveal::drop::is_droppable(&dir.join("2.png")));
    assert!(!reveal::drop::is_droppable(&dir.join("notes.txt")));
}

#[test]
fn resolves_a_dropped_image_to_itself() {
    let dir = fixture("file");
    let target = dir.join("10.png");
    assert_eq!(reveal::drop::resolve(std::slice::from_ref(&target)), Some(target));
}

#[test]
fn resolves_a_dropped_folder_to_its_first_image_in_natural_order() {
    let dir = fixture("folder");
    assert_eq!(reveal::drop::resolve(std::slice::from_ref(&dir)), Some(dir.join("2.png")));
}

#[test]
fn skips_unsupported_paths_and_takes_the_first_usable_one() {
    let dir = fixture("skip");
    let paths = vec![dir.join("notes.txt"), dir.join("10.png")];
    assert_eq!(reveal::drop::resolve(&paths), Some(dir.join("10.png")));
}

#[test]
fn resolves_nothing_when_no_path_is_usable() {
    let dir = fixture("none");
    assert_eq!(reveal::drop::resolve(&[dir.join("notes.txt")]), None);
}

#[test]
fn resolves_nothing_for_a_folder_without_images() {
    let dir = fixture("empty").join("sub");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("notes.txt"), "no images").unwrap();
    assert_eq!(reveal::drop::resolve(&[dir]), None);
}

#[test]
fn file_urls_become_paths() {
    let paths = reveal::drop::paths_from_urls(["file:///Users/me/holiday%20photo.png"]);
    assert_eq!(paths, vec![std::path::PathBuf::from("/Users/me/holiday photo.png")]);
}

#[test]
fn non_file_urls_are_ignored() {
    assert!(reveal::drop::paths_from_urls(["https://example.com/a.png"]).is_empty());
}

#[test]
fn percent_encoded_utf8_survives_the_round_trip() {
    let paths = reveal::drop::paths_from_urls(["file:///tmp/%D1%84%D0%BE%D1%82%D0%BE.jpg"]);
    assert_eq!(paths, vec![std::path::PathBuf::from("/tmp/фото.jpg")]);
}
