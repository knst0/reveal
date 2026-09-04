use std::path::{Path, PathBuf};

use crate::decode::supported_extensions;

pub async fn pick_image(start_in: Option<PathBuf>) -> Option<PathBuf> {
    let extensions = supported_extensions();
    let mut dialog = rfd::AsyncFileDialog::new()
        .set_title("Open Image")
        .add_filter("Images", &extensions)
        .add_filter("All Files", &["*"]);
    if let Some(dir) = start_in.as_deref().filter(|d| d.is_dir()) {
        dialog = dialog.set_directory(dir);
    }
    dialog.pick_file().await.map(|handle| handle.path().to_path_buf())
}

pub async fn pick_folder(start_in: Option<PathBuf>) -> Option<PathBuf> {
    let mut dialog = rfd::AsyncFileDialog::new().set_title("Open Folder");
    if let Some(dir) = start_in.as_deref().filter(|d| d.is_dir()) {
        dialog = dialog.set_directory(dir);
    }
    dialog.pick_folder().await.map(|handle| handle.path().to_path_buf())
}

pub fn start_directory(current: Option<&Path>) -> Option<PathBuf> {
    current?.parent().map(Path::to_path_buf)
}

#[cfg(test)]
mod tests {
    use super::start_directory;
    use crate::decode::{is_supported, supported_extensions};
    use std::path::{Path, PathBuf};

    #[test]
    fn dialog_filter_only_lists_openable_extensions() {
        let extensions = supported_extensions();
        assert!(extensions.contains(&"png"));
        for ext in &extensions {
            assert!(is_supported(&PathBuf::from(format!("a.{ext}"))), "{ext} not openable");
        }
    }

    #[test]
    fn start_directory_is_the_parent_of_the_open_image() {
        let parent = start_directory(Some(Path::new("/photos/trip/a.png")));
        assert_eq!(parent, Some(PathBuf::from("/photos/trip")));
    }

    #[test]
    fn start_directory_is_none_without_an_open_image() {
        assert_eq!(start_directory(None), None);
    }
}
