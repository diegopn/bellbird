pub const MAX_FILE_BYTES: usize = 10 * 1024 * 1024;
pub const SUPPORTED_EXTENSIONS: &[&str] = &[
    "doc", "docx", "epub", "htm", "html", "markdown", "md", "odt", "org", "rst", "tex", "txt",
];

macro_rules! convert_docling_runs {
    ($runs:expr, $simplified:expr) => {{
        let mut converted = Vec::new();
        let mut has_run = false;
        for run in ($runs).iter().filter(|run| !run.text.is_empty()) {
            if has_run {
                converted.push(Inline::Text(" ".to_owned()));
            }
            has_run = true;

            let mut inline = if run.formula {
                Inline::Math(run.text.clone())
            } else if run.code {
                Inline::Code(run.text.clone())
            } else {
                Inline::Text(run.text.clone())
            };
            if run.bold {
                inline = Inline::Strong(vec![inline]);
            }
            if run.italic {
                inline = Inline::Emphasis(vec![inline]);
            }
            if run.underline {
                inline = Inline::Underline(vec![inline]);
            }
            if run.strike {
                inline = Inline::Strikethrough(vec![inline]);
            }
            // Docling exposes InlineRun but does not re-export its Script enum.
            match format!("{:?}", run.script).as_str() {
                "Sub" => inline = Inline::Subscript(vec![inline]),
                "Super" => inline = Inline::Superscript(vec![inline]),
                "Baseline" => {}
                _ => *$simplified = true,
            }
            converted.push(inline);
        }
        converted
    }};
}

mod archive;
mod file;
mod markdown;
mod org;
mod plain_text;
mod restructured_text;
mod structured;

pub use file::{ImportedFile, ImportedSource, import_bytes, import_file, path_is_supported};
pub use markdown::parse_markdown;
pub use plain_text::parse_plain_text;

#[cfg(test)]
mod tests;
