mod appearance;
mod export;
mod import;
mod landing;
mod menu;
mod result;
mod state;
mod window;

use adw::prelude::*;
use gtk::glib;

const APP_ID: &str = "io.github.diegopn.Bellbird";

pub fn run() -> glib::ExitCode {
    let application = adw::Application::builder().application_id(APP_ID).build();
    application.connect_activate(window::build_window);
    application.run()
}

#[cfg(test)]
mod tests;
