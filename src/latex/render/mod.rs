use std::collections::BTreeSet;

use crate::document::Document;
use crate::latex::{LatexTemplate, escape_latex, validate_generated_latex};

mod blocks;
mod fallback;
mod inline;
mod metadata;
mod packages;
mod references;

use blocks::render_blocks;
use fallback::{SAFE_FALLBACK_NOTICE, render_minimal_safe_latex, replace_math_with_text};
use inline::collect_footnote_numbers;
use metadata::{render_abstract, render_keywords, render_title_and_authors, template_source};
use packages::inspect_block;
use references::{collect_citation_keys, is_external_url, render_references};

pub fn render_latex(document: &Document, template: LatexTemplate) -> String {
    let output = render_latex_template(document, template);
    if validate_generated_latex(&output).is_ok() {
        return output;
    }

    let mut safe_document = document.clone();
    replace_math_with_text(&mut safe_document.blocks);
    let output = render_latex_template(&safe_document, template);
    if validate_generated_latex(&output).is_ok() {
        return output;
    }

    format!(
        "{SAFE_FALLBACK_NOTICE}\n{}",
        render_minimal_safe_latex(&safe_document)
    )
}

pub fn used_safe_fallback(latex: &str) -> bool {
    latex.starts_with(SAFE_FALLBACK_NOTICE)
}

fn render_latex_template(document: &Document, template: LatexTemplate) -> String {
    let mut packages = BTreeSet::new();
    let footnote_numbers = collect_footnote_numbers(&document.blocks);
    let explicit_title = document
        .metadata
        .title
        .as_deref()
        .filter(|title| !title.trim().is_empty());
    let title = explicit_title
        .map(escape_latex)
        .unwrap_or_else(|| escape_latex("[INSERIR TÍTULO]"));
    let blocks = &document.blocks;
    for block in blocks {
        inspect_block(block, &mut packages);
    }
    if document
        .references
        .iter()
        .any(|reference| reference.url.as_deref().is_some_and(is_external_url))
    {
        packages.insert("hyperref");
    }

    let mut package_output = String::new();
    if template == LatexTemplate::SpringerNature && packages.contains("amsmath") {
        package_output.push_str("\\usepackage{amsmath,amssymb,amsfonts,amsthm,mathrsfs}\n");
    }
    for package in packages.iter().filter(|package| {
        **package != "hyperref"
            && !(template == LatexTemplate::SpringerNature && **package == "amsmath")
    }) {
        if *package == "ulem" {
            package_output.push_str("\\usepackage[normalem]{ulem}\n");
        } else {
            package_output.push_str(&format!("\\usepackage{{{package}}}\n"));
        }
    }
    if packages.contains("hyperref") && template != LatexTemplate::Acm {
        package_output.push_str("\\usepackage{hyperref}\n");
    }

    let authors = document
        .metadata
        .authors
        .iter()
        .map(String::as_str)
        .filter(|author| !author.trim().is_empty())
        .collect::<Vec<_>>();
    let authors = if authors.is_empty() {
        vec!["[INSERIR NOME DO AUTOR]"]
    } else {
        authors
    };
    let mut title_author = String::new();
    render_title_and_authors(&title, &authors, template, &mut title_author);

    let mut abstract_output = String::new();
    render_abstract(document, template, &mut abstract_output);
    let mut keywords = String::new();
    render_keywords(document, template, &mut keywords);
    let mut body = String::new();
    render_blocks(blocks, &mut body, template, &footnote_numbers);
    let mut references = String::new();
    render_references(
        &document.references,
        &collect_citation_keys(&document.blocks),
        &mut references,
    );

    template_source(template)
        .replace("{{PACKAGES}}", &package_output)
        .replace("{{TITLE_AUTHOR}}", &title_author)
        .replace("{{ABSTRACT}}", &abstract_output)
        .replace("{{KEYWORDS}}", &keywords)
        .replace("{{BODY}}", &body)
        .replace("{{REFERENCES}}", &references)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{Block, Inline, Metadata};

    #[test]
    fn validator_accepts_nested_inline_formatting() {
        let document = Document {
            blocks: vec![Block::Paragraph(vec![Inline::Strong(vec![
                Inline::Text("forte e ".into()),
                Inline::Emphasis(vec![Inline::Text("ênfase".into())]),
            ])])],
            ..Document::default()
        };
        let latex = render_latex_template(&document, LatexTemplate::Standard);
        let validation = validate_generated_latex(&latex);

        assert!(validation.is_ok(), "{validation:?}\n{latex}");
    }

    #[test]
    fn validator_accepts_ieee_metadata_and_environments() {
        let document = Document {
            metadata: Metadata {
                title: Some("Sample title".into()),
                authors: vec!["First Author".into(), "Second Author".into()],
                abstract_text: Some("A short abstract.".into()),
                keywords: vec!["first".into(), "second".into()],
                ..Metadata::default()
            },
            blocks: vec![Block::Paragraph(vec![Inline::Text("Body".into())])],
            ..Document::default()
        };
        let latex = render_latex_template(&document, LatexTemplate::Ieee);
        let validation = validate_generated_latex(&latex);

        assert!(validation.is_ok(), "{validation:?}\n{latex}");
    }
}
