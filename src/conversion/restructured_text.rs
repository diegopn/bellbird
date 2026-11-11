use parserst::{Block as RstBlock, Inline as RstInline, ListKind as RstListKind};

use crate::document::{Block, Document, Inline, ResourceReference};
use crate::error::BellbirdError;

use super::plain_text::extract_explicit_metadata;

pub(super) fn parse_restructured_text(input: &str) -> Result<Document, BellbirdError> {
    let parsed =
        parserst::parse(input).map_err(|error| BellbirdError::Conversion(error.to_string()))?;
    let mut resources = Vec::new();
    let blocks = parsed
        .iter()
        .flat_map(|block| convert_rst_block(block, &mut resources))
        .collect();
    let mut document = Document {
        blocks,
        resources,
        ..Document::default()
    };
    extract_explicit_metadata(&mut document);
    Ok(document)
}

fn convert_rst_block(block: &RstBlock, resources: &mut Vec<ResourceReference>) -> Vec<Block> {
    match block {
        RstBlock::Heading { level, inlines } => vec![Block::Heading {
            level: *level,
            content: convert_rst_inlines(inlines),
        }],
        RstBlock::Paragraph(inlines) => {
            vec![Block::Paragraph(convert_rst_inlines(inlines))]
        }
        RstBlock::List { kind, items } => vec![Block::List {
            ordered: *kind == RstListKind::Ordered,
            start: None,
            item_labels: Vec::new(),
            items: items
                .iter()
                .map(|item| vec![Block::Paragraph(convert_rst_inlines(item))])
                .collect(),
        }],
        RstBlock::CodeBlock(content) | RstBlock::LiteralBlock(content) => {
            vec![Block::CodeBlock {
                content: content.clone(),
                language: None,
            }]
        }
        RstBlock::Quote(children) => vec![Block::Quote(
            children
                .iter()
                .flat_map(|child| convert_rst_block(child, resources))
                .collect(),
        )],
        RstBlock::Table { headers, rows } => vec![Block::Table {
            headers: headers
                .iter()
                .map(|cell| convert_rst_inlines(cell))
                .collect(),
            rows: rows
                .iter()
                .map(|row| row.iter().map(|cell| convert_rst_inlines(cell)).collect())
                .collect(),
        }],
        RstBlock::Directive { name, argument, .. } if name == "image" => {
            resources.push(ResourceReference {
                source: argument.clone(),
            });
            vec![Block::Paragraph(vec![Inline::Image {
                source: argument.clone(),
                alt: vec![Inline::Text("image".to_owned())],
            }])]
        }
        RstBlock::Directive {
            name,
            argument,
            content,
        } if name == "code-block" || name == "code" => {
            let code = content
                .iter()
                .map(|block| match block {
                    RstBlock::LiteralBlock(text) | RstBlock::CodeBlock(text) => text.as_str(),
                    _ => "",
                })
                .collect::<Vec<_>>()
                .join("\n");
            vec![Block::CodeBlock {
                content: code,
                language: (!argument.is_empty()).then(|| argument.clone()),
            }]
        }
        RstBlock::Directive {
            name,
            argument,
            content,
        } => {
            let mut quote = vec![Block::Paragraph(vec![Inline::Text(
                if argument.is_empty() {
                    format!("{name} directive")
                } else {
                    format!("{name}: {argument}")
                },
            )])];
            quote.extend(
                content
                    .iter()
                    .flat_map(|child| convert_rst_block(child, resources)),
            );
            vec![Block::Quote(quote)]
        }
        RstBlock::Comment(_) => Vec::new(),
        RstBlock::FieldList { fields } => {
            let mut converted = Vec::new();
            for field in fields {
                let label = if field.argument.is_empty() {
                    format!("{}:", field.name)
                } else {
                    format!("{} {}:", field.name, field.argument)
                };
                converted.push(Block::Paragraph(vec![Inline::Strong(vec![Inline::Text(
                    label,
                )])]));
                for child in &field.body {
                    converted.extend(convert_rst_block(child, resources));
                }
            }
            converted
        }
    }
}

fn convert_rst_inlines(inlines: &[RstInline]) -> Vec<Inline> {
    inlines
        .iter()
        .map(|inline| match inline {
            RstInline::Text(text) => Inline::Text(text.clone()),
            RstInline::Em(children) => Inline::Emphasis(convert_rst_inlines(children)),
            RstInline::Strong(children) => Inline::Strong(convert_rst_inlines(children)),
            RstInline::Code(text) => Inline::Code(text.clone()),
            RstInline::Link { text, url } => Inline::Link {
                target: url.clone(),
                title: None,
                content: convert_rst_inlines(text),
            },
        })
        .collect()
}
