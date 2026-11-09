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

## Build from source

See [BUILDING.md](BUILDING.md) for Linux dependencies and development commands on Fedora, Debian/Ubuntu, Arch, and openSUSE.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for ways to report problems, test the app, improve translations, or contribute code and documentation.

## Support

Sponsorship supports Bellbird development, maintenance, documentation, and releases.

<p align="center">
  <a href="https://github.com/sponsors/diegopn"><img alt="Become a sponsor" src="https://img.shields.io/badge/Become_a_sponsor-EA4AAA?style=for-the-badge&logo=githubsponsors&logoColor=white"></a>
</p>

## License

GPL-3.0-or-later. See [LICENSE](LICENSE).
