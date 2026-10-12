use gettextrs::{LocaleCategory, bind_textdomain_codeset, bindtextdomain, setlocale, textdomain};

const APP_ID: &str = "io.github.diegopn.Bellbird";

pub fn initialize() {
    // SAFETY: initialization runs on the main thread before the application starts workers.
    let _ = unsafe { setlocale(LocaleCategory::LcAll, "") };
    let _ = bindtextdomain(APP_ID, env!("BELLBIRD_LOCALE_DIR"));
    let _ = bind_textdomain_codeset(APP_ID, "UTF-8");
    let _ = textdomain(APP_ID);
}

pub fn tr(message: &str) -> String {
    gettextrs::gettext(message)
}
