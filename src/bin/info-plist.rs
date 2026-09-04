fn main() {
    let version = std::env::args().nth(1).unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_owned());
    print!("{}", reveal::formats::info_plist(&version));
}
