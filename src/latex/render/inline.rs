use std::collections::BTreeMap;

use crate::document::{Block, Inline};
use crate::latex::escape_latex;

use super::references::{is_external_url, latex_reference_key};

pub(super) fn collect_footnote_numbers(blocks: &[Block]) -> FootnoteNumbers {
    let mut numbers = FootnoteNumbers::default();
    collect_footnote_numbers_from_blocks(blocks, &mut numbers);
    numbers
}

pub(super) type FootnoteNumbers = BTreeMap<String, usize>;

fn collect_footnote_numbers_from_blocks(blocks: &[Block], numbers: &mut FootnoteNumbers) {
    for block in blocks {
        match block {
            Block::Heading { content, .. } | Block::Paragraph(content) => {
                collect_footnote_numbers_from_inlines(content, numbers);
            }
            Block::List { items, .. } => {
                for item in items {
                    collect_footnote_numbers_from_blocks(item, numbers);
                }
            }
            Block::Quote(children) => collect_footnote_numbers_from_blocks(children, numbers),
            Block::Table { headers, rows } => {
                collect_footnote_numbers_from_row(headers, numbers);
                for row in rows {
                    collect_footnote_numbers_from_row(row, numbers);
                }
            }
            Block::FootnoteDefinition { content, .. } => {
                collect_footnote_numbers_from_blocks(content, numbers);
            }
            Block::CodeBlock { .. } | Block::DisplayMath(_) | Block::HorizontalRule => {}
        }
    }
}

fn collect_footnote_numbers_from_row(row: &[Vec<Inline>], numbers: &mut FootnoteNumbers) {
    for cell in row {
        collect_footnote_numbers_from_inlines(cell, numbers);
    }
}

fn collect_footnote_numbers_from_inlines(inlines: &[Inline], numbers: &mut FootnoteNumbers) {
    for inline in inlines {
        match inline {
            Inline::FootnoteReference(label) => {
                let next = numbers.len() + 1;
                numbers.entry(label.clone()).or_insert(next);
            }
            Inline::Emphasis(children)
            | Inline::Strong(children)
            | Inline::Strikethrough(children)
            | Inline::Underline(children)
            | Inline::Subscript(children)
            | Inline::Superscript(children) => {
                collect_footnote_numbers_from_inlines(children, numbers);
            }
            Inline::Link { content, .. } => {
                collect_footnote_numbers_from_inlines(content, numbers);
            }
            Inline::Image { alt, .. } => collect_footnote_numbers_from_inlines(alt, numbers),
            Inline::Text(_)
            | Inline::Code(_)
            | Inline::Math(_)
            | Inline::Citation(_)
            | Inline::LineBreak => {}
        }
    }
}

pub(super) fn render_inlines(inlines: &[Inline], footnote_numbers: &FootnoteNumbers) -> String {
    render_inlines_with_context(inlines, footnote_numbers, false)
}

pub(super) fn render_inlines_with_context(
    inlines: &[Inline],
    footnote_numbers: &FootnoteNumbers,
    in_table_cell: bool,
) -> String {
    let mut output = String::new();
    for inline in inlines {
        match inline {
            Inline::Text(text) => output.push_str(&escape_latex(text)),
            Inline::Emphasis(children) => output.push_str(&format!(
                "\\emph{{{}}}",
                render_inlines_with_context(children, footnote_numbers, in_table_cell)
            )),
            Inline::Strong(children) => output.push_str(&format!(
                "\\textbf{{{}}}",
                render_inlines_with_context(children, footnote_numbers, in_table_cell)
            )),
            Inline::Strikethrough(children) => output.push_str(&format!(
                "\\sout{{{}}}",
                render_inlines_with_context(children, footnote_numbers, in_table_cell)
            )),
            Inline::Underline(children) => output.push_str(&format!(
                "\\underline{{{}}}",
                render_inlines_with_context(children, footnote_numbers, in_table_cell)
            )),
            Inline::Subscript(children) => output.push_str(&format!(
                "\\textsubscript{{{}}}",
                render_inlines_with_context(children, footnote_numbers, in_table_cell)
            )),
            Inline::Superscript(children) => output.push_str(&format!(
                "\\textsuperscript{{{}}}",
                render_inlines_with_context(children, footnote_numbers, in_table_cell)
            )),
            Inline::Code(text) => output.push_str(&format!("\\texttt{{{}}}", escape_latex(text))),
            Inline::Link {
                target, content, ..
            } => {
                let content = render_inlines_with_context(content, footnote_numbers, in_table_cell);
                if is_external_url(target) {
                    output.push_str(&format!("\\href{{{}}}{{{content}}}", escape_latex(target)));
                } else {
                    output.push_str(&content);
                }
            }
            Inline::Image { alt, .. } => output.push_str(&render_inlines_with_context(
                alt,
                footnote_numbers,
                in_table_cell,
            )),
            Inline::Math(formula) => output.push_str(&format!("\\({formula}\\)")),
            Inline::Citation(keys) => {
                let keys = keys
                    .iter()
                    .filter(|key| !key.trim().is_empty())
                    .map(|key| latex_reference_key(key))
                    .collect::<Vec<_>>();
                if keys.is_empty() {
                    output.push_str("[citação sem identificador]");
                } else {
                    output.push_str(&format!("\\cite{{{}}}", keys.join(",")));
                }
            }
            Inline::FootnoteReference(label) => {
                if let Some(number) = footnote_numbers.get(label) {
                    output.push_str(&format!("\\footnotemark[{number}]"));
                } else {
                    output.push_str("\\footnotemark{}");
                }
            }
            Inline::LineBreak if in_table_cell => output.push_str("\\newline{}"),
            Inline::LineBreak => output.push_str("\\\\\n"),
        }
    }
    output
}
