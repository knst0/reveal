# Reveal

[![Crates.io](https://img.shields.io/crates/v/reveal.svg)](https://crates.io/crates/reveal)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A fast image viewer, built with Rust and GPUI.

<img width="1376" height="926" alt="screenshot2_transparent" src="https://github.com/user-attachments/assets/ea0da908-6125-49d7-a696-8f5bc254118c" />

## Install

### Windows

#### Installer

Download and run the [installer](https://github.com/knst0/reveal/releases/latest/download/reveal-setup.exe).

#### Portable

Download the latest [release](https://github.com/knst0/reveal/releases/latest/download/reveal-x86_64-pc-windows-msvc.zip).

### macOS

Install with [Homebrew](https://brew.sh/):

```shell
brew install --cask knst0/reveal/reveal
```

The binaries are ad-hoc signed. If Gatekeeper blocks the app:

```shell
xattr -dr com.apple.quarantine /Applications/Reveal.app
```

### Linux

#### Flatpak

Add the Reveal repository and install the app:

```shell
flatpak remote-add --if-not-exists reveal https://knst0.github.io/reveal/reveal.flatpakrepo
flatpak install reveal io.github.knst0.reveal
```

### Cargo

Install with [cargo-binstall](https://github.com/cargo-bins/cargo-binstall):

```shell
cargo binstall reveal
```

or build from source:

```shell
cargo install reveal
```

## License

This project is licensed under the terms of the [MIT License](/LICENSE).
