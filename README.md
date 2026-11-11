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


<p align="center">
  <a href="https://www.rust-lang.org/"><img alt="Rust" src="https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white"></a>
  <a href="https://www.gtk.org/"><img alt="GTK 4" src="https://img.shields.io/badge/GTK-4A86CF?style=for-the-badge&logo=gnome&logoColor=white"></a>
  <a href="https://gnome.pages.gitlab.gnome.org/libadwaita/"><img alt="Libadwaita" src="https://img.shields.io/badge/Libadwaita-3584E4?style=for-the-badge&logo=gnome&logoColor=white"></a>
</p>

Bellbird converts documents to structured LaTeX locally. It prepares the source for editing in a LaTeX application.

## At a glance

| Input | Templates | Result |
| --- | --- | --- |
| `.doc`, `.docx`, `.epub`, `.htm`, `.html`, `.markdown`, `.md`, `.odt`, `.org`, `.rst`, `.tex`, `.txt` | ABNT, IEEE, ACM, Elsevier, Springer Nature | One editable `.tex` file, ready to copy or export |

Bellbird uses author information from the source when available. It does not compile PDFs or export separate image or bibliography files; image descriptions are kept as text when available.

## Screenshots

<table>
  <tr>
    <td align="center"><strong>Document conversion</strong></td>
    <td align="center"><strong>Publication format selection</strong></td>
  </tr>
  <tr>
    <td><img src="data/screenshots/bellbird-01-editor.png" alt="Bellbird displaying generated LaTeX from a Word document" width="100%"></td>
    <td><img src="data/screenshots/bellbird-04-formats.png" alt="Bellbird format menu with ABNT, IEEE, ACM, Elsevier, and Springer Nature templates" width="100%"></td>
  </tr>
</table>

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
cargo clippy --all-targets -- --deny warnings
cargo test
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for ways to report problems, test the app, improve translations, or contribute code and documentation.

## Support

Sponsorship supports Bellbird development, maintenance, documentation, and releases.

<p align="left">
  <a href="https://github.com/sponsors/diegopn"><img alt="Become a sponsor" src="https://img.shields.io/badge/Become_a_sponsor-EA4AAA?style=for-the-badge&logo=githubsponsors&logoColor=white"></a>
</p>

## License

GPL-3.0-or-later. See [LICENSE](LICENSE).
