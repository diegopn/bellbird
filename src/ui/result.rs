use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use adw::prelude::*;
use bellbird::conversion::ImportedSource;
use bellbird::document::LatexTemplate;
use bellbird::i18n::tr;
use gtk::glib;

use super::state::Views;

pub(super) struct ResultWidgets {
    pub(super) file_name: gtk::Label,
    pub(super) template: gtk::DropDown,
    pub(super) spinner: gtk::Spinner,
    pub(super) status: gtk::Label,
    pub(super) resource_notice: gtk::Label,
    pub(super) code_buffer: gtk::TextBuffer,
    pub(super) line_number_buffer: gtk::TextBuffer,
    pub(super) copy_button: gtk::Button,
    pub(super) export_button: gtk::Button,
}

pub(super) fn build_result() -> (gtk::Widget, ResultWidgets) {
    let page = gtk::Box::new(gtk::Orientation::Vertical, 12);
    page.set_margin_top(18);
    page.set_margin_bottom(18);
    page.set_margin_start(18);
    page.set_margin_end(18);
    page.set_vexpand(true);

    let details = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    let file_name = gtk::Label::new(None);
    file_name.set_hexpand(true);
    file_name.set_halign(gtk::Align::Start);
    file_name.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
    file_name.set_selectable(true);
    details.append(&file_name);
    let template = build_template_selector();
    let template_accessible = tr("LaTeX style");
    template.update_property(&[gtk::accessible::Property::Label(&template_accessible)]);
    details.append(&template);
    page.append(&details);

    let progress = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let spinner = gtk::Spinner::new();
    spinner.set_visible(false);
    let status = gtk::Label::new(None);
    status.set_halign(gtk::Align::Start);
    progress.append(&spinner);
    progress.append(&status);
    page.append(&progress);

    let resource_notice = gtk::Label::new(Some(&tr(
        "Images and attached resources are omitted from LaTeX output; image descriptions are kept when available.",
    )));
    resource_notice.set_wrap(true);
    resource_notice.set_halign(gtk::Align::Start);
    resource_notice.add_css_class("warning");
    resource_notice.set_visible(false);
    page.append(&resource_notice);

    let code_buffer = gtk::TextBuffer::new(None);
    let line_number_buffer = gtk::TextBuffer::new(None);
    let code_view = gtk::TextView::new();
    code_view.set_buffer(Some(&code_buffer));
    code_view.set_editable(false);
    code_view.set_cursor_visible(true);
    code_view.set_monospace(true);
    code_view.set_wrap_mode(gtk::WrapMode::WordChar);
    code_view.set_hexpand(true);
    code_view.set_vexpand(true);
    code_view.set_halign(gtk::Align::Fill);
    code_view.add_css_class("latex-output");
    code_view.set_left_margin(15);
    code_view.set_right_margin(15);
    code_view.set_top_margin(15);
    code_view.set_bottom_margin(15);
    let output_accessible = tr("LaTeX output");
    code_view.update_property(&[
        gtk::accessible::Property::Label(&output_accessible),
        gtk::accessible::Property::ReadOnly(true),
    ]);
    let line_number_view = gtk::TextView::new();
    line_number_view.set_buffer(Some(&line_number_buffer));
    line_number_view.set_editable(false);
    line_number_view.set_cursor_visible(false);
    line_number_view.set_monospace(true);
    line_number_view.set_wrap_mode(gtk::WrapMode::None);
    line_number_view.set_size_request(52, -1);
    line_number_view.set_hexpand(false);
    line_number_view.set_vexpand(true);
    line_number_view.set_justification(gtk::Justification::Right);
    line_number_view.set_left_margin(12);
    line_number_view.set_right_margin(10);
    line_number_view.set_top_margin(15);
    line_number_view.set_bottom_margin(15);
    line_number_view.set_can_focus(false);
    line_number_view.add_css_class("line-number-gutter");
    let line_numbers_accessible = tr("Line numbers");
    line_number_view.update_property(&[
        gtk::accessible::Property::Label(&line_numbers_accessible),
        gtk::accessible::Property::ReadOnly(true),
    ]);
    let editor = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    editor.set_hexpand(true);
    editor.set_vexpand(true);
    editor.append(&line_number_view);
    editor.append(&code_view);
    let scroller = gtk::ScrolledWindow::new();
    scroller.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    scroller.set_hexpand(true);
    scroller.set_vexpand(true);
    scroller.set_child(Some(&editor));
    let output_frame = gtk::Frame::new(None);
    output_frame.add_css_class("latex-output-frame");
    output_frame.set_child(Some(&scroller));
    output_frame.set_hexpand(true);
    output_frame.set_vexpand(true);
    page.append(&output_frame);

    let view_for_buffer_change = code_view.clone();
    let gutter_for_buffer_change = line_number_buffer.clone();
    code_buffer.connect_changed(move |_| {
        let view = view_for_buffer_change.clone();
        let gutter = gutter_for_buffer_change.clone();
        glib::idle_add_local_once(move || update_line_number_gutter(&view, &gutter));
    });
    let view_for_resize = code_view.clone();
    let gutter_for_resize = line_number_buffer.clone();
    let last_width = Rc::new(Cell::new(0));
    let last_width_for_resize = last_width.clone();
    glib::timeout_add_local(Duration::from_millis(250), move || {
        let width = view_for_resize.width();
        if width > 0 && width != last_width_for_resize.replace(width) {
            update_line_number_gutter(&view_for_resize, &gutter_for_resize);
        }
        glib::ControlFlow::Continue
    });

    let footer = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    footer.set_halign(gtk::Align::End);
    let export_button = gtk::Button::with_label(&tr("Export LaTeX"));
    export_button.set_sensitive(false);
    footer.append(&export_button);
    let copy_button = gtk::Button::with_label(&tr("Copy LaTeX"));
    copy_button.add_css_class("suggested-action");
    copy_button.set_sensitive(false);
    footer.append(&copy_button);
    page.append(&footer);

    (
        page.upcast(),
        ResultWidgets {
            file_name,
            template,
            spinner,
            status,
            resource_notice,
            code_buffer,
            line_number_buffer,
            copy_button,
            export_button,
        },
    )
}

fn build_template_selector() -> gtk::DropDown {
    let labels = LatexTemplate::EXPORT_OPTIONS
        .iter()
        .map(|template| tr(template.label()))
        .collect::<Vec<_>>();
    let label_refs = labels.iter().map(String::as_str).collect::<Vec<_>>();
    let template = gtk::DropDown::from_strings(&label_refs);
    template.set_selected(0);
    template
}

pub(super) fn selected_template(views: &Views) -> LatexTemplate {
    views
        .template
        .upgrade()
        .map(|template| selected_template_from_index(template.selected()))
        .unwrap_or(LatexTemplate::Abnt)
}

pub(super) fn selected_template_from_index(index: u32) -> LatexTemplate {
    LatexTemplate::EXPORT_OPTIONS
        .get(index as usize)
        .copied()
        .unwrap_or(LatexTemplate::Abnt)
}

pub(super) fn line_number_text(content: &str) -> String {
    let count = content.lines().count().max(1);
    (1..=count)
        .map(|line| line.to_string())
        .collect::<Vec<_>>()
        .join("\n")
}

pub(super) fn line_number_text_for_wrapped_rows(wrapped_rows: &[usize]) -> String {
    let mut rows = Vec::new();
    for (index, visual_rows) in wrapped_rows.iter().enumerate() {
        rows.push((index + 1).to_string());
        rows.extend(std::iter::repeat_n(
            String::new(),
            visual_rows.saturating_sub(1),
        ));
    }
    rows.join("\n")
}

fn update_line_number_gutter(view: &gtk::TextView, gutter: &gtk::TextBuffer) {
    let buffer = view.buffer();
    let mut heights = Vec::new();
    for line in 0..buffer.line_count() {
        if let Some(iter) = buffer.iter_at_line(line) {
            let (_, height) = view.line_yrange(&iter);
            heights.push(height.max(1));
        }
    }
    if heights.is_empty() {
        gutter.set_text("1");
        return;
    }

    let line_height = *heights.iter().min().unwrap_or(&1);
    let wrapped_rows = heights
        .iter()
        .map(|height| ((*height + line_height - 1) / line_height).max(1) as usize)
        .collect::<Vec<_>>();
    let text = line_number_text_for_wrapped_rows(&wrapped_rows);
    if gutter.text(&gutter.start_iter(), &gutter.end_iter(), false) != text {
        gutter.set_text(&text);
    }
}

pub(super) fn template_selector_enabled(source: Option<&ImportedSource>) -> bool {
    !matches!(source, Some(ImportedSource::ExistingLatex(_)))
}

pub(super) fn show_toast(views: &Views, message: &str) {
    if let Some(overlay) = views.toast_overlay.upgrade() {
        overlay.add_toast(adw::Toast::new(message));
    }
}
