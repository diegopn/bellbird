use std::io::{Cursor, Write};

use bellbird::conversion::{
    ImportedSource, MAX_FILE_BYTES, import_bytes, parse_markdown, parse_plain_text,
};
use bellbird::document::{Block, Document, Inline, LatexTemplate, render_latex};
use bellbird::error::BellbirdError;

#[test]
fn markdown_keeps_headings_lists_and_unicode() {
    let document = parse_markdown("# Café\n\nOlá, mundo!\n\n- primeiro\n- segundo\n");

    assert!(matches!(
        document.blocks[0],
        Block::Heading { level: 1, .. }
    ));
    assert!(matches!(
        document.blocks[2],
        Block::List { ordered: false, .. }
    ));
    let latex = render_latex(&document, LatexTemplate::Standard);
    assert!(latex.contains("\\section{Café}"), "{latex}");
    assert!(latex.contains("Olá, mundo!"));
    assert!(latex.contains("\\begin{itemize}"));
}

#[test]
fn markdown_ordered_lists_preserve_their_start_number() {
    let document = parse_markdown("3. terceiro\n4. quarto\n");
    let latex = render_latex(&document, LatexTemplate::Standard);

    assert!(latex.contains(r"\item[3.] terceiro"), "{latex}");
    assert!(latex.contains(r"\item[4.] quarto"), "{latex}");
}

#[test]
fn plain_text_does_not_infer_headings_and_escapes_latex() {
    let document = parse_plain_text("Title-like line\n\nPrice: $5 & 10%\n");

    assert!(
        document
            .blocks
            .iter()
            .all(|block| matches!(block, Block::Paragraph(_)))
    );
    let latex = render_latex(&document, LatexTemplate::Standard);
    assert!(latex.contains("Title-like line"));
    assert!(latex.contains("Price: \\$5 \\& 10\\%"));
}

#[test]
fn markdown_links_and_code_are_rendered_without_losing_targets() {
    let document = parse_markdown("Leia [a documentação](https://example.org/a_b) e `x_y`.\n");
    let latex = render_latex(&document, LatexTemplate::Standard);

    assert!(latex.contains("\\href{https://example.org/a\\_b}{a documentação}"));
    assert!(latex.contains("\\texttt{x\\_y}"));
}

#[test]
fn templates_use_their_academic_classes() {
    use bellbird::document::Metadata;

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

    let abnt = render_latex(&document, LatexTemplate::Abnt);
    assert!(abnt.contains(r"\documentclass[article,12pt,oneside,a4paper]{abntex2}"));
    assert!(abnt.contains(r"\usepackage[alf]{abntex2cite}"));
    assert!(abnt.contains("\\titulo{Sample title}"));
    assert!(abnt.contains("\\begin{resumo}"));
    assert!(abnt.contains("Palavras-chave:"));
    assert!(abnt.contains(r"\textual"));
    assert!(abnt.contains(r"\postextual"));

    let ieee = render_latex(&document, LatexTemplate::Ieee);
    assert!(
        ieee.contains(r"\documentclass[conference]{IEEEtran}"),
        "{ieee}"
    );
    assert!(ieee.contains(r"\IEEEauthorblockN{First Author, Second Author}"));
    assert!(ieee.contains("\\begin{IEEEkeywords}"));

    let acm = render_latex(&document, LatexTemplate::Acm);
    assert!(acm.contains("\\documentclass[sigconf]{acmart}"));
    assert!(acm.contains("\\keywords{first, second}"));
    assert!(acm.contains("\\author{First Author}"));
    assert!(acm.contains("\\author{Second Author}"));

    let elsevier = render_latex(&document, LatexTemplate::Elsevier);
    assert!(elsevier.contains("\\documentclass[preprint,12pt]{elsarticle}"));
    assert!(elsevier.contains("\\begin{frontmatter}"));
    assert!(elsevier.contains("\\begin{keyword}\nfirst \\sep second\n\\end{keyword}"));
    assert!(elsevier.contains("\\end{frontmatter}"));

    let springer = render_latex(&document, LatexTemplate::SpringerNature);
    assert!(springer.contains("\\documentclass[pdflatex,sn-mathphys-num]{sn-jnl}"));
    assert!(springer.contains("\\abstract{A short abstract.}"));
    assert!(springer.contains("\\keywords{first, second}"));
    assert!(springer.contains("\\title[Sample title]{Sample title}"));
    assert!(springer.contains("\\author[]{\\fnm{First} \\sur{Author}}"));
}

#[test]
fn export_template_options_match_the_academic_styles() {
    let labels = LatexTemplate::EXPORT_OPTIONS.map(LatexTemplate::label);

    assert_eq!(
        labels,
        [
            "ABNT · abnTeX2",
            "IEEE · IEEEtran",
            "ACM · acmart",
            "Elsevier · elsarticle",
            "Springer Nature · sn-jnl",
        ]
    );
    assert!(
        LatexTemplate::EXPORT_OPTIONS
            .iter()
            .all(|template| template.source_url().is_some())
    );
}

#[test]
fn nested_markdown_emphasis_is_preserved() {
    let document = parse_markdown("**forte e _ênfase_**\n");
    let latex = render_latex(&document, LatexTemplate::Standard);

    assert!(
        latex.contains("\\textbf{forte e \\emph{ênfase}}"),
        "{latex}"
    );
}

#[test]
fn existing_latex_is_passed_through_without_template_changes() {
    let source = "\\documentclass{article}\n% café\n\\begin{document}x_1\\end{document}";
    let imported = import_bytes("source.tex", "TEX", source.as_bytes()).unwrap();

    assert_eq!(imported.display_name, "source.tex");
    assert_eq!(
        imported.source,
        ImportedSource::ExistingLatex(source.into())
    );
}

#[test]
fn invalid_utf8_and_oversized_files_are_rejected() {
    assert!(matches!(
        import_bytes("bad.md", "md", &[0xff]),
        Err(BellbirdError::InvalidUtf8)
    ));
    assert!(matches!(
        import_bytes("large.md", "md", &vec![b'a'; MAX_FILE_BYTES + 1]),
        Err(BellbirdError::FileTooLarge)
    ));
}

#[test]
fn structured_html_is_converted_into_the_existing_document_model() {
    let imported = import_bytes(
        "source.html",
        "html",
        b"<!doctype html><html><body><h1>Relatorio</h1><p>Texto &amp; conteudo.</p></body></html>",
    )
    .unwrap();

    let ImportedSource::Document(document) = imported.source else {
        panic!("HTML should produce a document");
    };
    let latex = render_latex(&document, LatexTemplate::Standard);
    assert!(latex.contains("\\section{Relatorio}"), "{latex}");
    assert!(latex.contains("Texto \\& conteudo."));
}

#[test]
fn supported_extension_aliases_are_case_insensitive() {
    let markdown = import_bytes("notes.MARKDOWN", "MARKDOWN", b"# Markdown alias").unwrap();
    let ImportedSource::Document(document) = markdown.source else {
        panic!("Markdown should produce a document");
    };
    assert!(render_latex(&document, LatexTemplate::Standard).contains("Markdown alias"));

    let html = import_bytes(
        "page.HTM",
        "HTM",
        b"<html><body><h1>HTML alias</h1></body></html>",
    )
    .unwrap();
    let ImportedSource::Document(document) = html.source else {
        panic!("HTML should produce a document");
    };
    assert!(render_latex(&document, LatexTemplate::Standard).contains("HTML alias"));

    assert!(bellbird::conversion::path_is_supported(
        std::path::Path::new("report.MD")
    ));
    assert!(!bellbird::conversion::path_is_supported(
        std::path::Path::new("report.pdf")
    ));
}

#[test]
fn structured_html_reports_images_that_are_not_embedded_in_the_output() {
    let imported = import_bytes(
        "source.html",
        "html",
        b"<html><body><p>Figura:</p><img src=\"figura.png\" alt=\"grafico\"></body></html>",
    )
    .unwrap();
    let ImportedSource::Document(document) = imported.source else {
        panic!("HTML should produce a document");
    };

    assert!(!document.resources.is_empty());
    let latex = render_latex(&document, LatexTemplate::Standard);
    assert!(!latex.contains("<!-- image -->"));
    assert!(!latex.contains("figura.png"));
}

#[test]
fn restructured_text_preserves_common_document_structure() {
    let imported = import_bytes(
        "guide.rst",
        "rst",
        "Guia\n====\n\nUm parágrafo com *ênfase*, **destaque** e ``código``.\n\n- primeiro item\n- segundo item\n".as_bytes(),
    )
    .unwrap();
    let ImportedSource::Document(document) = imported.source else {
        panic!("reStructuredText should produce a document");
    };

    assert!(
        document
            .blocks
            .iter()
            .any(|block| matches!(block, Block::Heading { .. }))
    );
    let latex = render_latex(&document, LatexTemplate::Standard);
    assert!(latex.contains("Guia"));
    assert!(latex.contains("\\emph{ênfase}"));
    assert!(latex.contains("\\textbf{destaque}"));
    assert!(latex.contains("\\texttt{código}"));
    assert!(latex.contains("segundo item"));
}

#[test]
fn org_mode_preserves_common_document_structure() {
    let imported = import_bytes(
        "notes.org",
        "org",
        "* Título\n\nUm parágrafo com /ênfase/, *destaque* e =código=.\n\n** Seção\n\n- primeiro item\n- segundo item\n".as_bytes(),
    )
    .unwrap();
    let ImportedSource::Document(document) = imported.source else {
        panic!("Org should produce a document");
    };

    assert!(
        document
            .blocks
            .iter()
            .any(|block| matches!(block, Block::Heading { .. }))
    );
    let latex = render_latex(&document, LatexTemplate::Standard);
    assert!(latex.contains("Título"));
    assert!(latex.contains("\\emph{ênfase}"));
    assert!(latex.contains("\\textbf{destaque}"));
    assert!(latex.contains("\\texttt{código}"));
    assert!(latex.contains("segundo item"));
}

#[test]
fn docx_odt_and_epub_use_the_structured_converter() {
    let docx = zip_bytes(&[
        (
            "[Content_Types].xml",
            br#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>"#,
        ),
        (
            "_rels/.rels",
            br#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#,
        ),
        (
            "word/document.xml",
            br#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr><w:r><w:t>DOCX title</w:t></w:r></w:p><w:p><w:r><w:t>DOCX body</w:t></w:r></w:p></w:body></w:document>"#,
        ),
    ]);
    let odt = zip_bytes(&[
        (
            "mimetype",
            b"application/vnd.oasis.opendocument.text",
        ),
        (
            "content.xml",
            br#"<office:document-content xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"><office:body><office:text><text:h text:outline-level="1">ODT title</text:h><text:p>ODT body</text:p></office:text></office:body></office:document-content>"#,
        ),
    ]);
    let epub = zip_bytes(&[
        ("mimetype", b"application/epub+zip"),
        (
            "META-INF/container.xml",
            br#"<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container"><rootfiles><rootfile full-path="OEBPS/package.opf" media-type="application/oebps-package+xml"/></rootfiles></container>"#,
        ),
        (
            "OEBPS/package.opf",
            br#"<package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="id"><metadata xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:identifier id="id">book</dc:identifier><dc:title>EPUB title</dc:title><dc:language>en</dc:language></metadata><manifest><item id="chapter" href="chapter.xhtml" media-type="application/xhtml+xml"/></manifest><spine><itemref idref="chapter"/></spine></package>"#,
        ),
        (
            "OEBPS/chapter.xhtml",
            br#"<html xmlns="http://www.w3.org/1999/xhtml"><body><h1>EPUB heading</h1><p>EPUB body</p></body></html>"#,
        ),
    ]);

    for (name, extension, bytes, heading, text) in [
        ("document.docx", "docx", docx, "DOCX title", "DOCX body"),
        ("document.odt", "odt", odt, "ODT title", "ODT body"),
        ("document.epub", "epub", epub, "EPUB heading", "EPUB body"),
    ] {
        let imported = import_bytes(name, extension, &bytes).unwrap();
        let ImportedSource::Document(document) = imported.source else {
            panic!("{extension} should produce a document");
        };
        assert!(
            document
                .blocks
                .iter()
                .any(|block| matches!(block, Block::Heading { .. })),
            "{extension} heading structure was lost"
        );
        let latex = render_latex(&document, LatexTemplate::Standard);
        assert!(latex.contains(heading), "{extension} output: {latex}");
        assert!(latex.contains(text), "{extension} output: {latex}");
    }
}

fn zip_bytes(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut archive = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, content) in entries {
        let options = if *name == "mimetype" {
            zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored)
        } else {
            zip::write::SimpleFileOptions::default()
        };
        archive.start_file(*name, options).unwrap();
        archive.write_all(content).unwrap();
    }
    archive.finish().unwrap().into_inner()
}

#[test]
fn markdown_tables_and_math_select_only_needed_packages() {
    let document = parse_markdown("| A | B |\n| --- | --- |\n| 1 | $x^2$ |\n");
    let latex = render_latex(&document, LatexTemplate::Standard);

    assert!(latex.contains("\\usepackage{amsmath}"));
    assert!(latex.contains("\\begin{tabular}"));
    assert!(latex.contains("\\(x^2\\)"));
    assert!(!latex.contains("\\usepackage{graphicx}"));
}

#[test]
fn table_cell_line_break_does_not_end_the_row() {
    let document = Document {
        blocks: vec![Block::Table {
            headers: vec![
                vec![Inline::Text("Descrição".into())],
                vec![Inline::Text("Valor".into())],
            ],
            rows: vec![vec![
                vec![
                    Inline::Text("primeira linha".into()),
                    Inline::LineBreak,
                    Inline::Text("segunda linha".into()),
                    Inline::Text(" ".into()),
                    Inline::Strong(vec![
                        Inline::Text("destaque".into()),
                        Inline::LineBreak,
                        Inline::Text("em duas linhas".into()),
                    ]),
                ],
                vec![Inline::Text("conteúdo".into())],
            ]],
        }],
        ..Document::default()
    };

    let latex = render_latex(&document, LatexTemplate::Standard);

    assert!(
        latex.contains(
            r"primeira linha\newline{}segunda linha \textbf{destaque\newline{}em duas linhas} & conteúdo\\"
        ),
        "{latex}"
    );
    assert!(bellbird::document::validate_generated_latex(&latex).is_ok());
}

#[test]
fn strikethrough_keeps_emphasis_semantics_and_hyperref_loads_last() {
    let document = parse_markdown("[link](https://example.org) and ~~removed~~\n");
    let latex = render_latex(&document, LatexTemplate::Standard);

    assert!(latex.contains("\\usepackage[normalem]{ulem}"));
    assert!(
        latex.find("\\usepackage[normalem]{ulem}").unwrap()
            < latex.find("\\usepackage{hyperref}").unwrap()
    );
}

#[test]
fn markdown_footnotes_keep_their_numbers_and_repeated_references() {
    let document = parse_markdown(
        "Primeira[^first], repetida[^first], segunda[^second].\n\n[^first]: Nota primeira.\n[^second]: Nota segunda.\n",
    );
    let latex = render_latex(&document, LatexTemplate::Standard);

    assert_eq!(latex.matches(r"\footnotemark[1]").count(), 2);
    assert_eq!(latex.matches(r"\footnotemark[2]").count(), 1);
    assert!(latex.contains(r"\footnotetext[1]{Nota primeira."));
    assert!(latex.contains(r"\footnotetext[2]{Nota segunda."));
}

#[test]
fn image_paths_are_not_referenced_in_generated_latex() {
    let document = Document {
        blocks: vec![Block::Paragraph(vec![Inline::Image {
            source: "images/{logo}.png".into(),
            alt: vec![Inline::Text("logo".into())],
        }])],
        ..Document::default()
    };
    let latex = render_latex(&document, LatexTemplate::Standard);

    assert!(latex.contains("logo"));
    assert!(!latex.contains("images/{logo}.png"));
    assert!(!latex.contains(r"\includegraphics"));
}

#[test]
fn image_paths_with_percent_are_omitted_from_generated_latex() {
    let document = Document {
        blocks: vec![Block::Paragraph(vec![Inline::Image {
            source: "images/50% complete.png".into(),
            alt: vec![Inline::Text("chart".into())],
        }])],
        ..Document::default()
    };
    let latex = render_latex(&document, LatexTemplate::Standard);

    assert!(latex.contains("chart"));
    assert!(!latex.contains("images/50% complete.png"));
    assert!(!latex.contains(r"\includegraphics"));
}
