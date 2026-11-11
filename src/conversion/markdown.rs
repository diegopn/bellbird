use std::iter::Peekable;

use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

use crate::document::{Block, Document, Inline, ResourceReference};

use super::plain_text::extract_explicit_metadata;

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

pub(super) fn markdown_options() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_MATH
}

pub(super) fn parse_blocks(
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
