use std::cell::RefCell;
use std::rc::Rc;

use gtk::glib;

use bellbird::conversion::ImportedFile;

#[derive(Clone)]
pub(super) struct Views {
    pub(super) window: glib::WeakRef<adw::ApplicationWindow>,
    pub(super) toast_overlay: glib::WeakRef<adw::ToastOverlay>,
    pub(super) stack: glib::WeakRef<gtk::Stack>,
    pub(super) open_button: glib::WeakRef<gtk::Button>,
    pub(super) menu_button: glib::WeakRef<gtk::MenuButton>,
    pub(super) file_name: glib::WeakRef<gtk::Label>,
    pub(super) template: glib::WeakRef<gtk::DropDown>,
    pub(super) spinner: glib::WeakRef<gtk::Spinner>,
    pub(super) status: glib::WeakRef<gtk::Label>,
    pub(super) resource_notice: glib::WeakRef<gtk::Label>,
    pub(super) code_buffer: glib::WeakRef<gtk::TextBuffer>,
    pub(super) line_number_buffer: glib::WeakRef<gtk::TextBuffer>,
    pub(super) copy_button: glib::WeakRef<gtk::Button>,
    pub(super) export_button: glib::WeakRef<gtk::Button>,
    pub(super) state: Rc<RefCell<UiState>>,
}

#[derive(Default)]
pub(super) struct UiState {
    pub(super) imported: Option<ImportedFile>,
    pub(super) output: String,
    pub(super) import_generation: u64,
}
