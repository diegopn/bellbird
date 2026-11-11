use adw::prelude::*;
use bellbird::conversion::SUPPORTED_EXTENSIONS;
use bellbird::i18n::tr;
use gtk::gio;

use super::import;
use super::result;
use super::state::Views;

pub(super) fn build_landing(views: &Views) -> gtk::Widget {
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
    add_button.connect_clicked(move |_| import::open_file_chooser(&chooser_views));

    let target = gtk::DropTarget::new(gio::File::static_type(), gtk::gdk::DragAction::COPY);
    let drop_views = views.clone();
    target.connect_drop(move |_, value, _, _| {
        let Ok(file) = value.get::<gio::File>() else {
            return false;
        };
        let Some(path) = file.path() else {
            result::show_toast(
                &drop_views,
                &tr("The selected file is not available as a local path."),
            );
            return false;
        };
        import::begin_import(path, &drop_views);
        true
    });
    drop_area.add_controller(target);

    page.upcast()
}
