use std::fmt;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

#[derive(Debug)]
pub enum Error {
    Unsupported,
    NotBundled,
    Io(std::io::Error),
    Failed(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported => write!(f, "Setting defaults is not supported on this platform."),
            Self::NotBundled => {
                write!(f, "Move Reveal.app into Applications and run it from there to register it.")
            }
            Self::Io(e) => write!(f, "{e}"),
            Self::Failed(msg) => write!(f, "{msg}"),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

pub struct Outcome {
    pub registered: usize,
    pub needs_user_action: Option<String>,
}

/// Human-readable result of registering the app as a default image viewer.
pub fn notice() -> String {
    match register() {
        Ok(outcome) => {
            let mut text = format!("Registered {} formats.", outcome.registered);
            if let Some(extra) = outcome.needs_user_action {
                text.push(' ');
                text.push_str(&extra);
            }
            text
        }
        Err(e) => format!("Could not set defaults: {e}"),
    }
}

pub fn register() -> Result<Outcome, Error> {
    #[cfg(target_os = "windows")]
    return windows::register();
    #[cfg(target_os = "macos")]
    return macos::register();
    #[cfg(target_os = "linux")]
    return linux::register();
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    Err(Error::Unsupported)
}
