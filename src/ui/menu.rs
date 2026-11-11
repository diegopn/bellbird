use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use bellbird::i18n::tr;

use super::APP_ID;
use super::appearance::{self, Appearance};

pub(super) struct HeaderMenu {
    pub(super) button: gtk::MenuButton,
    pub(super) open_document_button: gtk::Button,
}

pub(super) fn build_header_menu(window: &adw::ApplicationWindow) -> HeaderMenu {
    let appearance = appearance::load_appearance();
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
            appearance::draw_appearance_swatch(context, width, height, option);
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
            appearance::save_appearance(option);
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
        "bellbird-about-symbolic",
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
