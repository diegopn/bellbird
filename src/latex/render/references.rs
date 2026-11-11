use std::collections::{BTreeMap, BTreeSet};

use crate::document::{Block, Inline, Reference};
use crate::latex::escape_latex;

pub(super) fn collect_citation_keys(blocks: &[Block]) -> BTreeMap<String, String> {
    let mut keys = BTreeMap::new();
    collect_citation_keys_from_blocks(blocks, &mut keys);
    keys
}

fn collect_citation_keys_from_blocks(blocks: &[Block], keys: &mut BTreeMap<String, String>) {
    for block in blocks {
        match block {
            Block::Heading { content, .. } | Block::Paragraph(content) => {
                collect_citation_keys_from_inlines(content, keys)
            }
            Block::List { items, .. } => {
                for item in items {
                    collect_citation_keys_from_blocks(item, keys);
                }
            }
            Block::Quote(children)
            | Block::FootnoteDefinition {
                content: children, ..
            } => collect_citation_keys_from_blocks(children, keys),
            Block::Table { headers, rows } => {
                collect_citation_keys_from_row(headers, keys);
                for row in rows {
                    collect_citation_keys_from_row(row, keys);
                }
            }
            Block::CodeBlock { .. } | Block::DisplayMath(_) | Block::HorizontalRule => {}
        }
    }
}

fn collect_citation_keys_from_row(row: &[Vec<Inline>], keys: &mut BTreeMap<String, String>) {
    for cell in row {
        collect_citation_keys_from_inlines(cell, keys);
    }
}

fn collect_citation_keys_from_inlines(inlines: &[Inline], keys: &mut BTreeMap<String, String>) {
    for inline in inlines {
        match inline {
            Inline::Citation(citations) => {
                for citation in citations.iter().filter(|key| !key.trim().is_empty()) {
                    keys.entry(latex_reference_key(citation))
                        .or_insert_with(|| citation.trim().to_owned());
                }
            }
            Inline::Emphasis(children)
            | Inline::Strong(children)
            | Inline::Strikethrough(children)
            | Inline::Underline(children)
            | Inline::Subscript(children)
            | Inline::Superscript(children) => collect_citation_keys_from_inlines(children, keys),
            Inline::Link { content, .. } => collect_citation_keys_from_inlines(content, keys),
            Inline::Image { alt, .. } => collect_citation_keys_from_inlines(alt, keys),
            Inline::Text(_)
            | Inline::Code(_)
            | Inline::Math(_)
            | Inline::FootnoteReference(_)
            | Inline::LineBreak => {}
        }
    }
}

pub(super) fn latex_reference_key(key: &str) -> String {
    let mut result = String::new();
    for character in key.trim().chars() {
        if character.is_ascii_alphanumeric() {
            result.push(character);
        } else {
            result.push_str(&format!("_{:X}_", character as u32));
        }
    }
    if result.is_empty() {
        "reference".to_owned()
    } else {
        result
    }
}

pub(super) fn render_references(
    references: &[Reference],
    citation_keys: &BTreeMap<String, String>,
    output: &mut String,
) {
    if references.is_empty() && citation_keys.is_empty() {
        return;
    }
    output.push_str("\\begin{thebibliography}{99}\n");
    let mut rendered_keys = BTreeSet::new();
    for reference in references {
        let key = latex_reference_key(&reference.key);
        if !rendered_keys.insert(key.clone()) {
            continue;
        }
        output.push_str(&format!("\\bibitem{{{key}}} "));
        if !reference.authors.is_empty() {
            output.push_str(&escape_latex(&reference.authors.join(", ")));
        } else {
            output.push_str("[INSERIR AUTOR DA REFERÊNCIA]");
        }
        output.push_str(". ");
        let title = if reference.title.trim().is_empty() {
            "[INSERIR TÍTULO DA REFERÊNCIA]"
        } else {
            &reference.title
        };
        output.push_str(&format!("\\textit{{{}}}", escape_latex(title)));
        if let Some(year) = &reference.year {
            output.push_str(&format!(", {}", escape_latex(year)));
        } else {
            output.push_str(", [INSERIR ANO]");
        }
        if let Some(url) = reference.url.as_deref().filter(|url| is_external_url(url)) {
            output.push_str(&format!(". \\url{{{}}}", escape_latex(url)));
        }
        output.push_str(".\n");
    }
    for (key, original_key) in citation_keys {
        if rendered_keys.contains(key) {
            continue;
        }
        output.push_str(&format!(
            "\\bibitem{{{key}}} [INSERIR REFERÊNCIA PARA {}].\n",
            escape_latex(original_key)
        ));
    }
    output.push_str("\\end{thebibliography}\n");
}

pub(super) fn is_external_url(target: &str) -> bool {
    let target = target.trim().to_ascii_lowercase();
    ["http://", "https://", "mailto:", "ftp://", "tel:"]
        .iter()
        .any(|scheme| target.starts_with(scheme))
}
