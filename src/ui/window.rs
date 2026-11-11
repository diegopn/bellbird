use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use bellbird::i18n::tr;

use super::export;
use super::import;
use super::landing;
use super::menu;
use super::result;
use super::state::{UiState, Views};

pub(super) fn build_window(application: &adw::Application) {
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

    let header_menu = menu::build_header_menu(&window);
    let menu_button = header_menu.button.clone();
    menu_button.set_visible(true);
    header.pack_start(&menu_button);
    shell.append(&header);

    let stack = gtk::Stack::new();
    stack.set_hexpand(true);
    stack.set_vexpand(true);
    stack.set_transition_type(gtk::StackTransitionType::Crossfade);
    let (result_page, result) = result::build_result();
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
    let landing = landing::build_landing(&views);
    stack.add_named(&landing, Some("import"));
    stack.set_visible_child_name("import");
    shell.append(&stack);
    toast_overlay.set_child(Some(&shell));
    window.set_content(Some(&toast_overlay));

    let open_views = views.clone();
    open_button.connect_clicked(move |_| import::open_file_chooser(&open_views));

    let menu_open_views = views.clone();
    header_menu
        .open_document_button
        .connect_clicked(move |_| import::open_file_chooser(&menu_open_views));

    let template_views = views.clone();
    result.template.connect_selected_notify(move |_| {
        let imported = template_views.state.borrow().imported.clone();
        if let Some(imported) = imported {
            import::display_imported(
                &template_views,
                imported,
                result::selected_template(&template_views),
            );
        }
    });

    let copy_views = views.clone();
    result.copy_button.connect_clicked(move |_| {
        if let Some(display) = gtk::gdk::Display::default() {
            let output = copy_views.state.borrow().output.clone();
            display.clipboard().set_text(&output);
            result::show_toast(&copy_views, &tr("Copied to clipboard"));
        }
    });

    let export_views = views.clone();
    result
        .export_button
        .connect_clicked(move |_| export::export_latex_with_chooser(&export_views));

    window.present();
}

fn install_styles() {
    let Some(display) = gtk::gdk::Display::default() else {
        return;
    };
    let provider = gtk::CssProvider::new();
    provider.load_from_data(include_str!("../../data/style.css"));
    gtk::style_context_add_provider_for_display(
        &display,
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}
