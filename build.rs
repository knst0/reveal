fn main() {
    emit_build_info();

    #[cfg(windows)]
    {
        println!("cargo:rerun-if-changed=resource/reveal.ico");
        let icon = std::path::Path::new("resource/reveal.ico");
        let mut res = winres::WindowsResource::new();
        if icon.exists() {
            res.set_icon("resource/reveal.ico");
        }
        res.set("ProductName", "Reveal");
        res.set("FileDescription", "Reveal");
        res.set("InternalName", "Reveal");
        res.set("OriginalFilename", "reveal.exe");
        if let Err(e) = res.compile() {
            println!("cargo:warning=winres failed: {e}");
        }
    }
}

fn emit_build_info() {
    let target = std::env::var("TARGET").unwrap_or_default();
    let profile = std::env::var("PROFILE").unwrap_or_default();
    println!("cargo:rustc-env=REVEAL_TARGET={target}");
    println!("cargo:rustc-env=REVEAL_PROFILE={profile}");
    println!("cargo:rustc-env=REVEAL_RUSTC={}", rustc_version());
    println!("cargo:rustc-env=REVEAL_COMMIT={}", git_commit());
}

fn rustc_version() -> String {
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_owned());
    std::process::Command::new(rustc)
        .arg("--version")
        .output()
        .ok()
        .filter(|out| out.status.success())
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .map(|text| text.trim().to_owned())
        .unwrap_or_default()
}

fn git_commit() -> String {
    println!("cargo:rerun-if-changed=.git/HEAD");
    std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|out| out.status.success())
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .map(|text| text.trim().to_owned())
        .unwrap_or_default()
}
