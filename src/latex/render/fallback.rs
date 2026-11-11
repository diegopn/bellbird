use crate::document::{Block, Document, Inline, plain_block_text};
use crate::latex::escape_latex;

pub(super) const SAFE_FALLBACK_NOTICE: &str = "% Bellbird: conteúdo irregular foi convertido para texto simples para preservar um LaTeX válido.";

pub(super) fn replace_math_with_text(blocks: &mut [Block]) {
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

pub(super) fn render_minimal_safe_latex(document: &Document) -> String {
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
