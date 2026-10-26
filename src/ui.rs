use std::cell::{Cell, RefCell};
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use adw::prelude::*;
use gtk::{gio, glib};

use bellbird::conversion::{self, ImportedFile, ImportedSource, SUPPORTED_EXTENSIONS};
use bellbird::document::{
    LatexTemplate, render_latex, used_safe_fallback, validate_generated_latex,
};
use bellbird::i18n::tr;

const APP_ID: &str = "io.github.diegopn.Bellbird";

#[derive(Clone)]
struct Views {
    window: glib::WeakRef<adw::ApplicationWindow>,
    toast_overlay: glib::WeakRef<adw::ToastOverlay>,
    stack: glib::WeakRef<gtk::Stack>,
    open_button: glib::WeakRef<gtk::Button>,
    menu_button: glib::WeakRef<gtk::MenuButton>,
    file_name: glib::WeakRef<gtk::Label>,
    template: glib::WeakRef<gtk::DropDown>,
    spinner: glib::WeakRef<gtk::Spinner>,
    status: glib::WeakRef<gtk::Label>,
    resource_notice: glib::WeakRef<gtk::Label>,
    code_buffer: glib::WeakRef<gtk::TextBuffer>,
    line_number_buffer: glib::WeakRef<gtk::TextBuffer>,
    copy_button: glib::WeakRef<gtk::Button>,
    export_button: glib::WeakRef<gtk::Button>,
    state: Rc<RefCell<UiState>>,
}

#[derive(Default)]
struct UiState {
    imported: Option<ImportedFile>,
    output: String,
    import_generation: u64,
}

pub fn run() -> glib::ExitCode {
    let application = adw::Application::builder().application_id(APP_ID).build();
    application.connect_activate(build_window);
    application.run()
}

fn build_window(application: &adw::Application) {
    install_styles();

    let window = adw::ApplicationWindow::builder()
        .application(application)
        .title("Bellbird")
        .default_width(760)
        .default_height(620)
        .build();
    window.add_css_class("bellbird-window");
    let state = Rc::new(RefCell::new(UiState::default()));
    let toast_overlay = adw::ToastOverlay::new();
    let shell = gtk::Box::new(gtk::Orientation::Vertical, 0);
    shell.add_css_class("bellbird-root");
    let header = adw::HeaderBar::new();
    header.add_css_class("bellbird-headerbar");
    header.set_title_widget(Some(&gtk::Label::new(Some("Bellbird"))));

    let open_button = gtk::Button::builder()
        .icon_name("document-open-symbolic")
        .tooltip_text(tr("Open another document"))
        .build();
    let open_accessible = tr("Open another document");
    open_button.update_property(&[gtk::accessible::Property::Label(&open_accessible)]);
    open_button.set_visible(false);
    header.pack_end(&open_button);

    let header_menu = build_header_menu(&window);
    let menu_button = header_menu.button.clone();
    menu_button.set_visible(false);
    header.pack_start(&menu_button);
    shell.append(&header);

    let stack = gtk::Stack::new();
    stack.set_hexpand(true);
    stack.set_vexpand(true);
    stack.set_transition_type(gtk::StackTransitionType::Crossfade);
    let (result_page, result) = build_result();
    stack.add_named(&result_page, Some("result"));

    let views = Views {
        window: window.downgrade(),
        toast_overlay: toast_overlay.downgrade(),
        stack: stack.downgrade(),
        open_button: open_button.downgrade(),
        menu_button: menu_button.downgrade(),
        file_name: result.file_name.downgrade(),
        template: result.template.downgrade(),
        spinner: result.spinner.downgrade(),
        status: result.status.downgrade(),
        resource_notice: result.resource_notice.downgrade(),
        code_buffer: result.code_buffer.downgrade(),
        line_number_buffer: result.line_number_buffer.downgrade(),
        copy_button: result.copy_button.downgrade(),
        export_button: result.export_button.downgrade(),
        state,
    };
    let landing = build_landing(&views);
    stack.add_named(&landing, Some("import"));
    stack.set_visible_child_name("import");
    shell.append(&stack);
    toast_overlay.set_child(Some(&shell));
    window.set_content(Some(&toast_overlay));

    let open_views = views.clone();
    open_button.connect_clicked(move |_| open_file_chooser(&open_views));

    let menu_open_views = views.clone();
    header_menu
        .open_document_button
        .connect_clicked(move |_| open_file_chooser(&menu_open_views));

    let template_views = views.clone();
    result.template.connect_selected_notify(move |_| {
        let imported = template_views.state.borrow().imported.clone();
        if let Some(imported) = imported {
            display_imported(
                &template_views,
                imported,
                selected_template(&template_views),
            );
        }
    });

    let copy_views = views.clone();
    result.copy_button.connect_clicked(move |_| {
        if let Some(display) = gtk::gdk::Display::default() {
            let output = copy_views.state.borrow().output.clone();
            display.clipboard().set_text(&output);
            show_toast(&copy_views, &tr("Copied to clipboard"));
        }
    });

    let export_views = views.clone();
    result
        .export_button
        .connect_clicked(move |_| export_latex_with_chooser(&export_views));

    window.present();
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Appearance {
    Automatic,
    Light,
    Dark,
}

impl Appearance {
    const ALL: [Self; 3] = [Self::Automatic, Self::Light, Self::Dark];

    fn key(self) -> &'static str {
        match self {
            Self::Automatic => "automatic",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    fn from_key(key: &str) -> Option<Self> {
        match key {
            "automatic" => Some(Self::Automatic),
            "light" => Some(Self::Light),
            "dark" => Some(Self::Dark),
            _ => None,
        }
    }

    fn color_scheme(self) -> adw::ColorScheme {
        match self {
            Self::Automatic => adw::ColorScheme::Default,
            Self::Light => adw::ColorScheme::ForceLight,
            Self::Dark => adw::ColorScheme::ForceDark,
        }
    }

    fn label(self) -> String {
        tr(match self {
            Self::Automatic => "Automatic",
            Self::Light => "Light",
            Self::Dark => "Dark",
        })
    }
}

struct HeaderMenu {
    button: gtk::MenuButton,
    open_document_button: gtk::Button,
}

fn build_header_menu(window: &adw::ApplicationWindow) -> HeaderMenu {
    let appearance = load_appearance();
    let style_manager = adw::StyleManager::default();
    style_manager.set_color_scheme(appearance.color_scheme());

    let popover = gtk::Popover::new();
    let content = gtk::Box::new(gtk::Orientation::Vertical, 8);
    content.add_css_class("bellbird-menu-content");
    let appearance_row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    appearance_row.set_halign(gtk::Align::Center);
    let indicators = Rc::new(RefCell::new(Vec::<(Appearance, gtk::Label)>::new()));

    for option in Appearance::ALL {
        let swatch = gtk::DrawingArea::new();
        swatch.set_content_width(34);
        swatch.set_content_height(34);
        swatch.set_draw_func(move |_, context, width, height| {
            draw_appearance_swatch(context, width, height, option);
        });

        let check = gtk::Label::new(Some("✓"));
        check.add_css_class("appearance-check");
        check.set_halign(gtk::Align::End);
        check.set_valign(gtk::Align::End);
        check.set_visible(option == appearance);

        let swatch_overlay = gtk::Overlay::new();
        swatch_overlay.set_child(Some(&swatch));
        swatch_overlay.add_overlay(&check);

        let button = gtk::Button::new();
        button.add_css_class("flat");
        button.add_css_class("appearance-choice");
        let label = option.label();
        button.set_tooltip_text(Some(&label));
        button.update_property(&[gtk::accessible::Property::Label(&label)]);
        button.set_child(Some(&swatch_overlay));
        appearance_row.append(&button);
        indicators.borrow_mut().push((option, check.clone()));

        let selected_indicators = indicators.clone();
        let selected_style_manager = style_manager.clone();
        button.connect_clicked(move |_| {
            selected_style_manager.set_color_scheme(option.color_scheme());
            save_appearance(option);
            for (candidate, indicator) in selected_indicators.borrow().iter() {
                indicator.set_visible(*candidate == option);
            }
        });
    }
    content.append(&appearance_row);
    content.append(&gtk::Separator::new(gtk::Orientation::Horizontal));

    let open_document_button = gtk::Button::new();
    open_document_button.set_has_frame(false);
    open_document_button.add_css_class("bellbird-menu-item");
    open_document_button.set_child(Some(&menu_action_content(
        "document-open-symbolic",
        &tr("Open another document"),
    )));
    let open_popover = popover.clone();
    open_document_button.connect_clicked(move |_| open_popover.popdown());
    content.append(&open_document_button);

    let about_button = gtk::Button::new();
    about_button.set_has_frame(false);
    about_button.add_css_class("bellbird-menu-item");
    about_button.set_child(Some(&menu_action_content(
        "help-about-symbolic",
        &tr("About Bellbird"),
    )));
    let about_parent = window.clone();
    let about_popover = popover.clone();
    about_button.connect_clicked(move |_| {
        about_popover.popdown();
        show_about_dialog(&about_parent);
    });
    content.append(&about_button);

    popover.set_child(Some(&content));
    let menu_button = gtk::MenuButton::new();
    menu_button.set_icon_name("open-menu-symbolic");
    menu_button.set_popover(Some(&popover));
    let accessible_label = tr("Application menu");
    menu_button.set_tooltip_text(Some(&accessible_label));
    menu_button.update_property(&[gtk::accessible::Property::Label(&accessible_label)]);
    HeaderMenu {
        button: menu_button,
        open_document_button,
    }
}

fn draw_appearance_swatch(
    context: &gtk::cairo::Context,
    width: i32,
    height: i32,
    appearance: Appearance,
) {
    let width = f64::from(width);
    let height = f64::from(height);
    let center_x = width / 2.0;
    let center_y = height / 2.0;
    let radius = center_x.min(center_y) - 1.5;

    let _ = context.save();
    context.arc(center_x, center_y, radius, 0.0, std::f64::consts::TAU);
    context.clip();
    match appearance {
        Appearance::Automatic => {
            context.set_source_rgb(0.96, 0.96, 0.96);
            let _ = context.paint();
            context.move_to(0.0, 0.0);
            context.line_to(width, 0.0);
            context.line_to(0.0, height);
            context.close_path();
            context.set_source_rgb(0.12, 0.12, 0.13);
            let _ = context.fill();
        }
        Appearance::Light => {
            context.set_source_rgb(0.96, 0.96, 0.96);
            let _ = context.paint();
        }
        Appearance::Dark => {
            context.set_source_rgb(0.12, 0.12, 0.13);
            let _ = context.paint();
        }
    }
    let _ = context.restore();
    context.arc(center_x, center_y, radius, 0.0, std::f64::consts::TAU);
    context.set_source_rgba(0.55, 0.57, 0.60, 0.8);
    context.set_line_width(1.0);
    let _ = context.stroke();
}

fn menu_action_content(icon_name: &str, title: &str) -> gtk::Widget {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    row.set_hexpand(true);
    let icon = gtk::Image::from_icon_name(icon_name);
    icon.set_pixel_size(18);
    let label = gtk::Label::new(Some(title));
    label.set_halign(gtk::Align::Start);
    label.set_hexpand(true);
    row.append(&icon);
    row.append(&label);
    row.upcast()
}

fn show_about_dialog(parent: &adw::ApplicationWindow) {
    let content = gtk::Box::new(gtk::Orientation::Vertical, 14);
    content.add_css_class("bellbird-about-content");
    content.set_hexpand(true);
    content.set_margin_top(16);
    content.set_margin_bottom(20);
    content.set_margin_start(20);
    content.set_margin_end(20);

    let close_row = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    close_row.set_halign(gtk::Align::End);
    let close_button = gtk::Button::from_icon_name("window-close-symbolic");
    close_button.add_css_class("circular");
    close_button.set_tooltip_text(Some(&tr("Close")));
    close_row.append(&close_button);
    content.append(&close_row);

    let icon = gtk::Image::from_icon_name(APP_ID);
    icon.set_pixel_size(120);
    icon.set_halign(gtk::Align::Center);
    content.append(&icon);

    let name = gtk::Label::new(Some("Bellbird"));
    name.add_css_class("title-1");
    name.set_halign(gtk::Align::Center);
    content.append(&name);

    let creator = gtk::Label::new(Some(&tr("Created by Diego Pereira do Nascimento")));
    creator.set_halign(gtk::Align::Center);
    creator.set_wrap(true);
    content.append(&creator);

    let version = gtk::Label::new(Some(env!("CARGO_PKG_VERSION")));
    version.add_css_class("about-version");
    version.set_halign(gtk::Align::Center);
    content.append(&version);

    let dedication = gtk::Label::new(Some(&tr(
        "Dedicated to my wife Karina, who always encourages me to go further.",
    )));
    dedication.set_halign(gtk::Align::Center);
    dedication.set_justify(gtk::Justification::Center);
    dedication.set_wrap(true);
    content.append(&dedication);

    let issue_link = about_link(
        "https://github.com/diegopn/bellbird/issues",
        &tr("Report an issue"),
    );
    issue_link.add_css_class("about-link-card");
    content.append(&issue_link);

    let project_links = gtk::Box::new(gtk::Orientation::Vertical, 0);
    project_links.add_css_class("about-link-group");
    project_links.set_hexpand(true);
    project_links.append(&about_link(
        "https://github.com/diegopn/bellbird",
        &tr("Project repository"),
    ));
    project_links.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    project_links.append(&about_link(
        "https://diegopn.github.io/bellbird-site/",
        &tr("Project website"),
    ));
    content.append(&project_links);

    let creator_link = about_link(
        "https://diegopn.github.io/",
        &tr("Project creator’s website"),
    );
    creator_link.add_css_class("about-link-card");
    content.append(&creator_link);

    let scroller = gtk::ScrolledWindow::new();
    scroller.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    scroller.set_child(Some(&content));
    let dialog = adw::Dialog::builder()
        .title(tr("About Bellbird"))
        .content_width(440)
        .content_height(680)
        .child(&scroller)
        .build();
    let close_dialog = dialog.clone();
    close_button.connect_clicked(move |_| {
        close_dialog.close();
    });
    dialog.present(Some(parent));
}

fn about_link(url: &str, title: &str) -> gtk::LinkButton {
    let link = gtk::LinkButton::with_label(url, title);
    link.add_css_class("about-link");
    link.set_hexpand(true);
    link.set_halign(gtk::Align::Fill);
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    row.set_hexpand(true);
    let label = gtk::Label::new(Some(title));
    label.set_halign(gtk::Align::Start);
    label.set_hexpand(true);
    let icon = gtk::Image::from_icon_name("external-link-symbolic");
    icon.set_pixel_size(16);
    row.append(&label);
    row.append(&icon);
    link.set_child(Some(&row));
    link
}

fn appearance_path() -> std::path::PathBuf {
    glib::user_config_dir().join(APP_ID).join("appearance")
}

fn load_appearance() -> Appearance {
    fs::read_to_string(appearance_path())
        .ok()
        .and_then(|value| Appearance::from_key(value.trim()))
        .unwrap_or(Appearance::Automatic)
}

fn save_appearance(appearance: Appearance) {
    let path = appearance_path();
    let Some(parent) = path.parent() else {
        return;
    };
    let result = fs::create_dir_all(parent).and_then(|()| fs::write(&path, appearance.key()));
    if let Err(error) = result {
        eprintln!("Could not save appearance preference: {error}");
    }
}

fn install_styles() {
    let Some(display) = gtk::gdk::Display::default() else {
        return;
    };
    let provider = gtk::CssProvider::new();
    provider.load_from_data(include_str!("../data/style.css"));
    gtk::style_context_add_provider_for_display(
        &display,
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

struct ResultWidgets {
    file_name: gtk::Label,
    template: gtk::DropDown,
    spinner: gtk::Spinner,
    status: gtk::Label,
    resource_notice: gtk::Label,
    code_buffer: gtk::TextBuffer,
    line_number_buffer: gtk::TextBuffer,
    copy_button: gtk::Button,
    export_button: gtk::Button,
}

fn build_result() -> (gtk::Widget, ResultWidgets) {
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

fn build_landing(views: &Views) -> gtk::Widget {
    let page = gtk::Box::new(gtk::Orientation::Vertical, 24);
    page.add_css_class("bellbird-landing");
    page.set_margin_top(32);
    page.set_margin_bottom(24);
    page.set_margin_start(24);
    page.set_margin_end(24);
    page.set_vexpand(true);

    let drop_area = gtk::Box::new(gtk::Orientation::Vertical, 16);
    drop_area.set_valign(gtk::Align::Center);
    drop_area.set_halign(gtk::Align::Center);
    drop_area.set_vexpand(true);
    drop_area.set_hexpand(true);
    drop_area.add_css_class("bellbird-drop-area");

    let instruction = gtk::Label::new(Some(&tr("Drop a document here")));
    instruction.add_css_class("title-1");
    drop_area.append(&instruction);
    let select_hint = gtk::Label::new(Some(&tr("or select a file")));
    select_hint.add_css_class("dim-label");
    drop_area.append(&select_hint);

    let add_button = gtk::Button::builder()
        .icon_name("list-add-symbolic")
        .tooltip_text(tr("Select a file"))
        .build();
    add_button.add_css_class("circular");
    add_button.add_css_class("suggested-action");
    let add_accessible = tr("Select a file");
    add_button.update_property(&[gtk::accessible::Property::Label(&add_accessible)]);
    drop_area.append(&add_button);
    page.append(&drop_area);

    let formats = SUPPORTED_EXTENSIONS
        .iter()
        .map(|extension| extension.to_ascii_uppercase())
        .collect::<Vec<_>>()
        .join(", ");
    let supported = gtk::Label::new(Some(&format!("{} {formats}", tr("Supported formats:"))));
    supported.add_css_class("dim-label");
    page.append(&supported);

    let chooser_views = views.clone();
    add_button.connect_clicked(move |_| open_file_chooser(&chooser_views));

    let target = gtk::DropTarget::new(gio::File::static_type(), gtk::gdk::DragAction::COPY);
    let drop_views = views.clone();
    target.connect_drop(move |_, value, _, _| {
        let Ok(file) = value.get::<gio::File>() else {
            return false;
        };
        let Some(path) = file.path() else {
            show_toast(
                &drop_views,
                &tr("The selected file is not available as a local path."),
            );
            return false;
        };
        begin_import(path, &drop_views);
        true
    });
    drop_area.add_controller(target);

    page.upcast()
}

fn open_file_chooser(views: &Views) {
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
                    show_toast(
                        &dialog_views,
                        &tr("The selected file is not available as a local path."),
                    );
                }
            }
            Err(error) if error.matches(gio::IOErrorEnum::Cancelled) => {}
            Err(_) => show_toast(&dialog_views, &tr("Could not open the file chooser.")),
        },
    );
}

fn begin_import(path: std::path::PathBuf, views: &Views) {
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
        template.set_sensitive(template_selector_enabled(None));
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
                    template.set_sensitive(template_selector_enabled(Some(&imported.source)));
                }
                task_views.state.borrow_mut().imported = Some(imported.clone());
                display_imported(&task_views, imported, selected_template(&task_views));
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
                show_toast(&task_views, &error.user_message());
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
                show_toast(&task_views, &tr("Conversion failed."));
                glib::ControlFlow::Break
            }
        }
    });
}

fn display_imported(views: &Views, imported: ImportedFile, template: LatexTemplate) {
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
        buffer.set_text(&line_number_text(&output));
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
        show_toast(
            views,
            &tr("Generated LaTeX has structural or citation issues. Review it before exporting."),
        );
    } else if safe_fallback_used || simplified_conversion {
        show_toast(
            views,
            &tr("Some content was converted to plain text to preserve valid LaTeX."),
        );
    }
}

fn selected_template(views: &Views) -> LatexTemplate {
    views
        .template
        .upgrade()
        .map(|template| selected_template_from_index(template.selected()))
        .unwrap_or(LatexTemplate::Abnt)
}

fn selected_template_from_index(index: u32) -> LatexTemplate {
    LatexTemplate::EXPORT_OPTIONS
        .get(index as usize)
        .copied()
        .unwrap_or(LatexTemplate::Abnt)
}

fn line_number_text(content: &str) -> String {
    let count = content.lines().count().max(1);
    (1..=count)
        .map(|line| line.to_string())
        .collect::<Vec<_>>()
        .join("\n")
}

fn line_number_text_for_wrapped_rows(wrapped_rows: &[usize]) -> String {
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

fn template_selector_enabled(source: Option<&ImportedSource>) -> bool {
    !matches!(source, Some(ImportedSource::ExistingLatex(_)))
}

fn show_toast(views: &Views, message: &str) {
    if let Some(overlay) = views.toast_overlay.upgrade() {
        overlay.add_toast(adw::Toast::new(message));
    }
}

fn export_latex_with_chooser(views: &Views) {
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
                    show_toast(
                        &dialog_views,
                        &tr("The selected file is not available as a local path."),
                    );
                    return;
                };
                let Some(directory) = path.parent() else {
                    show_toast(&dialog_views, &tr("Could not export LaTeX."));
                    return;
                };
                let Some(file_name) = path.file_name() else {
                    show_toast(&dialog_views, &tr("Could not export LaTeX."));
                    return;
                };
                match save_without_overwriting(directory, Path::new(file_name), output.as_bytes()) {
                    Ok(()) => show_toast(&dialog_views, &tr("LaTeX exported successfully")),
                    Err(error) => {
                        eprintln!("Could not export LaTeX: {error}");
                        show_toast(&dialog_views, &tr("Could not export LaTeX."));
                    }
                }
            }
            Err(error) if error.matches(gio::IOErrorEnum::Cancelled) => {}
            Err(error) => {
                eprintln!("Could not open export dialog: {error}");
                show_toast(&dialog_views, &tr("Could not export LaTeX."));
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

#[cfg(test)]
mod tests {
    use super::{
        Appearance, line_number_text, line_number_text_for_wrapped_rows,
        selected_template_from_index, template_selector_enabled,
    };
    use bellbird::conversion::ImportedSource;
    use bellbird::document::{Document, LatexTemplate};

    #[test]
    fn template_selector_is_reenabled_for_a_new_import() {
        let existing_latex = ImportedSource::ExistingLatex(String::new());
        let document = ImportedSource::Document(Document::default());

        assert!(!template_selector_enabled(Some(&existing_latex)));
        assert!(template_selector_enabled(None));
        assert!(template_selector_enabled(Some(&document)));
    }

    #[test]
    fn appearance_options_round_trip_through_their_saved_keys() {
        for appearance in Appearance::ALL {
            assert_eq!(Appearance::from_key(appearance.key()), Some(appearance));
        }
        assert_eq!(Appearance::from_key("unrecognized"), None);
    }

    #[test]
    fn selected_format_matches_the_export_list_order() {
        assert_eq!(selected_template_from_index(0), LatexTemplate::Abnt);
        assert_eq!(selected_template_from_index(1), LatexTemplate::Ieee);
        assert_eq!(selected_template_from_index(2), LatexTemplate::Acm);
        assert_eq!(selected_template_from_index(3), LatexTemplate::Elsevier);
        assert_eq!(
            selected_template_from_index(4),
            LatexTemplate::SpringerNature
        );
        assert_eq!(selected_template_from_index(99), LatexTemplate::Abnt);
    }

    #[test]
    fn line_numbers_follow_source_lines_without_changing_the_source() {
        assert_eq!(line_number_text("first\nsecond\n"), "1\n2");
        assert_eq!(line_number_text(""), "1");
        assert_eq!(line_number_text_for_wrapped_rows(&[1, 3, 1]), "1\n2\n\n\n3");
    }
}
