use crate::document::{Block, Document, Inline, ResourceReference};
use crate::error::BellbirdError;
use pulldown_cmark::Parser;

use super::markdown::{markdown_options, parse_blocks};
use super::plain_text::{extract_explicit_metadata, parse_plain_text, plain_block_text};

pub(super) fn convert_structured_document(
    display_name: String,
    extension: &str,
    bytes: &[u8],
) -> Result<Document, BellbirdError> {
    let format = docling::InputFormat::from_extension(extension)
        .filter(|format| format.is_compiled_in())
        .ok_or_else(|| BellbirdError::UnsupportedFormat(extension.to_owned()))?;
    let source = docling::SourceDocument::from_bytes(display_name, format, bytes.to_vec());
    let converted = docling::DocumentConverter::new()
        .strict(true)
        .fetch_images(false)
        .convert(source)
        .map_err(|error| BellbirdError::Conversion(error.to_string()))?;

    Ok(convert_docling_document(&converted.document))
}

#[derive(Clone)]
struct DoclingListItem {
    ordered: bool,
    number: u64,
    first_in_list: bool,
    level: u8,
    marker: Option<String>,
    content: Vec<Inline>,
}

fn convert_docling_document(document: &docling::DoclingDocument) -> Document {
    let mut converted = Document::default();
    converted.blocks = convert_docling_nodes(
        &document.nodes,
        &mut converted.resources,
        &mut converted.has_simplified_content,
    );
    extract_explicit_metadata(&mut converted);
    converted
}

fn convert_docling_nodes(
    nodes: &[docling::Node],
    resources: &mut Vec<ResourceReference>,
    simplified: &mut bool,
) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut list_items = Vec::new();
    for node in nodes {
        if let docling::Node::ListItem {
            ordered,
            number,
            first_in_list,
            text,
            level,
            marker,
            dclx,
            href,
            layer,
            ..
        } = node
        {
            if layer.is_some() {
                *simplified = true;
            }
            let (item_ordered, source_marker, mut content) = match dclx.as_ref() {
                Some(override_content) => {
                    let content = if override_content.runs.is_empty() {
                        parse_markdown_inlines(&override_content.text, resources)
                    } else {
                        convert_docling_runs!(&override_content.runs, simplified)
                    };
                    (
                        override_content.ordered,
                        override_content.marker.as_deref(),
                        content,
                    )
                }
                None => (
                    *ordered,
                    marker.as_deref(),
                    parse_markdown_inlines(text, resources),
                ),
            };
            if let Some(href) = href.as_deref().filter(|href| !href.trim().is_empty()) {
                content = vec![Inline::Link {
                    target: href.to_owned(),
                    title: None,
                    content,
                }];
            }
            let mut item_ordered = item_ordered;
            let mut item_marker = None;
            if let Some(marker) = source_marker.filter(|marker| !marker.is_empty()) {
                if marker.chars().all(|character| {
                    character.is_ascii_alphanumeric() || "().-".contains(character)
                }) {
                    if item_ordered {
                        item_marker = Some(marker.to_owned());
                    } else {
                        *simplified = true;
                        content.insert(0, Inline::Text(format!("{marker} ")));
                    }
                } else {
                    *simplified = true;
                    content.insert(0, Inline::Text(format!("{marker} ")));
                    item_ordered = false;
                }
            }
            list_items.push(DoclingListItem {
                ordered: item_ordered,
                number: *number,
                first_in_list: *first_in_list,
                level: *level,
                marker: item_marker,
                content,
            });
            continue;
        }

        if !list_items.is_empty() {
            let mut cursor = 0;
            blocks.extend(convert_docling_list_items(
                &list_items,
                &mut cursor,
                0,
                simplified,
            ));
            list_items.clear();
        }
        blocks.extend(convert_docling_node(node, resources, simplified));
    }
    if !list_items.is_empty() {
        let mut cursor = 0;
        blocks.extend(convert_docling_list_items(
            &list_items,
            &mut cursor,
            0,
            simplified,
        ));
    }
    blocks
}

fn convert_docling_list_items(
    items: &[DoclingListItem],
    cursor: &mut usize,
    level: u8,
    simplified: &mut bool,
) -> Vec<Block> {
    let mut blocks = Vec::new();
    while let Some(item) = items.get(*cursor) {
        let item_level = item.level;
        if item_level < level {
            break;
        }
        if item_level > level {
            *simplified = true;
            blocks.extend(convert_docling_list_items(
                items, cursor, item_level, simplified,
            ));
            continue;
        }

        let ordered = item.ordered;
        let start_number = item.number;
        let mut list = Vec::new();
        let mut item_labels = Vec::new();
        while let Some(item) = items.get(*cursor) {
            if item.level != level
                || item.ordered != ordered
                || (!list.is_empty() && item.first_in_list)
            {
                break;
            }
            let mut content = vec![Block::Paragraph(item.content.clone())];
            item_labels.push(item.marker.clone());
            *cursor += 1;
            if items.get(*cursor).is_some_and(|next| next.level > level) {
                content.extend(convert_docling_list_items(
                    items,
                    cursor,
                    level.saturating_add(1),
                    simplified,
                ));
            }
            list.push(content);
        }
        blocks.push(Block::List {
            ordered,
            start: (ordered && start_number != 1).then_some(start_number),
            item_labels,
            items: list,
        });
    }
    blocks
}

pub(super) fn convert_docling_node(
    node: &docling::Node,
    resources: &mut Vec<ResourceReference>,
    simplified: &mut bool,
) -> Vec<Block> {
    use docling::Node;

    let paragraph = |inlines| vec![Block::Paragraph(inlines)];
    match node {
        Node::Heading { level, text } => vec![Block::Heading {
            level: *level,
            content: parse_markdown_inlines(text, resources),
        }],
        Node::Paragraph { text } => paragraph(parse_markdown_inlines(text, resources)),
        Node::CheckboxItem { checked, text } => {
            let mut inlines = vec![Inline::Text(format!(
                "[{}] ",
                if *checked { 'x' } else { ' ' }
            ))];
            inlines.extend(parse_markdown_inlines(text, resources));
            paragraph(inlines)
        }
        Node::Code { language, text, .. } => vec![Block::CodeBlock {
            content: text.clone(),
            language: language.clone(),
        }],
        Node::Table(table) => {
            let mut blocks = Vec::new();
            if let Some(caption) = table.caption.as_deref().filter(|text| !text.is_empty()) {
                blocks.push(Block::Paragraph(vec![Inline::Strong(
                    parse_markdown_inlines(caption, resources),
                )]));
            }
            if let Some((header, rows)) = table.rows.split_first() {
                let headers = header
                    .iter()
                    .enumerate()
                    .map(|(column, cell)| {
                        docling_table_cell_inlines(table, 0, column, cell, resources, simplified)
                    })
                    .collect();
                let rows = rows
                    .iter()
                    .enumerate()
                    .map(|(row_index, row)| {
                        row.iter()
                            .enumerate()
                            .map(|(column, cell)| {
                                docling_table_cell_inlines(
                                    table,
                                    row_index + 1,
                                    column,
                                    cell,
                                    resources,
                                    simplified,
                                )
                            })
                            .collect()
                    })
                    .collect();
                blocks.push(Block::Table { headers, rows });
            }
            blocks
        }
        Node::Picture {
            caption,
            caption_href,
            description,
            ..
        } => {
            if !resources
                .iter()
                .any(|resource| resource.source == "image omitted by the source converter")
            {
                resources.push(ResourceReference {
                    source: "image omitted by the source converter".to_owned(),
                });
            }
            let mut blocks = caption
                .as_deref()
                .filter(|text| !text.trim().is_empty())
                .map(|text| {
                    paragraph(docling_link(
                        parse_markdown_inlines(text, resources),
                        caption_href,
                    ))
                })
                .unwrap_or_default();
            if let Some(description) = description
                .as_ref()
                .filter(|description| !description.text.trim().is_empty())
            {
                blocks.extend(paragraph(parse_markdown_inlines(
                    &description.text,
                    resources,
                )));
            }
            blocks
        }
        Node::Formula { latex, .. } => vec![Block::DisplayMath(latex.clone())],
        Node::Caption { text, href } => {
            paragraph(docling_link(parse_markdown_inlines(text, resources), href))
        }
        Node::LabeledText { text, href, .. } => {
            paragraph(docling_link(parse_markdown_inlines(text, resources), href))
        }
        Node::Chart { table, caption, .. } => {
            *simplified = true;
            let mut blocks =
                convert_docling_node(&Node::Table(table.clone()), resources, simplified);
            if let Some(caption) = caption.as_deref().filter(|text| !text.trim().is_empty()) {
                blocks.insert(
                    0,
                    Block::Paragraph(vec![Inline::Strong(parse_markdown_inlines(
                        caption, resources,
                    ))]),
                );
            }
            blocks
        }
        Node::Group {
            children, layer, ..
        } => {
            if layer.is_some() {
                *simplified = true;
            }
            convert_docling_nodes(children, resources, simplified)
        }
        Node::InlineGroup { runs, md_text, .. } => {
            let inlines = if md_text.contains("](") {
                if runs.iter().any(|run| {
                    run.underline || matches!(format!("{:?}", run.script).as_str(), "Sub" | "Super")
                }) {
                    *simplified = true;
                }
                parse_markdown_inlines(md_text, resources)
            } else if runs.is_empty() {
                parse_markdown_inlines(md_text, resources)
            } else {
                convert_docling_runs!(runs, simplified)
            };
            paragraph(inlines)
        }
        Node::Furniture { inner, .. }
        | Node::Commented { inner, .. }
        | Node::Track { inner, .. }
        | Node::Located { inner, .. }
        | Node::Prov { inner, .. }
        | Node::DoclangOnly(inner) => {
            if matches!(node, Node::Furniture { .. } | Node::Commented { .. }) {
                *simplified = true;
            }
            convert_docling_node(inner, resources, simplified)
        }
        Node::FieldRegion { items } => {
            *simplified = true;
            items
                .iter()
                .flat_map(|item| {
                    let fields = item
                        .marker
                        .iter()
                        .chain(item.key.iter())
                        .chain(item.value.iter())
                        .map(String::as_str)
                        .collect::<Vec<_>>()
                        .join(": ");
                    paragraph(vec![Inline::Text(fields)])
                })
                .collect()
        }
        Node::KeyValueGraph { cells, .. } => {
            *simplified = true;
            cells
                .iter()
                .filter(|cell| !cell.text.trim().is_empty())
                .flat_map(|cell| {
                    paragraph(vec![Inline::Text(format!("{}: {}", cell.label, cell.text))])
                })
                .collect()
        }
        Node::CommentSection { text, .. }
        | Node::PageFurniture { text, .. }
        | Node::FurnitureText { text, .. } => {
            *simplified = true;
            paragraph(parse_markdown_inlines(text, resources))
        }
        Node::PictureChildren(children) => {
            *simplified = true;
            convert_docling_nodes(children, resources, simplified)
        }
        Node::PageBreak => {
            *simplified = true;
            vec![Block::HorizontalRule]
        }
        Node::PageInfo { .. } => Vec::new(),
        Node::TextDump(text) => {
            *simplified = true;
            parse_plain_text(text).blocks
        }
        Node::ListItem { .. } => unreachable!("list items are grouped before node conversion"),
    }
}

fn docling_table_cell_inlines(
    table: &docling::Table,
    row: usize,
    column: usize,
    flat_text: &str,
    resources: &mut Vec<ResourceReference>,
    simplified: &mut bool,
) -> Vec<Inline> {
    let Some(cell_nodes) = table
        .cell_blocks
        .as_ref()
        .and_then(|rows| rows.get(row))
        .and_then(|cells| cells.get(column))
        .filter(|nodes| !nodes.is_empty())
    else {
        return parse_markdown_inlines(flat_text, resources);
    };

    let blocks = convert_docling_nodes(cell_nodes, resources, simplified);
    let mut inlines = Vec::new();
    for block in blocks {
        let block_inlines = match &block {
            Block::Heading { content, .. } => {
                *simplified = true;
                vec![Inline::Strong(content.clone())]
            }
            Block::Paragraph(content) => content.clone(),
            Block::CodeBlock { content, .. } => {
                *simplified = true;
                vec![Inline::Code(content.clone())]
            }
            Block::DisplayMath(formula) => {
                *simplified = true;
                vec![Inline::Math(formula.clone())]
            }
            Block::HorizontalRule => {
                *simplified = true;
                vec![Inline::Text("—".to_owned())]
            }
            Block::List { .. }
            | Block::Quote(_)
            | Block::Table { .. }
            | Block::FootnoteDefinition { .. } => {
                *simplified = true;
                plain_block_text_inlines(crate::document::plain_block_text(&block))
            }
        };
        if !inlines.is_empty() && !block_inlines.is_empty() {
            inlines.push(Inline::LineBreak);
        }
        inlines.extend(block_inlines);
    }

    if inlines.is_empty() {
        parse_markdown_inlines(flat_text, resources)
    } else {
        inlines
    }
}

fn plain_block_text_inlines(text: String) -> Vec<Inline> {
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

fn docling_link(content: Vec<Inline>, href: &Option<String>) -> Vec<Inline> {
    match href.as_deref().filter(|href| !href.trim().is_empty()) {
        Some(target) => vec![Inline::Link {
            target: target.to_owned(),
            title: None,
            content,
        }],
        None => content,
    }
}

fn parse_markdown_inlines(input: &str, resources: &mut Vec<ResourceReference>) -> Vec<Inline> {
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
