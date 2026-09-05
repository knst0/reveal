use std::path::PathBuf;

pub fn report_path() -> PathBuf {
    directories::ProjectDirs::from("", "", "reveal")
        .map(|d| d.data_local_dir().to_path_buf())
        .unwrap_or_else(std::env::temp_dir)
        .join("reveal-panic.txt")
}

pub fn format_report(info: &str, version: &str, os: &str) -> String {
    format!(
        "reveal {version} panicked\n\
         os: {os}\n\
         time: {:?}\n\
         \n{info}\n",
        std::time::SystemTime::now()
    )
}

thread_local! {
    static SUPPRESSED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

pub fn suppress_during<T>(f: impl FnOnce() -> T) -> T {
    SUPPRESSED.with(|s| s.set(true));
    let out = f();
    SUPPRESSED.with(|s| s.set(false));
    out
}

pub fn install() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if SUPPRESSED.with(std::cell::Cell::get) {
            log::warn!("recovered from a panic: {info}");
            return;
        }
        let report =
            format_report(&info.to_string(), env!("CARGO_PKG_VERSION"), std::env::consts::OS);
        let path = report_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if std::fs::write(&path, &report).is_ok() {
            eprintln!("reveal crashed; wrote a report to {}", path.display());
        } else {
            eprintln!("{report}");
        }
        default_hook(info);
    }));
}
