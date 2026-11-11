use std::collections::BTreeSet;

use crate::document::{Block, Inline};

use super::references::is_external_url;

pub(super) fn inspect_block(block: &Block, packages: &mut BTreeSet<&'static str>) {
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
