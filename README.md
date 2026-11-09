<p align="center">
  <a href="https://diegopn.github.io/bellbird-site/">
    <img src="data/icons/hicolor/scalable/apps/io.github.diegopn.Bellbird.svg" alt="Bellbird icon" width="112" />
  </a>
</p>

<h1 align="center">Bellbird</h1>

<p align="center">An offline GNOME app that converts supported documents into structured LaTeX. It processes files locally and exports a single <code>.tex</code> file.</p>

<p align="center">
  <a href="https://github.com/diegopn/bellbird/releases/latest">Download</a> ·
  <a href="https://diegopn.github.io/bellbird-site/">Website</a> ·
  <a href="https://diegopn.github.io/bellbird-site/manual/">User manual</a>
</p>


## Features

- Opens legacy Word `.doc` files as well as `.docx`, `.epub`, `.htm`, `.html`, `.markdown`, `.md`, `.odt`, `.org`, `.rst`, `.tex`, and `.txt` files.
- Generates documents using ABNT (abnTeX2), IEEE (IEEEtran), ACM (acmart), Elsevier (elsarticle), and Springer Nature (sn-jnl) formats.
- Displays the generated LaTeX with line numbers, and lets you copy or export the code.
- Suggests the Downloads folder and a `.tex` filename when exporting; you can choose a different location and filename.
- Uses author information found in the source document. If no author is available, the LaTeX output includes a placeholder.
- Exports a single `.tex` file. It does not generate PDFs or separate image or bibliography files. Images and attachments are omitted; image descriptions are preserved as text when available.

Bellbird does not compile LaTeX. To create a PDF, open the exported file with a LaTeX distribution that includes the class and packages required by the selected format.

## Building from source on Linux

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
cargo clippy --all-targets -- -D warnings
cargo test
```

## License

GPL-3.0-or-later. See [`LICENSE`](LICENSE).

## More links

- [Source repository](https://github.com/diegopn/bellbird)
- [Contributing guide](CONTRIBUTING.md)
- [Report an issue](https://github.com/diegopn/bellbird/issues)
- [Creator's website](https://diegopn.github.io/)
