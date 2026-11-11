use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use adw::prelude::*;
use bellbird::conversion::{self, ImportedFile, ImportedSource, SUPPORTED_EXTENSIONS};
use bellbird::document::{
    LatexTemplate, render_latex, used_safe_fallback, validate_generated_latex,
};
use bellbird::i18n::tr;
use gtk::{gio, glib};

use super::result;
use super::state::Views;

pub(super) fn open_file_chooser(views: &Views) {
    let Some(window) = views.window.upgrade() else {
        return;
    };
    let dialog = gtk::FileDialog::new();
    dialog.set_title(&tr("Choose a document"));
    dialog.set_accept_label(Some(&tr("Open")));
    dialog.set_modal(true);
    let filter = gtk::FileFilter::new();
    filter.set_name(Some(&tr("Supported documents")));
    for extension in SUPPORTED_EXTENSIONS {
        filter.add_suffix(extension);
    }
    let filters = gio::ListStore::new::<gtk::FileFilter>();
    filters.append(&filter);
    dialog.set_filters(Some(&filters));
    dialog.set_default_filter(Some(&filter));
    let dialog_views = views.clone();
    dialog.open(
        Some(&window),
        None::<&gio::Cancellable>,
        move |response| match response {
            Ok(file) => {
                if let Some(path) = file.path() {
                    begin_import(path, &dialog_views);
                } else {
                    result::show_toast(
                        &dialog_views,
                        &tr("The selected file is not available as a local path."),
                    );
                }
            }
            Err(error) if error.matches(gio::IOErrorEnum::Cancelled) => {}
            Err(_) => result::show_toast(&dialog_views, &tr("Could not open the file chooser.")),
        },
    );
}

pub(super) fn begin_import(path: std::path::PathBuf, views: &Views) {
    let generation = {
        let mut state = views.state.borrow_mut();
        state.import_generation = state.import_generation.wrapping_add(1);
        state.imported = None;
        state.output.clear();
        state.import_generation
    };
    if let Some(open_button) = views.open_button.upgrade() {
        open_button.set_visible(true);
        open_button.set_sensitive(false);
    }
    if let Some(menu_button) = views.menu_button.upgrade() {
        menu_button.set_visible(false);
    }
    if let Some(copy_button) = views.copy_button.upgrade() {
        copy_button.set_sensitive(false);
    }
    if let Some(export_button) = views.export_button.upgrade() {
        export_button.set_sensitive(false);
    }
    if let Some(template) = views.template.upgrade() {
        template.set_sensitive(result::template_selector_enabled(None));
    }
    if let Some(file_name) = views.file_name.upgrade() {
        file_name.set_text(
            path.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default(),
        );
    }
    if let Some(buffer) = views.code_buffer.upgrade() {
        buffer.set_text("");
    }
    if let Some(buffer) = views.line_number_buffer.upgrade() {
        buffer.set_text("");
    }
    if let Some(status) = views.status.upgrade() {
        status.set_text(&tr("Processing document…"));
    }
    if let Some(resource_notice) = views.resource_notice.upgrade() {
        resource_notice.set_visible(false);
    }
    if let Some(spinner) = views.spinner.upgrade() {
        spinner.set_visible(true);
        spinner.start();
    }
    if let Some(stack) = views.stack.upgrade() {
        stack.set_visible_child_name("result");
    }

    let (sender, receiver) = mpsc::channel();
    let _worker = thread::spawn(move || {
        let _ = sender.send(conversion::import_file(&path));
    });
    let task_views = views.clone();
    glib::timeout_add_local(Duration::from_millis(40), move || {
        if task_views.state.borrow().import_generation != generation {
            return glib::ControlFlow::Break;
        }
        match receiver.try_recv() {
            Ok(Ok(imported)) => {
                if let Some(spinner) = task_views.spinner.upgrade() {
                    spinner.stop();
                    spinner.set_visible(false);
                }
                if let Some(status) = task_views.status.upgrade() {
                    status.set_text("");
                }
                if let Some(template) = task_views.template.upgrade() {
                    template
                        .set_sensitive(result::template_selector_enabled(Some(&imported.source)));
                }
                task_views.state.borrow_mut().imported = Some(imported.clone());
                display_imported(
                    &task_views,
                    imported,
                    result::selected_template(&task_views),
                );
                glib::ControlFlow::Break
            }
            Ok(Err(error)) => {
                if let Some(spinner) = task_views.spinner.upgrade() {
                    spinner.stop();
                    spinner.set_visible(false);
                }
                task_views.state.borrow_mut().imported = None;
                if let Some(stack) = task_views.stack.upgrade() {
                    stack.set_visible_child_name("import");
                }
                if let Some(open_button) = task_views.open_button.upgrade() {
                    open_button.set_visible(false);
                    open_button.set_sensitive(true);
                }
                if let Some(menu_button) = task_views.menu_button.upgrade() {
                    menu_button.set_visible(true);
                }
                result::show_toast(&task_views, &error.user_message());
                glib::ControlFlow::Break
            }
            Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
            Err(mpsc::TryRecvError::Disconnected) => {
                if let Some(spinner) = task_views.spinner.upgrade() {
                    spinner.stop();
                    spinner.set_visible(false);
                }
                if let Some(stack) = task_views.stack.upgrade() {
                    stack.set_visible_child_name("import");
                }
                if let Some(open_button) = task_views.open_button.upgrade() {
                    open_button.set_visible(false);
                    open_button.set_sensitive(true);
                }
                if let Some(menu_button) = task_views.menu_button.upgrade() {
                    menu_button.set_visible(true);
                }
                result::show_toast(&task_views, &tr("Conversion failed."));
                glib::ControlFlow::Break
            }
        }
    });
}

pub(super) fn display_imported(views: &Views, imported: ImportedFile, template: LatexTemplate) {
    let mut validation_issue = None;
    let mut safe_fallback_used = false;
    let mut simplified_conversion = false;
    let output = match &imported.source {
        ImportedSource::Document(document) => {
            let output = render_latex(document, template);
            safe_fallback_used = used_safe_fallback(&output);
            simplified_conversion = document.has_simplified_content;
            validation_issue = validate_generated_latex(&output).err();
            output
        }
        ImportedSource::ExistingLatex(source) => source.clone(),
    };
    if let Some(file_name) = views.file_name.upgrade() {
        file_name.set_text(&imported.display_name);
    }
    if let Some(buffer) = views.code_buffer.upgrade() {
        buffer.set_text(&output);
    }
    if let Some(buffer) = views.line_number_buffer.upgrade() {
        buffer.set_text(&result::line_number_text(&output));
    }
    if let Some(copy_button) = views.copy_button.upgrade() {
        copy_button.set_sensitive(true);
    }
    if let Some(export_button) = views.export_button.upgrade() {
        export_button.set_sensitive(true);
    }
    if let Some(open_button) = views.open_button.upgrade() {
        open_button.set_visible(true);
        open_button.set_sensitive(true);
    }
    if let Some(menu_button) = views.menu_button.upgrade() {
        menu_button.set_visible(true);
    }
    if let Some(resource_notice) = views.resource_notice.upgrade() {
        resource_notice.set_visible(matches!(&imported.source, ImportedSource::Document(document) if !document.resources.is_empty()));
    }
    if let Some(stack) = views.stack.upgrade() {
        stack.set_visible_child_name("result");
    }
    views.state.borrow_mut().output = output;
    if let Some(issue) = validation_issue {
        eprintln!("Generated LaTeX needs review: {issue}");
        result::show_toast(
            views,
            &tr("Generated LaTeX has structural or citation issues. Review it before exporting."),
        );
    } else if safe_fallback_used || simplified_conversion {
        result::show_toast(
            views,
            &tr("Some content was converted to plain text to preserve valid LaTeX."),
        );
    }
}
