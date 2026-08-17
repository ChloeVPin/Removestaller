# Removestaller

Removestaller is a minimal GTK4 and libadwaita application for removing installed applications from supported Linux package formats.

Version: 1.0.2

## Build

Requirements:

- Rust and Cargo
- GTK4 4.10 or newer
- libadwaita 1.5 or newer
- Meson and Ninja for packaged builds

Run the development build:

```sh
cargo run
```

Removestaller can inspect and remove applications from APT, DNF, Zypper, Pacman, Flatpak, Snap, AppImage, Python, Node.js, and Rust sources when their tools are available.

Licensed under GPL-3.0-or-later.
