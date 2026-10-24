# Contributing

All kinds of help are welcome. You can report a problem, test the app, improve a translation, or contribute code and documentation. You do not need Rust experience to help.

## Ways to help

- **Report problems:** [open an issue](https://github.com/diegopn/bellbird/issues) with the Bellbird version, your Linux distribution, the input file format, what you expected, and what happened. Include steps to reproduce the issue.
- **Help test:** try importing supported file formats and exporting each LaTeX template. Let us know which combinations you tried and what happened.
- **Improve translations:** help translate the interface in `po/pt_BR.po` or `po/es.po`. The original interface messages are written in English in the source code.
- **Improve the code:** small, focused changes are easier to review. When changing conversion behavior, include tests for the changed behavior when practical.
- **Improve the documentation:** clarify features, supported formats, limitations, and examples.

When reporting or testing document issues, use a small sample created for that purpose. Do not include personal documents, private data, or credentials in issues or pull requests.

## Set up the development environment

The [README](README.md#build-on-fedora) lists the dependencies and instructions for running Bellbird. After installing them, run the same checks used by continuous integration:

```sh
cargo fmt --all -- --check
msgfmt --check po/pt_BR.po -o /tmp/bellbird-pt_BR.mo
msgfmt --check po/es.po -o /tmp/bellbird-es.mo
cargo clippy --all-targets -- -D warnings
cargo test
```

## Pull requests

- Explain the problem or improvement and how reviewers can reproduce or review the result.
- List the checks you ran.
- Keep each pull request focused on one change.
- When adding third-party files or excerpts, check that their license is compatible and preserve copyright and license notices. Bellbird is licensed under GPL-3.0-or-later; see [`LICENSE`](LICENSE).
