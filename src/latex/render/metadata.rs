use crate::document::Document;
use crate::latex::{LatexTemplate, escape_latex};

pub(super) fn template_source(template: LatexTemplate) -> &'static str {
    match template {
        LatexTemplate::Standard => include_str!("../templates/standard.tex"),
        LatexTemplate::Abnt => include_str!("../templates/abnt.tex"),
        LatexTemplate::Ieee => include_str!("../templates/ieee.tex"),
        LatexTemplate::Acm => include_str!("../templates/acm.tex"),
        LatexTemplate::Elsevier => include_str!("../templates/elsevier.tex"),
        LatexTemplate::SpringerNature => include_str!("../templates/springer-nature.tex"),
    }
}

pub(super) fn render_title_and_authors(
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

pub(super) fn render_abstract(document: &Document, template: LatexTemplate, output: &mut String) {
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

pub(super) fn render_keywords(document: &Document, template: LatexTemplate, output: &mut String) {
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
