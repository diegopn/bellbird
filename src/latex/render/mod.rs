use std::collections::BTreeSet;

use crate::document::{Block, Document, Inline, plain_block_text};
use crate::latex::{LatexTemplate, escape_latex, validate_generated_latex};

mod blocks;
mod inline;
mod references;

use blocks::render_blocks;
use inline::collect_footnote_numbers;
use references::{collect_citation_keys, is_external_url, render_references};

const SAFE_FALLBACK_NOTICE: &str = "% Bellbird: conteúdo irregular foi convertido para texto simples para preservar um LaTeX válido.";

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

fn replace_math_with_text(blocks: &mut [Block]) {
    for block in blocks {
        match block {
            Block::Heading { content, .. } | Block::Paragraph(content) => {
                replace_inline_math_with_text(content)
            }
            Block::List { items, .. } => {
                for item in items {
                    replace_math_with_text(item);
                }
            }
            Block::Quote(children)
            | Block::FootnoteDefinition {
                content: children, ..
            } => replace_math_with_text(children),
            Block::Table { headers, rows } => {
                replace_row_math_with_text(headers);
                for row in rows {
                    replace_row_math_with_text(row);
                }
            }
            Block::DisplayMath(_) => {
                if let Block::DisplayMath(formula) = std::mem::replace(block, Block::HorizontalRule)
                {
                    *block = Block::Paragraph(vec![Inline::Code(formula)]);
                }
            }
            Block::CodeBlock { .. } | Block::HorizontalRule => {}
        }
    }
}

fn replace_row_math_with_text(row: &mut [Vec<Inline>]) {
    for cell in row {
        replace_inline_math_with_text(cell);
    }
}

fn replace_inline_math_with_text(inlines: &mut [Inline]) {
    for inline in inlines {
        match inline {
            Inline::Math(_) => {
                if let Inline::Math(formula) =
                    std::mem::replace(inline, Inline::Text(String::new()))
                {
                    *inline = Inline::Code(formula);
                }
            }
            Inline::Emphasis(children)
            | Inline::Strong(children)
            | Inline::Strikethrough(children)
            | Inline::Underline(children)
            | Inline::Subscript(children)
            | Inline::Superscript(children) => replace_inline_math_with_text(children),
            Inline::Link { content, .. } => replace_inline_math_with_text(content),
            Inline::Image { alt, .. } => replace_inline_math_with_text(alt),
            Inline::Text(_)
            | Inline::Code(_)
            | Inline::Citation(_)
            | Inline::FootnoteReference(_)
            | Inline::LineBreak => {}
        }
    }
}

fn render_minimal_safe_latex(document: &Document) -> String {
    let title = document
        .metadata
        .title
        .as_deref()
        .filter(|title| !title.trim().is_empty())
        .map(escape_latex)
        .unwrap_or_else(|| escape_latex("[INSERIR TÍTULO]"));
    let authors = if document.metadata.authors.is_empty() {
        "[INSERIR NOME DO AUTOR]".to_owned()
    } else {
        document
            .metadata
            .authors
            .iter()
            .map(|author| escape_latex(author))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let mut output = format!(
        "\\documentclass[12pt]{{article}}\n\\usepackage[utf8]{{inputenc}}\n\\usepackage[T1]{{fontenc}}\n\\title{{{title}}}\n\\author{{{authors}}}\n\\date{{}}\n\\begin{{document}}\n\\maketitle\n"
    );
    if let Some(abstract_text) = &document.metadata.abstract_text {
        output.push_str("\\begin{abstract}\n");
        output.push_str(&escape_latex(abstract_text));
        output.push_str("\n\\end{abstract}\n\n");
    }
    if !document.metadata.keywords.is_empty() {
        output.push_str("\\noindent\\textbf{Keywords:} ");
        output.push_str(&escape_latex(&document.metadata.keywords.join(", ")));
        output.push_str("\\par\n\n");
    }
    for block in &document.blocks {
        let text = plain_block_text(block);
        if !text.trim().is_empty() {
            output.push_str(&escape_latex(text.trim()));
            output.push_str("\\par\n\n");
        }
    }
    for reference in &document.references {
        let mut details = Vec::new();
        if !reference.authors.is_empty() {
            details.push(reference.authors.join(", "));
        }
        if !reference.title.trim().is_empty() {
            details.push(reference.title.clone());
        }
        if let Some(year) = &reference.year {
            details.push(year.clone());
        }
        if let Some(url) = &reference.url {
            details.push(url.clone());
        }
        if !details.is_empty() {
            output.push_str(&escape_latex(&details.join(". ")));
            output.push_str("\\par\n\n");
        }
    }
    output.push_str("\\end{document}\n");
    output
}

fn template_source(template: LatexTemplate) -> &'static str {
    match template {
        LatexTemplate::Standard => include_str!("../templates/standard.tex"),
        LatexTemplate::Abnt => include_str!("../templates/abnt.tex"),
        LatexTemplate::Ieee => include_str!("../templates/ieee.tex"),
        LatexTemplate::Acm => include_str!("../templates/acm.tex"),
        LatexTemplate::Elsevier => include_str!("../templates/elsevier.tex"),
        LatexTemplate::SpringerNature => include_str!("../templates/springer-nature.tex"),
    }
}

fn render_title_and_authors(
    title: &str,
    authors: &[&str],
    template: LatexTemplate,
    output: &mut String,
) {
    match template {
        LatexTemplate::Abnt => {
            output.push_str(&format!("\\titulo{{{title}}}\n"));
            output.push_str(&format!(
                "\\autor{{{}}}\n\\data{{\\today}}\n",
                authors
                    .iter()
                    .map(|author| escape_latex(author))
                    .collect::<Vec<_>>()
                    .join(" \\and ")
            ));
        }
        LatexTemplate::SpringerNature => {
            output.push_str(&format!("\\title[{title}]{{{title}}}\n"));
            for author in authors {
                let mut name_parts = author.split_whitespace();
                let given_name = name_parts.next().unwrap_or_default();
                let family_name = name_parts.collect::<Vec<_>>().join(" ");
                let formatted_name = if family_name.is_empty() {
                    format!("\\fnm{{{}}}", escape_latex(given_name))
                } else {
                    format!(
                        "\\fnm{{{}}} \\sur{{{}}}",
                        escape_latex(given_name),
                        escape_latex(&family_name)
                    )
                };
                output.push_str(&format!("\\author[]{{{formatted_name}}}\n"));
            }
        }
        LatexTemplate::Acm | LatexTemplate::Elsevier => {
            output.push_str(&format!("\\title{{{title}}}\n"));
            for author in authors {
                output.push_str(&format!("\\author{{{}}}\n", escape_latex(author)));
            }
        }
        LatexTemplate::Ieee => {
            let authors = authors
                .iter()
                .map(|author| escape_latex(author))
                .collect::<Vec<_>>()
                .join(", ");
            output.push_str(&format!("\\title{{{title}}}\n"));
            output.push_str(&format!("\\author{{\\IEEEauthorblockN{{{authors}}}}}\n"));
        }
        _ => {
            output.push_str(&format!("\\title{{{title}}}\n\\date{{}}\n"));
            let authors = authors
                .iter()
                .map(|author| escape_latex(author))
                .collect::<Vec<_>>()
                .join(" \\and ");
            output.push_str(&format!("\\author{{{authors}}}\n"));
        }
    }
}

fn render_abstract(document: &Document, template: LatexTemplate, output: &mut String) {
    let Some(abstract_text) = &document.metadata.abstract_text else {
        return;
    };
    if template == LatexTemplate::SpringerNature {
        output.push_str(&format!("\\abstract{{{}}}\n", escape_latex(abstract_text)));
        return;
    }
    let environment = if template == LatexTemplate::Abnt {
        "resumo"
    } else {
        "abstract"
    };
    output.push_str(&format!("\\begin{{{environment}}}\n"));
    output.push_str(&escape_latex(abstract_text));
    output.push_str(&format!("\n\\end{{{environment}}}\n"));
}

fn render_keywords(document: &Document, template: LatexTemplate, output: &mut String) {
    if document.metadata.keywords.is_empty() {
        return;
    }
    let keywords = escape_latex(&document.metadata.keywords.join(", "));
    match template {
        LatexTemplate::Abnt => output.push_str(&format!(
            "\\noindent\\textbf{{Palavras-chave:}} {keywords}\n\n"
        )),
        LatexTemplate::Ieee => output.push_str(&format!(
            "\\begin{{IEEEkeywords}}\n{keywords}\n\\end{{IEEEkeywords}}\n"
        )),
        LatexTemplate::Acm => output.push_str(&format!("\\keywords{{{keywords}}}\n")),
        LatexTemplate::Elsevier => {
            let keywords = document
                .metadata
                .keywords
                .iter()
                .map(|keyword| escape_latex(keyword))
                .collect::<Vec<_>>()
                .join(" \\sep ");
            output.push_str(&format!(
                "\\begin{{keyword}}\n{keywords}\n\\end{{keyword}}\n"
            ));
        }
        LatexTemplate::Standard => {
            output.push_str(&format!("\\noindent\\textbf{{Keywords:}} {keywords}\n\n"))
        }
        LatexTemplate::SpringerNature => output.push_str(&format!("\\keywords{{{keywords}}}\n")),
    }
}

fn inspect_block(block: &Block, packages: &mut BTreeSet<&'static str>) {
    match block {
        Block::Heading { content, .. } | Block::Paragraph(content) => {
            inspect_inlines(content, packages)
        }
        Block::List { items, .. } => {
            for item in items {
                for nested in item {
                    inspect_block(nested, packages);
                }
            }
        }
        Block::Quote(items) => {
            for nested in items {
                inspect_block(nested, packages);
            }
        }
        Block::Table { headers, rows } => {
            inspect_row(headers, packages);
            for row in rows {
                inspect_row(row, packages);
            }
        }
        Block::DisplayMath(_) => {
            packages.insert("amsmath");
        }
        Block::FootnoteDefinition { content, .. } => {
            for nested in content {
                inspect_block(nested, packages);
            }
        }
        Block::CodeBlock { .. } | Block::HorizontalRule => {}
    }
}

fn inspect_row(row: &[Vec<Inline>], packages: &mut BTreeSet<&'static str>) {
    for cell in row {
        inspect_inlines(cell, packages);
    }
}

fn inspect_inlines(inlines: &[Inline], packages: &mut BTreeSet<&'static str>) {
    for inline in inlines {
        match inline {
            Inline::Emphasis(children)
            | Inline::Strong(children)
            | Inline::Underline(children)
            | Inline::Subscript(children)
            | Inline::Superscript(children) => inspect_inlines(children, packages),
            Inline::Strikethrough(children) => {
                packages.insert("ulem");
                inspect_inlines(children, packages);
            }
            Inline::Link {
                target, content, ..
            } => {
                if is_external_url(target) {
                    packages.insert("hyperref");
                }
                inspect_inlines(content, packages);
            }
            Inline::Image { alt, .. } => {
                inspect_inlines(alt, packages);
            }
            Inline::Math(_) => {
                packages.insert("amsmath");
            }
            Inline::Citation(_)
            | Inline::FootnoteReference(_)
            | Inline::Text(_)
            | Inline::Code(_)
            | Inline::LineBreak => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Metadata;

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
