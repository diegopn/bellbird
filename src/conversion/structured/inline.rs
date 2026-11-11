use crate::document::{Block, Inline, ResourceReference, plain_block_text};

use super::super::markdown::{markdown_options, parse_blocks};
use pulldown_cmark::Parser;

pub(super) fn plain_block_text_inlines(text: String) -> Vec<Inline> {
    let mut inlines = Vec::new();
    for (index, line) in text.split('\n').enumerate() {
        if index > 0 {
            inlines.push(Inline::LineBreak);
        }
        if !line.is_empty() {
            inlines.push(Inline::Text(line.trim_end_matches('\r').to_owned()));
        }
    }
    inlines
}

pub(super) fn docling_link(content: Vec<Inline>, href: &Option<String>) -> Vec<Inline> {
    match href.as_deref().filter(|href| !href.trim().is_empty()) {
        Some(target) => vec![Inline::Link {
            target: target.to_owned(),
            title: None,
            content,
        }],
        None => content,
    }
}

pub(super) fn parse_markdown_inlines(
    input: &str,
    resources: &mut Vec<ResourceReference>,
) -> Vec<Inline> {
    let mut events = Parser::new_ext(input, markdown_options()).peekable();
    let blocks = parse_blocks(&mut events, None, resources);
    let mut inlines = Vec::new();
    for block in blocks {
        match block {
            Block::Heading { content, .. } | Block::Paragraph(content) => inlines.extend(content),
            Block::CodeBlock { content, .. } => inlines.push(Inline::Code(content)),
            Block::DisplayMath(formula) => inlines.push(Inline::Math(formula)),
            Block::List { items, .. } => {
                for item in items {
                    for block in item {
                        if !inlines.is_empty() {
                            inlines.push(Inline::LineBreak);
                        }
                        inlines
                            .extend(parse_markdown_inlines(&plain_block_text(&block), resources));
                    }
                }
            }
            Block::Quote(children) => {
                for child in children {
                    inlines.extend(parse_markdown_inlines(&plain_block_text(&child), resources));
                }
            }
            Block::Table { headers, rows } => {
                for cell in headers.into_iter().chain(rows.into_iter().flatten()) {
                    if !inlines.is_empty() {
                        inlines.push(Inline::Text(" | ".to_owned()));
                    }
                    inlines.extend(cell);
                }
            }
            Block::FootnoteDefinition { .. } | Block::HorizontalRule => {}
        }
    }
    inlines
}
