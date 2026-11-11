use crate::document::{Block, Inline};
use crate::latex::LatexTemplate;
use crate::latex::escape_latex;

use super::inline::{FootnoteNumbers, render_inlines, render_inlines_with_context};

pub(super) fn render_blocks(
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
            output.push_str(&render_inlines_with_context(cell, footnote_numbers, true));
        }
    }
}
