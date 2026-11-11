use super::archive::validate_archive_limits;
use super::structured::convert_docling_node;
use super::*;
use crate::document::{Block, Inline};
use crate::error::BellbirdError;
use std::io::{Cursor, Write};
use std::path::Path;

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
