use std::path::{Path, PathBuf};
use std::process::Command;

use super::{Error, Outcome};
use crate::formats::FORMATS;

fn bundle_root() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let macos_dir = exe.parent()?;
    if macos_dir.file_name()? != "MacOS" {
        return None;
    }
    let contents = macos_dir.parent()?;
    if contents.file_name()? != "Contents" {
        return None;
    }
    let app = contents.parent()?;
    (app.extension()? == "app").then(|| app.to_path_buf())
}

fn lsregister(app: &Path) -> Result<(), Error> {
    const TOOL: &str = "/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister";
    let output =
        Command::new(TOOL).args(["-f", "-R", "-trusted"]).arg(app).output().map_err(Error::Io)?;
    if !output.status.success() {
        return Err(Error::Failed(String::from_utf8_lossy(&output.stderr).trim().to_owned()));
    }
    Ok(())
}

pub fn register() -> Result<Outcome, Error> {
    let app = bundle_root().ok_or(Error::NotBundled)?;
    lsregister(&app)?;
    Ok(Outcome {
        registered: FORMATS.len(),
        needs_user_action: Some(
            "Reveal is registered with Launch Services. Pick it per format with Get Info \u{203a} Open With \u{203a} Change All."
                .to_owned(),
        ),
    })
}
