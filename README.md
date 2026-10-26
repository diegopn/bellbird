# Bellbird

Bellbird is a GNOME app that converts documents into structured LaTeX. Conversion happens locally; the app does not require an account or an internet connection to process files.

## Features

- Opens `.docx`, `.epub`, `.htm`, `.html`, `.markdown`, `.md`, `.odt`, `.org`, `.rst`, `.tex`, and `.txt` files.
- Generates documents using ABNT (abnTeX2), IEEE (IEEEtran), ACM (acmart), Elsevier (elsarticle), and Springer Nature (sn-jnl) formats.
- Displays the generated LaTeX with line numbers, and lets you copy or export the code.
- Suggests the Downloads folder and a `.tex` filename when exporting; you can choose a different location and filename.
- Uses author information found in the source document. If no author is available, the LaTeX output includes a placeholder.
- Exports a single `.tex` file. It does not generate PDFs or separate image or bibliography files. Images and attachments are omitted; image descriptions are preserved as text when available.

Bellbird does not compile LaTeX. To create a PDF, open the exported file with a LaTeX distribution that includes the class and packages required by the selected format.

## Build on Fedora

Requirements: Rust 1.92 or later, GTK 4, libadwaita, and GNU gettext. Install the build dependencies on Fedora:

```sh
sudo dnf install cargo gtk4-devel libadwaita-devel gettext
```

On the first build, Cargo downloads dependencies that are not already in its cache. Run the app with:

```sh
cargo run --locked
```

Run the project checks with:

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

## Links

- [Project website](https://diegopn.github.io/bellbird-site/)
- [Source repository](https://github.com/diegopn/bellbird)
- [Contributing guide](CONTRIBUTING.md)
- [Report an issue](https://github.com/diegopn/bellbird/issues)
- [Creator's website](https://diegopn.github.io/)
