use std::collections::{BTreeMap, BTreeSet};

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LatexTemplate {
    Standard,
    Abnt,
    Ieee,
    Acm,
    Elsevier,
    SpringerNature,
}

const SAFE_FALLBACK_NOTICE: &str = "% Bellbird: conteúdo irregular foi convertido para texto simples para preservar um LaTeX válido.";

impl LatexTemplate {
    pub const EXPORT_OPTIONS: [Self; 5] = [
        Self::Abnt,
        Self::Ieee,
        Self::Acm,
        Self::Elsevier,
        Self::SpringerNature,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Standard => "Standard LaTeX",
            Self::Abnt => "ABNT · abnTeX2",
            Self::Ieee => "IEEE · IEEEtran",
            Self::Acm => "ACM · acmart",
            Self::Elsevier => "Elsevier · elsarticle",
            Self::SpringerNature => "Springer Nature · sn-jnl",
        }
    }

    pub const fn source_url(self) -> Option<&'static str> {
        match self {
            Self::Standard => None,
            Self::Abnt => Some("https://ctan.org/pkg/abntex2"),
            Self::Ieee => Some("https://ctan.org/pkg/ieeetran"),
            Self::Acm => Some("https://www.acm.org/publications/proceedings-template"),
            Self::Elsevier => Some("https://ctan.org/pkg/elsarticle"),
            Self::SpringerNature => {
                Some("https://www.springernature.com/gp/authors/campaigns/latex-author-support")
            }
        }
    }
}

pub fn escape_latex(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len());
    for character in input.chars() {
        escaped.push_str(match character {
            '\\' => r"\textbackslash{}",
            '{' => r"\{",
            '}' => r"\}",
            '$' => r"\$",
            '&' => r"\&",
            '%' => r"\%",
            '#' => r"\#",
            '_' => r"\_",
            '^' => r"\textasciicircum{}",
            '~' => r"\textasciitilde{}",
            _ => {
                escaped.push(character);
                continue;
            }
        });
    }
    escaped
}

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

fn plain_block_text(block: &Block) -> String {
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

fn collect_citation_keys(blocks: &[Block]) -> BTreeMap<String, String> {
    let mut keys = BTreeMap::new();
    collect_citation_keys_from_blocks(blocks, &mut keys);
    keys
}

fn collect_citation_keys_from_blocks(blocks: &[Block], keys: &mut BTreeMap<String, String>) {
    for block in blocks {
        match block {
            Block::Heading { content, .. } | Block::Paragraph(content) => {
                collect_citation_keys_from_inlines(content, keys)
            }
            Block::List { items, .. } => {
                for item in items {
                    collect_citation_keys_from_blocks(item, keys);
                }
            }
            Block::Quote(children)
            | Block::FootnoteDefinition {
                content: children, ..
            } => collect_citation_keys_from_blocks(children, keys),
            Block::Table { headers, rows } => {
                collect_citation_keys_from_row(headers, keys);
                for row in rows {
                    collect_citation_keys_from_row(row, keys);
                }
            }
            Block::CodeBlock { .. } | Block::DisplayMath(_) | Block::HorizontalRule => {}
        }
    }
}

fn collect_citation_keys_from_row(row: &[Vec<Inline>], keys: &mut BTreeMap<String, String>) {
    for cell in row {
        collect_citation_keys_from_inlines(cell, keys);
    }
}

fn collect_citation_keys_from_inlines(inlines: &[Inline], keys: &mut BTreeMap<String, String>) {
    for inline in inlines {
        match inline {
            Inline::Citation(citations) => {
                for citation in citations.iter().filter(|key| !key.trim().is_empty()) {
                    keys.entry(latex_reference_key(citation))
                        .or_insert_with(|| citation.trim().to_owned());
                }
            }
            Inline::Emphasis(children)
            | Inline::Strong(children)
            | Inline::Strikethrough(children)
            | Inline::Underline(children)
            | Inline::Subscript(children)
            | Inline::Superscript(children) => collect_citation_keys_from_inlines(children, keys),
            Inline::Link { content, .. } => collect_citation_keys_from_inlines(content, keys),
            Inline::Image { alt, .. } => collect_citation_keys_from_inlines(alt, keys),
            Inline::Text(_)
            | Inline::Code(_)
            | Inline::Math(_)
            | Inline::FootnoteReference(_)
            | Inline::LineBreak => {}
        }
    }
}

fn latex_reference_key(key: &str) -> String {
    let mut result = String::new();
    for character in key.trim().chars() {
        if character.is_ascii_alphanumeric() {
            result.push(character);
        } else {
            result.push_str(&format!("_{:X}_", character as u32));
        }
    }
    if result.is_empty() {
        "reference".to_owned()
    } else {
        result
    }
}

fn template_source(template: LatexTemplate) -> &'static str {
    match template {
        LatexTemplate::Standard => include_str!("templates/standard.tex"),
        LatexTemplate::Abnt => include_str!("templates/abnt.tex"),
        LatexTemplate::Ieee => include_str!("templates/ieee.tex"),
        LatexTemplate::Acm => include_str!("templates/acm.tex"),
        LatexTemplate::Elsevier => include_str!("templates/elsevier.tex"),
        LatexTemplate::SpringerNature => include_str!("templates/springer-nature.tex"),
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

fn collect_footnote_numbers(blocks: &[Block]) -> FootnoteNumbers {
    let mut numbers = FootnoteNumbers::default();
    collect_footnote_numbers_from_blocks(blocks, &mut numbers);
    numbers
}

type FootnoteNumbers = BTreeMap<String, usize>;

fn collect_footnote_numbers_from_blocks(blocks: &[Block], numbers: &mut FootnoteNumbers) {
    for block in blocks {
        match block {
            Block::Heading { content, .. } | Block::Paragraph(content) => {
                collect_footnote_numbers_from_inlines(content, numbers);
            }
            Block::List { items, .. } => {
                for item in items {
                    collect_footnote_numbers_from_blocks(item, numbers);
                }
            }
            Block::Quote(children) => collect_footnote_numbers_from_blocks(children, numbers),
            Block::Table { headers, rows } => {
                collect_footnote_numbers_from_row(headers, numbers);
                for row in rows {
                    collect_footnote_numbers_from_row(row, numbers);
                }
            }
            Block::FootnoteDefinition { content, .. } => {
                collect_footnote_numbers_from_blocks(content, numbers);
            }
            Block::CodeBlock { .. } | Block::DisplayMath(_) | Block::HorizontalRule => {}
        }
    }
}

fn collect_footnote_numbers_from_row(row: &[Vec<Inline>], numbers: &mut FootnoteNumbers) {
    for cell in row {
        collect_footnote_numbers_from_inlines(cell, numbers);
    }
}

fn collect_footnote_numbers_from_inlines(inlines: &[Inline], numbers: &mut FootnoteNumbers) {
    for inline in inlines {
        match inline {
            Inline::FootnoteReference(label) => {
                let next = numbers.len() + 1;
                numbers.entry(label.clone()).or_insert(next);
            }
            Inline::Emphasis(children)
            | Inline::Strong(children)
            | Inline::Strikethrough(children)
            | Inline::Underline(children)
            | Inline::Subscript(children)
            | Inline::Superscript(children) => {
                collect_footnote_numbers_from_inlines(children, numbers);
            }
            Inline::Link { content, .. } => {
                collect_footnote_numbers_from_inlines(content, numbers);
            }
            Inline::Image { alt, .. } => collect_footnote_numbers_from_inlines(alt, numbers),
            Inline::Text(_)
            | Inline::Code(_)
            | Inline::Math(_)
            | Inline::Citation(_)
            | Inline::LineBreak => {}
        }
    }
}

fn render_blocks(
    blocks: &[Block],
    output: &mut String,
    template: LatexTemplate,
    footnote_numbers: &FootnoteNumbers,
) {
    for block in blocks {
        match block {
            Block::Heading { level, content } => {
                let command = match (template, level) {
                    (_, 1) => "section",
                    (_, 2) => "subsection",
                    (_, 3) => "subsubsection",
                    (_, 4) => "paragraph",
                    _ => "subparagraph",
                };
                output.push_str(&format!(
                    "\\{command}{{{}}}\n",
                    render_inlines(content, footnote_numbers)
                ));
            }
            Block::Paragraph(content) => {
                output.push_str(&render_inlines(content, footnote_numbers));
                output.push_str("\n\n");
            }
            Block::List {
                ordered,
                start,
                item_labels,
                items,
            } => {
                let environment = if *ordered { "enumerate" } else { "itemize" };
                output.push_str(&format!("\\begin{{{environment}}}\n"));
                for (index, item) in items.iter().enumerate() {
                    if *ordered
                        && let Some(label) = item_labels.get(index).and_then(Option::as_deref)
                    {
                        output.push_str(&format!("\\item[{label}] "));
                    } else if *ordered && let Some(start) = start.filter(|start| *start != 1) {
                        output
                            .push_str(&format!("\\item[{}.] ", u128::from(start) + index as u128));
                    } else {
                        output.push_str("\\item ");
                    }
                    render_blocks(item, output, template, footnote_numbers);
                }
                output.push_str(&format!("\\end{{{environment}}}\n\n"));
            }
            Block::CodeBlock { content, .. } => render_code_block(content, output),
            Block::Quote(content) => {
                output.push_str("\\begin{quote}\n");
                render_blocks(content, output, template, footnote_numbers);
                output.push_str("\\end{quote}\n\n");
            }
            Block::Table { headers, rows } => render_table(headers, rows, output, footnote_numbers),
            Block::DisplayMath(formula) => output.push_str(&format!("\\[\n{formula}\n\\]\n")),
            Block::FootnoteDefinition { label, content } => {
                if let Some(number) = footnote_numbers.get(label) {
                    output.push_str(&format!("\\footnotetext[{number}]{{"));
                    let mut note = String::new();
                    render_blocks(content, &mut note, template, footnote_numbers);
                    output.push_str(note.trim());
                    output.push_str("}\n");
                }
            }
            Block::HorizontalRule => output.push_str("\\noindent\\rule{\\linewidth}{0.4pt}\n\n"),
        }
    }
}

fn render_table(
    headers: &[Vec<Inline>],
    rows: &[Vec<Vec<Inline>>],
    output: &mut String,
    footnote_numbers: &FootnoteNumbers,
) {
    let columns = headers
        .len()
        .max(rows.iter().map(Vec::len).max().unwrap_or(0));
    if columns == 0 {
        return;
    }
    let width = 0.95_f64 / columns as f64;
    let layout = (0..columns)
        .map(|_| format!("p{{\\dimexpr{width:.4}\\linewidth - 2pt\\relax}}"))
        .collect::<Vec<_>>()
        .join("@{\\hspace{2pt}}");
    let layout = format!("@{{}}{layout}@{{}}");
    output.push_str(&format!("\\begin{{tabular}}{{{layout}}}\n\\hline\n"));
    if !headers.is_empty() {
        render_table_row(headers, columns, output, footnote_numbers);
        output.push_str("\\\\\n\\hline\n");
    }
    for row in rows {
        render_table_row(row, columns, output, footnote_numbers);
        output.push_str("\\\\\n");
    }
    output.push_str("\\hline\n\\end{tabular}\n\n");
}

fn render_code_block(content: &str, output: &mut String) {
    if content.contains("\\end{verbatim}") {
        output.push_str("\\begin{quote}\\ttfamily\n");
        for (index, line) in content.lines().enumerate() {
            if index > 0 {
                output.push_str("\\\\\n");
            }
            output.push_str(&escape_latex(line));
        }
        output.push_str("\n\\end{quote}\n");
        return;
    }

    output.push_str("\\begin{verbatim}\n");
    output.push_str(content);
    if !content.ends_with('\n') {
        output.push('\n');
    }
    output.push_str("\\end{verbatim}\n\n");
}

fn render_table_row(
    cells: &[Vec<Inline>],
    columns: usize,
    output: &mut String,
    footnote_numbers: &FootnoteNumbers,
) {
    for index in 0..columns {
        if index > 0 {
            output.push_str(" & ");
        }
        if let Some(cell) = cells.get(index) {
            output.push_str(&render_inlines(cell, footnote_numbers));
        }
    }
}

fn render_inlines(inlines: &[Inline], footnote_numbers: &FootnoteNumbers) -> String {
    let mut output = String::new();
    for inline in inlines {
        match inline {
            Inline::Text(text) => output.push_str(&escape_latex(text)),
            Inline::Emphasis(children) => output.push_str(&format!(
                "\\emph{{{}}}",
                render_inlines(children, footnote_numbers)
            )),
            Inline::Strong(children) => output.push_str(&format!(
                "\\textbf{{{}}}",
                render_inlines(children, footnote_numbers)
            )),
            Inline::Strikethrough(children) => output.push_str(&format!(
                "\\sout{{{}}}",
                render_inlines(children, footnote_numbers)
            )),
            Inline::Underline(children) => output.push_str(&format!(
                "\\underline{{{}}}",
                render_inlines(children, footnote_numbers)
            )),
            Inline::Subscript(children) => output.push_str(&format!(
                "\\textsubscript{{{}}}",
                render_inlines(children, footnote_numbers)
            )),
            Inline::Superscript(children) => output.push_str(&format!(
                "\\textsuperscript{{{}}}",
                render_inlines(children, footnote_numbers)
            )),
            Inline::Code(text) => output.push_str(&format!("\\texttt{{{}}}", escape_latex(text))),
            Inline::Link {
                target, content, ..
            } => {
                let content = render_inlines(content, footnote_numbers);
                if is_external_url(target) {
                    output.push_str(&format!("\\href{{{}}}{{{content}}}", escape_latex(target)));
                } else {
                    output.push_str(&content);
                }
            }
            Inline::Image { alt, .. } => output.push_str(&render_inlines(alt, footnote_numbers)),
            Inline::Math(formula) => output.push_str(&format!("\\({formula}\\)")),
            Inline::Citation(keys) => {
                let keys = keys
                    .iter()
                    .filter(|key| !key.trim().is_empty())
                    .map(|key| latex_reference_key(key))
                    .collect::<Vec<_>>();
                if keys.is_empty() {
                    output.push_str("[citação sem identificador]");
                } else {
                    output.push_str(&format!("\\cite{{{}}}", keys.join(",")));
                }
            }
            Inline::FootnoteReference(label) => {
                if let Some(number) = footnote_numbers.get(label) {
                    output.push_str(&format!("\\footnotemark[{number}]"));
                } else {
                    output.push_str("\\footnotemark{}");
                }
            }
            Inline::LineBreak => output.push_str("\\\\\n"),
        }
    }
    output
}

fn render_references(
    references: &[Reference],
    citation_keys: &BTreeMap<String, String>,
    output: &mut String,
) {
    if references.is_empty() && citation_keys.is_empty() {
        return;
    }
    output.push_str("\\begin{thebibliography}{99}\n");
    let mut rendered_keys = BTreeSet::new();
    for reference in references {
        let key = latex_reference_key(&reference.key);
        if !rendered_keys.insert(key.clone()) {
            continue;
        }
        output.push_str(&format!("\\bibitem{{{key}}} "));
        if !reference.authors.is_empty() {
            output.push_str(&escape_latex(&reference.authors.join(", ")));
        } else {
            output.push_str("[INSERIR AUTOR DA REFERÊNCIA]");
        }
        output.push_str(". ");
        let title = if reference.title.trim().is_empty() {
            "[INSERIR TÍTULO DA REFERÊNCIA]"
        } else {
            &reference.title
        };
        output.push_str(&format!("\\textit{{{}}}", escape_latex(title)));
        if let Some(year) = &reference.year {
            output.push_str(&format!(", {}", escape_latex(year)));
        } else {
            output.push_str(", [INSERIR ANO]");
        }
        if let Some(url) = reference.url.as_deref().filter(|url| is_external_url(url)) {
            output.push_str(&format!(". \\url{{{}}}", escape_latex(url)));
        }
        output.push_str(".\n");
    }
    for (key, original_key) in citation_keys {
        if rendered_keys.contains(key) {
            continue;
        }
        output.push_str(&format!(
            "\\bibitem{{{key}}} [INSERIR REFERÊNCIA PARA {}].\n",
            escape_latex(original_key)
        ));
    }
    output.push_str("\\end{thebibliography}\n");
}

fn is_external_url(target: &str) -> bool {
    let target = target.trim().to_ascii_lowercase();
    ["http://", "https://", "mailto:", "ftp://", "tel:"]
        .iter()
        .any(|scheme| target.starts_with(scheme))
}

pub fn validate_generated_latex(latex: &str) -> Result<(), String> {
    if [
        "{{PACKAGES}}",
        "{{TITLE_AUTHOR}}",
        "{{ABSTRACT}}",
        "{{KEYWORDS}}",
        "{{BODY}}",
        "{{REFERENCES}}",
    ]
    .iter()
    .any(|marker| latex.contains(marker))
    {
        return Err("an internal template marker was not replaced".to_owned());
    }

    let bytes = latex.as_bytes();
    let mut index = 0;
    let mut brace_depth = 0usize;
    let mut environments = Vec::new();
    let mut citations = BTreeSet::new();
    let mut bibliography_items = BTreeSet::new();

    while index < bytes.len() {
        match bytes[index] {
            b'%' => {
                while index < bytes.len() && bytes[index] != b'\n' {
                    index += 1;
                }
            }
            b'{' => {
                brace_depth += 1;
                index += 1;
            }
            b'}' => {
                if brace_depth == 0 {
                    return Err("a closing brace has no matching opening brace".to_owned());
                }
                brace_depth -= 1;
                index += 1;
            }
            b'\\' => {
                index += 1;
                if index == bytes.len() {
                    return Err("the document ends with an incomplete command".to_owned());
                }
                let command_start = index;
                if bytes[index].is_ascii_alphabetic() {
                    while index < bytes.len() && bytes[index].is_ascii_alphabetic() {
                        index += 1;
                    }
                } else {
                    index += 1;
                }
                let command = std::str::from_utf8(&bytes[command_start..index])
                    .map_err(|_| "the document contains an invalid command".to_owned())?;

                if command == "verb" {
                    if bytes.get(index) == Some(&b'*') {
                        index += 1;
                    }
                    let Some(&delimiter) = bytes.get(index) else {
                        return Err("an inline verbatim command is incomplete".to_owned());
                    };
                    index += 1;
                    let Some(end) = bytes[index..].iter().position(|byte| *byte == delimiter)
                    else {
                        return Err(
                            "an inline verbatim command has no closing delimiter".to_owned()
                        );
                    };
                    index += end + 1;
                    continue;
                }

                if is_external_file_command(command) {
                    return Err(format!("\\{command} references a separate content file"));
                }

                if command == "begin" || command == "end" {
                    let Some((environment, end)) = read_braced_group(bytes, index) else {
                        return Err(format!("\\{command} has no environment name"));
                    };
                    let environment = environment.trim();
                    if command == "begin" && is_verbatim_environment(environment) {
                        let marker = format!("\\end{{{environment}}}");
                        let Some(offset) = bytes[end..]
                            .windows(marker.len())
                            .position(|window| window == marker.as_bytes())
                        else {
                            return Err(format!("environment `{environment}` is not closed"));
                        };
                        index = end + offset + marker.len();
                        continue;
                    }
                    if command == "begin" {
                        environments.push(environment.to_owned());
                    } else if environments.pop().as_deref() != Some(environment) {
                        return Err(format!("environment `{environment}` closes out of order"));
                    }
                    index = end;
                    continue;
                }

                if is_citation_command(command) || command == "bibitem" {
                    let Some((keys, end)) = read_braced_group(bytes, index) else {
                        return Err(format!("\\{command} has no key list"));
                    };
                    let destination = if command == "bibitem" {
                        &mut bibliography_items
                    } else {
                        &mut citations
                    };
                    destination.extend(
                        keys.split(',')
                            .map(str::trim)
                            .filter(|key| !key.is_empty())
                            .map(str::to_owned),
                    );
                    index = end;
                    continue;
                }
            }
            _ => index += 1,
        }
    }

    if brace_depth != 0 {
        return Err("one or more opening braces are not closed".to_owned());
    }
    if let Some(environment) = environments.last() {
        return Err(format!("environment `{environment}` is not closed"));
    }
    if let Some(key) = citations.difference(&bibliography_items).next() {
        return Err(format!("citation `{key}` has no inline bibliography entry"));
    }
    Ok(())
}

fn read_braced_group(bytes: &[u8], start: usize) -> Option<(String, usize)> {
    let mut index = start;
    while bytes.get(index).is_some_and(u8::is_ascii_whitespace) {
        index += 1;
    }
    if bytes.get(index) != Some(&b'{') {
        return None;
    }

    let content_start = index + 1;
    index = content_start;
    let mut depth = 1usize;
    while index < bytes.len() {
        match bytes[index] {
            b'\\' => index = (index + 2).min(bytes.len()),
            b'{' => {
                depth += 1;
                index += 1;
            }
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    let content = std::str::from_utf8(&bytes[content_start..index]).ok()?;
                    return Some((content.to_owned(), index + 1));
                }
                index += 1;
            }
            _ => index += 1,
        }
    }
    None
}

fn is_citation_command(command: &str) -> bool {
    matches!(
        command,
        "cite"
            | "citep"
            | "citet"
            | "citeauthor"
            | "citeyear"
            | "autocite"
            | "parencite"
            | "textcite"
    )
}

fn is_external_file_command(command: &str) -> bool {
    matches!(
        command,
        "input"
            | "include"
            | "includegraphics"
            | "bibliography"
            | "bibliographystyle"
            | "addbibresource"
            | "includepdf"
            | "lstinputlisting"
            | "inputminted"
            | "verbatiminput"
            | "graphicspath"
            | "import"
            | "subimport"
            | "openin"
            | "openout"
            | "read"
            | "write"
            | "IfFileExists"
            | "InputIfFileExists"
            | "includeonly"
    )
}

fn is_verbatim_environment(environment: &str) -> bool {
    matches!(
        environment,
        "verbatim" | "verbatim*" | "lstlisting" | "minted"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

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
