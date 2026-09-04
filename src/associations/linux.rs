use std::process::Command;

use super::{Error, Outcome};
use crate::formats::mime_list;

pub fn register() -> Result<Outcome, Error> {
    let mimes = mime_list();
    let mut registered = 0;
    let mut failures = Vec::new();

    for mime in &mimes {
        let status = Command::new("xdg-mime").args(["default", "reveal.desktop", mime]).status();
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
