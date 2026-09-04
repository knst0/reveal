fn main() {
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
