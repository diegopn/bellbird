use std::env;
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};

use bellbird::conversion::{self, ImportedFile, ImportedSource};
use bellbird::document::{LatexTemplate, render_latex};

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let input_path = args
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| "usage: memory_probe <input-path> [--hold-seconds N]".to_owned())?;
    let mut hold_seconds = 5_u64;
    if let Some(option) = args.next() {
        if option != "--hold-seconds" {
            return Err(format!("unexpected argument: {option}"));
        }
        hold_seconds = args
            .next()
            .ok_or_else(|| "--hold-seconds requires a value".to_owned())?
            .parse()
            .map_err(|_| "--hold-seconds must be a nonnegative integer".to_owned())?;
    }
    if let Some(argument) = args.next() {
        return Err(format!("unexpected argument: {argument}"));
    }

    let import_started = Instant::now();
    let imported = thread::spawn(move || conversion::import_file(&input_path))
        .join()
        .map_err(|_| "document import worker panicked".to_owned())?
        .map_err(|error| error.to_string())?;
    let import_ms = import_started.elapsed().as_secs_f64() * 1_000.0;

    let render_started = Instant::now();
    let initial_output = render_imported(&imported, LatexTemplate::Abnt);
    let output = render_imported(&imported, LatexTemplate::Ieee);
    let render_ms = render_started.elapsed().as_secs_f64() * 1_000.0;
    drop(initial_output);

    println!(
        "import_ms={import_ms:.3} render_ms={render_ms:.3} output_bytes={}",
        output.len()
    );

    // Keep the same data alive as the result screen while RSS is sampled.
    let _retained_import = imported;
    let _retained_output = output;
    thread::sleep(Duration::from_secs(hold_seconds));
    Ok(())
}

fn render_imported(imported: &ImportedFile, template: LatexTemplate) -> String {
    match &imported.source {
        ImportedSource::Document(document) => render_latex(document, template),
        ImportedSource::ExistingLatex(source) => source.clone(),
    }
}
