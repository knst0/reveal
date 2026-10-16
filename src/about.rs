use std::path::PathBuf;

pub const NAME: &str = "Reveal";
pub const DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");

pub const RUSTC: &str = env!("REVEAL_RUSTC");
pub const COMMIT: &str = env!("REVEAL_COMMIT");

pub struct Dependency {
    pub name: &'static str,
    pub version: &'static str,
    pub license: &'static str,
    pub repository: &'static str,
}

include!(concat!(env!("OUT_DIR"), "/dependencies.rs"));

pub struct Asset {
    pub name: &'static str,
    pub detail: &'static str,
    pub license: &'static str,
    pub repository: &'static str,
}

pub const ASSETS: &[Asset] = &[Asset {
    name: "Lucide",
    detail: "Toolbar and menu icons",
    license: "ISC",
    repository: "https://github.com/lucide-icons/lucide",
}];

pub fn version_line() -> String {
    if COMMIT.is_empty() {
        format!("Version {VERSION}")
    } else {
        format!("Version {VERSION} ({COMMIT})")
    }
}

pub fn features() -> Vec<&'static str> {
    let mut features = Vec::new();
    if cfg!(feature = "raw") {
        features.push("raw");
    }
    if cfg!(feature = "updates") {
        features.push("updates");
    }
    features
}

pub fn features_line() -> String {
    let features = features();
    if features.is_empty() { "none".to_owned() } else { features.join(", ") }
}

pub fn rustc_line() -> &'static str {
    if RUSTC.is_empty() { "unknown" } else { RUSTC }
}

pub fn format_count() -> usize {
    crate::formats::mime_list().len()
}

pub fn extension_count() -> usize {
    crate::formats::extensions().len()
}

pub fn icon_count() -> usize {
    crate::icons::LUCIDE_ICONS.len()
}

pub fn config_location() -> Option<PathBuf> {
    crate::config::config_path()
}

pub fn cache_location() -> Option<PathBuf> {
    crate::config::cache_path()
}

pub fn issues_url() -> String {
    format!("{REPOSITORY}/issues")
}

pub fn licenses_url() -> String {
    format!("{REPOSITORY}/blob/main/LICENSE")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dependencies_are_sorted_and_complete() {
        let names: Vec<&str> = DEPENDENCIES.iter().map(|d| d.name).collect();
        let mut sorted = names.clone();
        sorted.sort_unstable();
        assert_eq!(names, sorted);
        assert!(DEPENDENCIES.iter().all(|d| !d.license.is_empty()));
        assert!(DEPENDENCIES.iter().all(|d| d.repository.starts_with("https://")));
    }

    #[test]
    fn assets_are_credited() {
        assert!(!ASSETS.is_empty());
        assert!(ASSETS.iter().all(|a| !a.license.is_empty()));
        assert!(ASSETS.iter().all(|a| !a.detail.is_empty()));
        assert!(ASSETS.iter().all(|a| a.repository.starts_with("https://")));
        assert!(ASSETS.iter().any(|a| a.name == "Lucide"), "the bundled icons must be credited");
    }

    #[test]
    fn version_line_includes_commit_when_known() {
        let line = version_line();
        assert!(line.starts_with(&format!("Version {VERSION}")));
    }

    #[test]
    fn every_manifest_dependency_is_credited() {
        let manifest = include_str!("../Cargo.toml");
        let mut names = Vec::new();
        for line in manifest
            .lines()
            .skip_while(|line| line.trim() != "[dependencies]")
            .skip(1)
            .take_while(|line| !line.starts_with('['))
        {
            let Some((name, _)) = line.split_once('=') else {
                continue;
            };
            let name = name.trim();
            if !name.is_empty() {
                names.push(name);
            }
        }
        assert!(!names.is_empty(), "no dependencies parsed from the manifest");

        for name in names {
            // gpui and gpui_platform are renames of the gpui-unofficial crates.
            let credited = match name {
                "gpui" => "gpui-unofficial",
                "gpui_platform" => "gpui-platform-gpui-unofficial",
                other => other,
            };
            if (credited == "rawler" && !cfg!(feature = "raw"))
                || (credited == "self_update" && !cfg!(feature = "updates"))
            {
                continue;
            }
            assert!(
                DEPENDENCIES.iter().any(|d| d.name == credited),
                "{credited} is missing from the third-party list"
            );
        }
    }

    #[test]
    fn optional_dependencies_track_features() {
        let listed = DEPENDENCIES.iter().any(|d| d.name == "rawler");
        assert_eq!(listed, cfg!(feature = "raw"));
        let listed = DEPENDENCIES.iter().any(|d| d.name == "self_update");
        assert_eq!(listed, cfg!(feature = "updates"));
    }
}
