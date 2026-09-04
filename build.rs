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
        if let Err(e) = res.compile() {
            println!("cargo:warning=winres failed: {e}");
        }
    }
}
