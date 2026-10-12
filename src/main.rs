mod ui;

fn main() -> gtk::glib::ExitCode {
    bellbird::i18n::initialize();
    ui::run()
}
