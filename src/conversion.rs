use std::fs::File;
use std::io::{Cursor, Read};
use std::iter::Peekable;
use std::path::Path;

use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

use crate::document::{Block, Document, Inline, ResourceReference};
use crate::error::BellbirdError;

use parserst::{Block as RstBlock, Inline as RstInline, ListKind as RstListKind};

pub const MAX_FILE_BYTES: usize = 10 * 1024 * 1024;
const MAX_ARCHIVE_ENTRIES: usize = 10_000;
const MAX_ARCHIVE_ENTRY_BYTES: u64 = 64 * 1024 * 1024;
const MAX_ARCHIVE_TOTAL_BYTES: u64 = 64 * 1024 * 1024;
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportedFile {
    pub display_name: String,
    pub source: ImportedSource,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImportedSource {
    Document(Document),
    ExistingLatex(String),
}

pub fn parse_markdown(input: &str) -> Document {
    let options = markdown_options();
    let mut events = Parser::new_ext(input, options).peekable();
    let mut resources = Vec::new();
    let blocks = parse_blocks(&mut events, None, &mut resources);
    let mut document = Document {
        blocks,
        resources,
        ..Document::default()
    };
    extract_explicit_metadata(&mut document);
    document
}

fn markdown_options() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_MATH
}

pub fn parse_plain_text(input: &str) -> Document {
    let mut blocks = Vec::new();
    let mut paragraph = Vec::new();
    for line in input.lines() {
        if line.trim().is_empty() {
            if !paragraph.is_empty() {
                blocks.push(Block::Paragraph(std::mem::take(&mut paragraph)));
            }
        } else {
            if !paragraph.is_empty() {
                paragraph.push(Inline::LineBreak);
            }
            paragraph.push(Inline::Text(line.to_owned()));
        }
    }
    if !paragraph.is_empty() {
        blocks.push(Block::Paragraph(paragraph));
    }
    let mut document = Document {
        blocks,
        ..Document::default()
    };
    extract_explicit_metadata(&mut document);
    document
}

fn extract_explicit_metadata(document: &mut Document) {
    let leading_index = usize::from(matches!(
        document.blocks.first(),
        Some(Block::Heading { level: 1, .. })
    ));
    while let Some(Block::Paragraph(content)) = document.blocks.get(leading_index) {
        let text = plain_text(content);
        let Some(author_text) = explicit_author_value(&text) else {
            break;
        };
        if document.metadata.authors.is_empty() {
            document.metadata.authors = author_text
                .split(';')
                .map(str::trim)
                .filter(|author| !author.is_empty())
                .map(str::to_owned)
                .collect();
        }
        document.blocks.remove(leading_index);
    }

    let mut remaining = Vec::with_capacity(document.blocks.len());
    let mut index = 0;
    while index < document.blocks.len() {
        let Some(Block::Heading { content, .. }) = document.blocks.get(index) else {
            remaining.push(document.blocks[index].clone());
            index += 1;
            continue;
        };
        let heading = plain_text(content).trim().to_lowercase();
        let (kind, inline_value) = match metadata_heading(&heading) {
            Some(value) => value,
            None => {
                remaining.push(document.blocks[index].clone());
                index += 1;
                continue;
            }
        };
        let metadata_already_exists = match kind {
            MetadataSection::Abstract => document.metadata.abstract_text.is_some(),
            MetadataSection::Keywords => !document.metadata.keywords.is_empty(),
        };
        if metadata_already_exists {
            remaining.push(document.blocks[index].clone());
            index += 1;
            continue;
        }

        let start = index + 1;
        let mut end = start;
        while end < document.blocks.len() && !matches!(document.blocks[end], Block::Heading { .. })
        {
            end += 1;
        }
        let mut section_text = inline_value.to_owned();
        for block in &document.blocks[start..end] {
            let text = plain_block_text(block);
            if !text.trim().is_empty() {
                if !section_text.is_empty() {
                    section_text.push('\n');
                }
                section_text.push_str(text.trim());
            }
        }
        if section_text.trim().is_empty() {
            remaining.push(document.blocks[index].clone());
            index += 1;
            continue;
        }
        match kind {
            MetadataSection::Abstract => document.metadata.abstract_text = Some(section_text),
            MetadataSection::Keywords => {
                document.metadata.keywords = section_text
                    .split([',', ';', '\n'])
                    .map(str::trim)
                    .filter(|keyword| !keyword.is_empty())
                    .map(str::to_owned)
                    .collect()
            }
        }
        index = end;
    }
    document.blocks = remaining;
}

#[derive(Clone, Copy)]
enum MetadataSection {
    Abstract,
    Keywords,
}

fn metadata_heading(heading: &str) -> Option<(MetadataSection, &str)> {
    let heading = heading.trim();
    for label in ["abstract:", "resumo:", "resumen:"] {
        if let Some(value) = heading.strip_prefix(label) {
            return Some((MetadataSection::Abstract, value.trim()));
        }
    }
    if matches!(heading, "abstract" | "resumo" | "resumen") {
        return Some((MetadataSection::Abstract, ""));
    }
    for label in [
        "keywords:",
        "palavras-chave:",
        "palavras chave:",
        "palabras clave:",
    ] {
        if let Some(value) = heading.strip_prefix(label) {
            return Some((MetadataSection::Keywords, value.trim()));
        }
    }
    if matches!(
        heading,
        "keywords" | "palavras-chave" | "palavras chave" | "palabras clave"
    ) {
        return Some((MetadataSection::Keywords, ""));
    }
    None
}

fn explicit_author_value(text: &str) -> Option<&str> {
    let text = text.trim();
    if text.contains('\n') {
        return None;
    }
    let lower = text.to_lowercase();
    ["author:", "authors:", "autor:", "autores:"]
        .iter()
        .find_map(|label| lower.strip_prefix(label).map(|_| &text[label.len()..]))
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn plain_text(inlines: &[Inline]) -> String {
    let mut output = String::new();
    for inline in inlines {
        match inline {
            Inline::Text(text) | Inline::Code(text) | Inline::Math(text) => output.push_str(text),
            Inline::Emphasis(children)
            | Inline::Strong(children)
            | Inline::Strikethrough(children)
            | Inline::Underline(children)
            | Inline::Subscript(children)
            | Inline::Superscript(children) => output.push_str(&plain_text(children)),
            Inline::Link { content, .. } => output.push_str(&plain_text(content)),
            Inline::Image { alt, .. } => output.push_str(&plain_text(alt)),
            Inline::Citation(_) => {}
            Inline::FootnoteReference(label) => output.push_str(label),
            Inline::LineBreak => output.push('\n'),
        }
    }
    output
}

fn plain_block_text(block: &Block) -> String {
    match block {
        Block::Paragraph(content) | Block::Heading { content, .. } => plain_text(content),
        Block::List { items, .. } => items
            .iter()
            .flat_map(|item| item.iter())
            .map(plain_block_text)
            .filter(|text| !text.trim().is_empty())
            .collect::<Vec<_>>()
            .join("\n"),
        Block::Quote(items) => items
            .iter()
            .map(plain_block_text)
            .filter(|text| !text.trim().is_empty())
            .collect::<Vec<_>>()
            .join("\n"),
        Block::Table { headers, rows } => headers
            .iter()
            .chain(rows.iter().flatten())
            .map(|cell| plain_text(cell))
            .collect::<Vec<_>>()
            .join(" "),
        Block::CodeBlock { .. }
        | Block::DisplayMath(_)
        | Block::FootnoteDefinition { .. }
        | Block::HorizontalRule => String::new(),
    }
}

pub fn import_file(path: &Path) -> Result<ImportedFile, BellbirdError> {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    if !SUPPORTED_EXTENSIONS.contains(&extension.as_str()) {
        return Err(BellbirdError::UnsupportedFormat(extension));
    }
    if !path.metadata()?.is_file() {
        return Err(BellbirdError::UnsupportedFormat(extension));
    }
    let mut file = File::open(path)?;
    if !file.metadata()?.is_file() {
        return Err(BellbirdError::UnsupportedFormat(extension));
    }
    let mut bytes = Vec::new();
    file.by_ref()
        .take((MAX_FILE_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_FILE_BYTES {
        return Err(BellbirdError::FileTooLarge);
    }
    import_bytes(file_name(path), &extension, &bytes)
}

pub fn import_bytes(
    name: impl Into<String>,
    extension: &str,
    bytes: &[u8],
) -> Result<ImportedFile, BellbirdError> {
    let extension = extension.to_ascii_lowercase();
    if !SUPPORTED_EXTENSIONS.contains(&extension.as_str()) {
        return Err(BellbirdError::UnsupportedFormat(extension));
    }
    if bytes.len() > MAX_FILE_BYTES {
        return Err(BellbirdError::FileTooLarge);
    }
    let display_name = name.into();
    let source = match extension.as_str() {
        "md" | "markdown" => ImportedSource::Document(parse_markdown(
            std::str::from_utf8(bytes).map_err(|_| BellbirdError::InvalidUtf8)?,
        )),
        "txt" => ImportedSource::Document(parse_plain_text(
            std::str::from_utf8(bytes).map_err(|_| BellbirdError::InvalidUtf8)?,
        )),
        "tex" => ImportedSource::ExistingLatex(
            std::str::from_utf8(bytes)
                .map_err(|_| BellbirdError::InvalidUtf8)?
                .to_owned(),
        ),
        "rst" => ImportedSource::Document(parse_restructured_text(
            std::str::from_utf8(bytes).map_err(|_| BellbirdError::InvalidUtf8)?,
        )?),
        "org" => {
            let source = std::str::from_utf8(bytes).map_err(|_| BellbirdError::InvalidUtf8)?;
            let mut html = Vec::new();
            orgize::Org::parse(source).write_html(&mut html)?;
            if html.len() > MAX_FILE_BYTES {
                return Err(BellbirdError::FileTooLarge);
            }
            ImportedSource::Document(convert_structured_document(
                display_name.clone(),
                "html",
                &html,
            )?)
        }
        "htm" | "html" => ImportedSource::Document(convert_structured_document(
            display_name.clone(),
            &extension,
            bytes,
        )?),
        "doc" => ImportedSource::Document(convert_structured_document(
            display_name.clone(),
            &extension,
            bytes,
        )?),
        "docx" | "odt" | "epub" => {
            validate_office_archive(bytes)?;
            ImportedSource::Document(convert_structured_document(
                display_name.clone(),
                &extension,
                bytes,
            )?)
        }
        _ => return Err(BellbirdError::UnsupportedFormat(extension)),
    };
    Ok(ImportedFile {
        display_name,
        source,
    })
}

fn parse_restructured_text(input: &str) -> Result<Document, BellbirdError> {
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

fn convert_structured_document(
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

fn convert_docling_node(
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
                .map(|item| {
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
                .flatten()
                .collect()
        }
        Node::KeyValueGraph { cells, .. } => {
            *simplified = true;
            cells
                .iter()
                .filter(|cell| !cell.text.trim().is_empty())
                .map(|cell| paragraph(vec![Inline::Text(format!("{}: {}", cell.label, cell.text))]))
                .flatten()
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

fn validate_office_archive(bytes: &[u8]) -> Result<(), BellbirdError> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes))
        .map_err(|error| BellbirdError::Conversion(format!("invalid document archive: {error}")))?;
    validate_archive_limits(
        &mut archive,
        MAX_ARCHIVE_ENTRIES,
        MAX_ARCHIVE_ENTRY_BYTES,
        MAX_ARCHIVE_TOTAL_BYTES,
    )
}

fn validate_archive_limits<R: Read + std::io::Seek>(
    archive: &mut zip::ZipArchive<R>,
    max_entries: usize,
    max_entry_bytes: u64,
    max_total_bytes: u64,
) -> Result<(), BellbirdError> {
    if archive.len() > max_entries {
        return Err(BellbirdError::ArchiveTooLarge);
    }

    let mut total_bytes = 0_u64;
    let mut buffer = [0_u8; 8192];
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|error| {
            BellbirdError::Conversion(format!("could not inspect document archive: {error}"))
        })?;
        if entry.is_dir() {
            continue;
        }
        if entry.size() > max_entry_bytes
            || total_bytes.saturating_add(entry.size()) > max_total_bytes
        {
            return Err(BellbirdError::ArchiveTooLarge);
        }

        let declared_size = entry.size();
        let mut actual_size = 0_u64;
        loop {
            let read = entry.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            actual_size = actual_size.saturating_add(read as u64);
            if actual_size > max_entry_bytes
                || total_bytes.saturating_add(actual_size) > max_total_bytes
            {
                return Err(BellbirdError::ArchiveTooLarge);
            }
        }
        if actual_size != declared_size {
            return Err(BellbirdError::Conversion(
                "document archive entry size does not match its header".to_owned(),
            ));
        }
        total_bytes = total_bytes.saturating_add(actual_size);
    }
    Ok(())
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(str::to_owned)
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

fn parse_blocks(
    events: &mut Peekable<Parser<'_>>,
    closing: Option<TagEnd>,
    resources: &mut Vec<ResourceReference>,
) -> Vec<Block> {
    let mut blocks = Vec::new();
    while let Some(event) = events.next() {
        match event {
            Event::End(end) if Some(end) == closing => break,
            Event::Start(Tag::Paragraph) => blocks.push(Block::Paragraph(parse_inlines(
                events,
                TagEnd::Paragraph,
                resources,
            ))),
            Event::Start(Tag::Heading { level, .. }) => blocks.push(Block::Heading {
                level: heading_level(level),
                content: parse_inlines(events, TagEnd::Heading(level), resources),
            }),
            Event::Start(Tag::CodeBlock(kind)) => {
                let language = match kind {
                    CodeBlockKind::Fenced(language) if !language.is_empty() => {
                        Some(language.to_string())
                    }
                    CodeBlockKind::Fenced(_) | CodeBlockKind::Indented => None,
                };
                let mut content = String::new();
                for code_event in events.by_ref() {
                    match code_event {
                        Event::End(TagEnd::CodeBlock) => break,
                        Event::Text(text) | Event::Code(text) | Event::Html(text) => {
                            content.push_str(&text)
                        }
                        Event::SoftBreak | Event::HardBreak => content.push('\n'),
                        _ => {}
                    }
                }
                blocks.push(Block::CodeBlock { content, language });
            }
            Event::Start(Tag::List(start)) => {
                let mut items = Vec::new();
                while let Some(item_event) = events.next() {
                    match item_event {
                        Event::Start(Tag::Item) => {
                            items.push(parse_blocks(events, Some(TagEnd::Item), resources))
                        }
                        Event::End(TagEnd::List(_)) => break,
                        _ => {}
                    }
                }
                blocks.push(Block::List {
                    ordered: start.is_some(),
                    start,
                    item_labels: Vec::new(),
                    items,
                });
            }
            Event::Start(Tag::BlockQuote(kind)) => blocks.push(Block::Quote(parse_blocks(
                events,
                Some(TagEnd::BlockQuote(kind)),
                resources,
            ))),
            Event::Start(Tag::Table(_)) => blocks.push(parse_table(events, resources)),
            Event::Start(Tag::FootnoteDefinition(label)) => {
                blocks.push(Block::FootnoteDefinition {
                    label: label.to_string(),
                    content: parse_blocks(events, Some(TagEnd::FootnoteDefinition), resources),
                })
            }
            Event::Start(Tag::HtmlBlock) => {
                let mut html = String::new();
                for html_event in events.by_ref() {
                    match html_event {
                        Event::End(TagEnd::HtmlBlock) => break,
                        Event::Html(text) | Event::Text(text) | Event::InlineHtml(text) => {
                            html.push_str(&text)
                        }
                        _ => {}
                    }
                }
                blocks.push(Block::Paragraph(vec![Inline::Text(html)]));
            }
            Event::DisplayMath(formula) => blocks.push(Block::DisplayMath(formula.to_string())),
            Event::Rule => blocks.push(Block::HorizontalRule),
            Event::Text(text) | Event::Html(text) | Event::InlineHtml(text) => {
                blocks.push(Block::Paragraph(vec![Inline::Text(text.to_string())]));
            }
            Event::End(_)
            | Event::Start(_)
            | Event::Code(_)
            | Event::InlineMath(_)
            | Event::FootnoteReference(_)
            | Event::SoftBreak
            | Event::HardBreak
            | Event::TaskListMarker(_) => {}
        }
    }
    blocks
}

fn parse_table(events: &mut Peekable<Parser<'_>>, resources: &mut Vec<ResourceReference>) -> Block {
    let mut rows = Vec::new();
    while let Some(event) = events.next() {
        match event {
            Event::End(TagEnd::Table) => break,
            Event::Start(Tag::TableHead) => {
                rows.push(parse_table_row(events, TagEnd::TableHead, resources))
            }
            Event::Start(Tag::TableRow) => {
                rows.push(parse_table_row(events, TagEnd::TableRow, resources))
            }
            _ => {}
        }
    }
    let headers = if rows.is_empty() {
        Vec::new()
    } else {
        rows.remove(0)
    };
    Block::Table { headers, rows }
}

fn parse_table_row(
    events: &mut Peekable<Parser<'_>>,
    closing: TagEnd,
    resources: &mut Vec<ResourceReference>,
) -> Vec<Vec<Inline>> {
    let mut cells = Vec::new();
    while let Some(event) = events.next() {
        match event {
            Event::End(end) if end == closing => break,
            Event::Start(Tag::TableCell) => {
                cells.push(parse_inlines(events, TagEnd::TableCell, resources))
            }
            _ => {}
        }
    }
    cells
}

fn parse_inlines(
    events: &mut Peekable<Parser<'_>>,
    closing: TagEnd,
    resources: &mut Vec<ResourceReference>,
) -> Vec<Inline> {
    let mut inlines = Vec::new();
    while let Some(event) = events.next() {
        match event {
            Event::End(end) if end == closing => break,
            Event::Text(text) | Event::InlineHtml(text) | Event::Html(text) => {
                inlines.push(Inline::Text(text.to_string()))
            }
            Event::Code(text) => inlines.push(Inline::Code(text.to_string())),
            Event::InlineMath(formula) => inlines.push(Inline::Math(formula.to_string())),
            Event::FootnoteReference(label) => {
                inlines.push(Inline::FootnoteReference(label.to_string()))
            }
            Event::SoftBreak => inlines.push(Inline::Text(" ".into())),
            Event::HardBreak => inlines.push(Inline::LineBreak),
            Event::Start(Tag::Emphasis) => inlines.push(Inline::Emphasis(parse_inlines(
                events,
                TagEnd::Emphasis,
                resources,
            ))),
            Event::Start(Tag::Strong) => inlines.push(Inline::Strong(parse_inlines(
                events,
                TagEnd::Strong,
                resources,
            ))),
            Event::Start(Tag::Strikethrough) => inlines.push(Inline::Strikethrough(parse_inlines(
                events,
                TagEnd::Strikethrough,
                resources,
            ))),
            Event::Start(Tag::Link {
                dest_url, title, ..
            }) => {
                let content = parse_inlines(events, TagEnd::Link, resources);
                inlines.push(Inline::Link {
                    target: dest_url.to_string(),
                    title: if title.is_empty() {
                        None
                    } else {
                        Some(title.to_string())
                    },
                    content,
                });
            }
            Event::Start(Tag::Image { dest_url, .. }) => {
                let source = dest_url.to_string();
                let alt = parse_inlines(events, TagEnd::Image, resources);
                if !resources.iter().any(|resource| resource.source == source) {
                    resources.push(ResourceReference {
                        source: source.clone(),
                    });
                }
                inlines.push(Inline::Image { source, alt });
            }
            Event::TaskListMarker(checked) => {
                inlines.push(Inline::Text(if checked { "[x] " } else { "[ ] " }.into()))
            }
            Event::Start(tag) => {
                let end = tag.to_end();
                inlines.extend(parse_inlines(events, end, resources));
            }
            Event::End(_) | Event::DisplayMath(_) | Event::Rule => {}
        }
    }
    inlines
}

fn heading_level(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

pub fn path_is_supported(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| SUPPORTED_EXTENSIONS.contains(&extension.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[derive(Debug)]
    enum TestScript {
        Baseline,
    }

    struct TestRun {
        text: String,
        formula: bool,
        code: bool,
        bold: bool,
        italic: bool,
        underline: bool,
        strike: bool,
        script: TestScript,
    }

    impl TestRun {
        fn text(text: &str) -> Self {
            Self {
                text: text.to_owned(),
                formula: false,
                code: false,
                bold: false,
                italic: false,
                underline: false,
                strike: false,
                script: TestScript::Baseline,
            }
        }
    }

    fn archive_with_entries(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut archive = zip::ZipWriter::new(Cursor::new(Vec::new()));
        for (name, content) in entries {
            archive
                .start_file(*name, zip::write::SimpleFileOptions::default())
                .unwrap();
            archive.write_all(content).unwrap();
        }
        archive.finish().unwrap().into_inner()
    }

    #[test]
    fn archive_expansion_is_capped_even_when_the_input_is_compressed() {
        let expanded = vec![b'x'; 1024];
        let bytes = archive_with_entries(&[("word/document.xml", &expanded)]);
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();

        let error = validate_archive_limits(&mut archive, 10, 1024, 512).unwrap_err();

        assert!(matches!(error, BellbirdError::ArchiveTooLarge));
    }

    #[test]
    fn archive_entry_count_is_capped_before_reading_entries() {
        let bytes = archive_with_entries(&[("a.xml", b"a"), ("b.xml", b"b")]);
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();

        let error = validate_archive_limits(&mut archive, 1, 1024, 1024).unwrap_err();

        assert!(matches!(error, BellbirdError::ArchiveTooLarge));
    }

    #[test]
    fn import_file_reads_supported_regular_documents() {
        let path = std::env::temp_dir().join(format!(
            "bellbird-{}-{}.md",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::write(&path, "# File import\n").unwrap();

        let result = import_file(&path);
        std::fs::remove_file(&path).unwrap();

        let imported = result.unwrap();
        assert_eq!(
            imported.display_name,
            path.file_name().unwrap().to_str().unwrap()
        );
        assert!(matches!(imported.source, ImportedSource::Document(_)));
    }

    #[test]
    fn legacy_doc_extension_is_supported_and_routed_to_docling() {
        assert!(path_is_supported(Path::new("paper.doc")));

        let error = import_bytes("paper.doc", "doc", b"not a Word document").unwrap_err();

        assert!(matches!(error, BellbirdError::Conversion(_)));
    }

    #[test]
    fn docling_inline_runs_preserve_spaces_across_formatting_boundaries() {
        let runs = [
            TestRun::text("The"),
            TestRun {
                bold: true,
                ..TestRun::text("quick")
            },
            TestRun::text("fox"),
        ];
        let mut simplified = false;

        let inlines = convert_docling_runs!(&runs, &mut simplified);

        assert_eq!(
            inlines,
            vec![
                Inline::Text("The".to_owned()),
                Inline::Text(" ".to_owned()),
                Inline::Strong(vec![Inline::Text("quick".to_owned())]),
                Inline::Text(" ".to_owned()),
                Inline::Text("fox".to_owned()),
            ]
        );
        assert!(!simplified);
    }

    #[test]
    fn docling_rich_table_cells_keep_formatting_and_paragraph_breaks() {
        use docling::{Node, Table};

        let mut inline_group = Node::InlineGroup {
            unwrapped: false,
            runs: vec![Default::default(), Default::default(), Default::default()],
            md_text: String::new(),
        };
        if let Node::InlineGroup { runs, .. } = &mut inline_group {
            runs[0].text = "destaque".to_owned();
            runs[0].bold = true;
            runs[1].text = "x_y".to_owned();
            runs[1].code = true;
            runs[2].text = "x^2".to_owned();
            runs[2].formula = true;
        }

        let table = Table {
            rows: vec![vec!["Campo".to_owned()], vec!["texto plano".to_owned()]],
            cell_blocks: Some(vec![
                vec![vec![]],
                vec![vec![
                    Node::Paragraph {
                        text: "Primeiro parágrafo".to_owned(),
                    },
                    inline_group,
                ]],
            ]),
            ..Table::default()
        };
        let node = Node::Table(table);
        let mut resources = Vec::new();
        let mut simplified = false;

        let blocks = convert_docling_node(&node, &mut resources, &mut simplified);

        let Block::Table { rows, .. } = &blocks[0] else {
            panic!("expected a table block");
        };
        assert_eq!(
            rows[0][0],
            vec![
                Inline::Text("Primeiro parágrafo".to_owned()),
                Inline::LineBreak,
                Inline::Strong(vec![Inline::Text("destaque".to_owned())]),
                Inline::Text(" ".to_owned()),
                Inline::Code("x_y".to_owned()),
                Inline::Text(" ".to_owned()),
                Inline::Math("x^2".to_owned()),
            ]
        );
        assert!(!simplified);
    }

    #[test]
    fn docling_table_cells_fall_back_when_rich_blocks_are_absent() {
        use docling::{Node, Table};

        let cell_block_variants: [Option<Vec<Vec<Vec<Node>>>>; 2] =
            [None, Some(vec![vec![Vec::new()]])];
        for cell_blocks in cell_block_variants {
            let table = Table {
                rows: vec![vec!["texto plano".to_owned()]],
                cell_blocks,
                ..Table::default()
            };
            let node = Node::Table(table);
            let mut resources = Vec::new();
            let mut simplified = false;

            let blocks = convert_docling_node(&node, &mut resources, &mut simplified);

            let Block::Table { headers, .. } = &blocks[0] else {
                panic!("expected a table block");
            };
            assert_eq!(headers[0], vec![Inline::Text("texto plano".to_owned())]);
            assert!(!simplified);
        }
    }

    #[test]
    fn docling_nested_table_cell_content_remains_readable_and_is_marked_simplified() {
        use docling::{Node, Table};

        let table = Table {
            rows: vec![vec!["flat fallback".to_owned()]],
            cell_blocks: Some(vec![vec![vec![Node::ListItem {
                ordered: false,
                number: 1,
                first_in_list: true,
                text: "item".to_owned(),
                level: 0,
                marker: None,
                location: None,
                dclx: None,
                href: None,
                layer: None,
            }]]]),
            ..Table::default()
        };
        let node = Node::Table(table);
        let mut resources = Vec::new();
        let mut simplified = false;

        let blocks = convert_docling_node(&node, &mut resources, &mut simplified);

        let Block::Table { headers, .. } = &blocks[0] else {
            panic!("expected a table block");
        };
        assert_eq!(headers[0], vec![Inline::Text("item".to_owned())]);
        assert!(simplified);
    }

    #[cfg(unix)]
    #[test]
    fn import_file_rejects_named_pipes_without_blocking() {
        use std::process::Command;
        use std::time::Duration;

        let path = std::env::temp_dir().join(format!(
            "bellbird-{}-{}.txt",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let status = Command::new("mkfifo").arg(&path).status().unwrap();
        assert!(status.success(), "mkfifo failed for {}", path.display());

        let (sender, receiver) = std::sync::mpsc::channel();
        let worker_path = path.clone();
        let worker = std::thread::spawn(move || {
            let _ = sender.send(import_file(&worker_path));
        });
        let result = receiver.recv_timeout(Duration::from_secs(1));
        let blocked = result.is_err();
        if blocked {
            let _ = std::fs::OpenOptions::new().write(true).open(&path);
        }
        let result = if blocked {
            receiver.recv_timeout(Duration::from_secs(1))
        } else {
            result
        };
        worker.join().unwrap();
        std::fs::remove_file(&path).unwrap();

        assert!(!blocked, "opening a named pipe blocked the import worker");
        assert!(matches!(
            result.unwrap(),
            Err(BellbirdError::UnsupportedFormat(_))
        ));
    }
}
