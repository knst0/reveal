use std::process::Command;

use super::{Error, Outcome};
use crate::formats::mime_list;

const DESKTOP_FILE: &str = "reveal.desktop";

fn applications_dir() -> Option<std::path::PathBuf> {
    let base = match std::env::var_os("XDG_DATA_HOME") {
        Some(dir) if !dir.is_empty() => std::path::PathBuf::from(dir),
        _ => std::path::PathBuf::from(std::env::var_os("HOME")?).join(".local/share"),
    };
    Some(base.join("applications"))
}

fn install_desktop_entry(mimes: &[&str]) -> Result<(), Error> {
    let dir = applications_dir()
        .ok_or_else(|| Error::Failed("could not locate the applications directory.".to_owned()))?;
    std::fs::create_dir_all(&dir)?;

    let exe = std::env::current_exe()?;
    let entry = format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name=Reveal\n\
         Exec=\"{}\" %f\n\
         Icon=reveal\n\
         Terminal=false\n\
         Categories=Graphics;Viewer;\n\
         MimeType={};\n",
        exe.display(),
        mimes.join(";"),
    );
    std::fs::write(dir.join(DESKTOP_FILE), entry)?;

    match Command::new("update-desktop-database").arg(&dir).status() {
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(Error::Io(e)),
    }
    Ok(())
}

pub fn register() -> Result<Outcome, Error> {
    let mimes = mime_list();
    install_desktop_entry(&mimes)?;
    let mut registered = 0;
    let mut failures = Vec::new();

    for mime in &mimes {
        let status = Command::new("xdg-mime").args(["default", DESKTOP_FILE, mime]).status();
        match status {
            Ok(status) if status.success() => registered += 1,
            Ok(_) => failures.push(*mime),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(Error::Failed("xdg-mime is not installed.".to_owned()));
            }
            Err(e) => return Err(Error::Io(e)),
        }
    }

    if registered == 0 {
        return Err(Error::Failed("xdg-mime rejected every format.".to_owned()));
    }

    Ok(Outcome {
        registered,
        needs_user_action: (!failures.is_empty())
            .then(|| format!("Could not claim: {}.", failures.join(", "))),
    })
}
