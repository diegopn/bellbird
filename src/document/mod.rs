pub use crate::latex::{
    LatexTemplate, escape_latex, render_latex, used_safe_fallback, validate_generated_latex,
};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Document {
    pub metadata: Metadata,
    pub blocks: Vec<Block>,
    pub references: Vec<Reference>,
    pub resources: Vec<ResourceReference>,
    pub has_simplified_content: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Metadata {
    pub title: Option<String>,
    pub authors: Vec<String>,
    pub abstract_text: Option<String>,
    pub keywords: Vec<String>,
    pub language: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reference {
    pub key: String,
    pub authors: Vec<String>,
    pub title: String,
    pub year: Option<String>,
    pub url: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceReference {
    pub source: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Block {
    Heading {
        level: u8,
        content: Vec<Inline>,
    },
    Paragraph(Vec<Inline>),
    List {
        ordered: bool,
        start: Option<u64>,
        item_labels: Vec<Option<String>>,
        items: Vec<Vec<Block>>,
    },
    CodeBlock {
        content: String,
        language: Option<String>,
    },
    Quote(Vec<Block>),
    Table {
        headers: Vec<Vec<Inline>>,
        rows: Vec<Vec<Vec<Inline>>>,
    },
    DisplayMath(String),
    FootnoteDefinition {
        label: String,
        content: Vec<Block>,
    },
    HorizontalRule,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Inline {
    Text(String),
    Emphasis(Vec<Inline>),
    Strong(Vec<Inline>),
    Strikethrough(Vec<Inline>),
    Underline(Vec<Inline>),
    Subscript(Vec<Inline>),
    Superscript(Vec<Inline>),
    Code(String),
    Link {
        target: String,
        title: Option<String>,
        content: Vec<Inline>,
    },
    Image {
        source: String,
        alt: Vec<Inline>,
    },
    Math(String),
    Citation(Vec<String>),
    FootnoteReference(String),
    LineBreak,
}

pub(crate) fn plain_block_text(block: &Block) -> String {
    match block {
        Block::Heading { content, .. } | Block::Paragraph(content) => plain_inline_text(content),
        Block::List { items, .. } => items
            .iter()
            .flat_map(|item| item.iter())
            .map(plain_block_text)
            .filter(|text| !text.trim().is_empty())
            .collect::<Vec<_>>()
            .join("\n"),
        Block::Quote(children)
        | Block::FootnoteDefinition {
            content: children, ..
        } => children
            .iter()
            .map(plain_block_text)
            .filter(|text| !text.trim().is_empty())
            .collect::<Vec<_>>()
            .join("\n"),
        Block::Table { headers, rows } => headers
            .iter()
            .chain(rows.iter().flatten())
            .map(|cell| plain_inline_text(cell))
            .collect::<Vec<_>>()
            .join(" | "),
        Block::CodeBlock { content, .. } | Block::DisplayMath(content) => content.clone(),
        Block::HorizontalRule => String::new(),
    }
}

fn plain_inline_text(inlines: &[Inline]) -> String {
    let mut output = String::new();
    for inline in inlines {
        match inline {
            Inline::Text(text) | Inline::Code(text) | Inline::Math(text) => output.push_str(text),
            Inline::Emphasis(children)
            | Inline::Strong(children)
            | Inline::Strikethrough(children)
            | Inline::Underline(children)
            | Inline::Subscript(children)
            | Inline::Superscript(children) => output.push_str(&plain_inline_text(children)),
            Inline::Link { content, .. } => output.push_str(&plain_inline_text(content)),
            Inline::Image { alt, .. } => output.push_str(&plain_inline_text(alt)),
            Inline::Citation(keys) => {
                output.push_str("[citação: ");
                output.push_str(&keys.join(", "));
                output.push(']');
            }
            Inline::FootnoteReference(label) => output.push_str(label),
            Inline::LineBreak => output.push('\n'),
        }
    }
    output
}
