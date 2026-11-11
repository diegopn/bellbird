use crate::document::{Block, Document, Inline};

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

pub(super) fn extract_explicit_metadata(document: &mut Document) {
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

pub(super) fn plain_block_text(block: &Block) -> String {
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
