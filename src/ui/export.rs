use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

use adw::prelude::*;
use bellbird::i18n::tr;
use gtk::{gio, glib};

use super::result;
use super::state::Views;

pub(super) fn export_latex_with_chooser(views: &Views) {
    let Some(window) = views.window.upgrade() else {
        return;
    };
    let (source_name, output) = {
        let state = views.state.borrow();
        let Some(imported) = state.imported.as_ref() else {
            return;
        };
        (imported.display_name.clone(), state.output.clone())
    };

    let mut output_name = PathBuf::from(source_name);
    output_name.set_extension("tex");
    let initial_name = output_name
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "document.tex".to_owned());

    let dialog = gtk::FileDialog::new();
    dialog.set_title(&tr("Export LaTeX"));
    dialog.set_accept_label(Some(&tr("Save")));
    dialog.set_modal(true);
    dialog.set_initial_name(Some(&initial_name));
    if let Some(downloads_dir) =
        glib::user_special_dir(glib::UserDirectory::Downloads).filter(|path| path.is_dir())
    {
        let downloads_folder = gio::File::for_path(downloads_dir);
        dialog.set_initial_folder(Some(&downloads_folder));
    }

    let dialog_views = views.clone();
    dialog.save(
        Some(&window),
        None::<&gio::Cancellable>,
        move |response| match response {
            Ok(file) => {
                let Some(path) = file.path() else {
                    result::show_toast(
                        &dialog_views,
                        &tr("The selected file is not available as a local path."),
                    );
                    return;
                };
                let Some(directory) = path.parent() else {
                    result::show_toast(&dialog_views, &tr("Could not export LaTeX."));
                    return;
                };
                let Some(file_name) = path.file_name() else {
                    result::show_toast(&dialog_views, &tr("Could not export LaTeX."));
                    return;
                };
                match save_without_overwriting(directory, Path::new(file_name), output.as_bytes()) {
                    Ok(()) => result::show_toast(&dialog_views, &tr("LaTeX exported successfully")),
                    Err(error) => {
                        eprintln!("Could not export LaTeX: {error}");
                        result::show_toast(&dialog_views, &tr("Could not export LaTeX."));
                    }
                }
            }
            Err(error) if error.matches(gio::IOErrorEnum::Cancelled) => {}
            Err(error) => {
                eprintln!("Could not open export dialog: {error}");
                result::show_toast(&dialog_views, &tr("Could not export LaTeX."));
            }
        },
    );
}

fn save_without_overwriting(
    directory: &Path,
    requested_name: &Path,
    contents: &[u8],
) -> std::io::Result<()> {
    let stem = requested_name
        .file_stem()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("bellbird");
    let mut suffix = 0u64;
    loop {
        let file_name = if suffix == 0 {
            format!("{stem}.tex")
        } else {
            format!("{stem} ({suffix}).tex")
        };
        let path = directory.join(file_name);
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut file) => {
                if let Err(error) = file.write_all(contents) {
                    drop(file);
                    let _ = fs::remove_file(&path);
                    return Err(error);
                }
                return Ok(());
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                suffix = suffix.checked_add(1).ok_or(error)?;
            }
            Err(error) => return Err(error),
        }
    }
}
