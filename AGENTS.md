# Agent guidance

## Project

Bellbird is an offline GNOME desktop app that converts documents into structured LaTeX. See `README.md` for supported formats, templates, platform requirements, and product limitations. See `CONTRIBUTING.md` for contribution guidance.

## Code map

- `src/ui.rs`: GTK/libadwaita interface and import/export flow.
- `src/conversion.rs`: file import, parsing, and conversion into the document model.
- `src/document.rs`: document model, LaTeX rendering, and generated-output validation.
- `src/templates/`: LaTeX templates for supported publication formats.
- `po/`: interface translations.

## Guidance

- Keep document processing local. Do not add network or account requirements for conversion.
- Preserve the documented export scope: one `.tex` file; Bellbird does not compile PDFs or export separate image or bibliography files.
- Keep changes focused and follow existing Rust patterns. For conversion or rendering changes, add or update relevant tests when practical.
- Preserve input validation and safety checks, especially when handling imported archives and generated LaTeX.
- User-facing source strings are written in English; update the translation catalogs in `po/` when changing them.
- Consult `README.md` for setup and run the project checks listed there before reporting a change as verified.
