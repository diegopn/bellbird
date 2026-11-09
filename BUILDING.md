# Building Bellbird

## Native Linux build

Requirements: Rust 1.92 or later, GTK 4.10 or later, libadwaita 1.5 or later, and GNU gettext. Package versions vary by distribution release; check these minimums before building.

### Fedora and compatible RPM distributions

```sh
sudo dnf install gcc rustup pkgconf-pkg-config gtk4-devel libadwaita-devel gettext
```

On Fedora, run `rustup-init` once after installing `rustup`.

### Debian, Ubuntu, and derivatives

```sh
sudo apt-get install build-essential rustup pkg-config gettext libgtk-4-dev libadwaita-1-dev
```

### Arch-based distributions

```sh
sudo pacman -S --needed base-devel rustup pkgconf gettext gtk4 libadwaita
```

### openSUSE Tumbleweed

```sh
sudo zypper install gcc rustup pkgconf-pkg-config gettext-tools gtk4-devel libadwaita-devel
```

### Build and run

On the first build, Cargo downloads dependencies that are not already in its cache.

```sh
cargo run --locked
```

### Project checks

```sh
cargo fmt --all -- --check
msgfmt --check po/pt_BR.po -o /tmp/bellbird-pt_BR.mo
msgfmt --check po/es.po -o /tmp/bellbird-es.mo
msgfmt --check po/fr.po -o /tmp/bellbird-fr.mo
msgfmt --check po/it.po -o /tmp/bellbird-it.mo
cargo clippy --all-targets -- --deny warnings
cargo test
```
