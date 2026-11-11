use std::fs;

use bellbird::i18n::tr;
use gtk::glib;

use super::APP_ID;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Appearance {
    Automatic,
    Light,
    Dark,
}

impl Appearance {
    pub(super) const ALL: [Self; 3] = [Self::Automatic, Self::Light, Self::Dark];

    pub(super) fn key(self) -> &'static str {
        match self {
            Self::Automatic => "automatic",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    pub(super) fn from_key(key: &str) -> Option<Self> {
        match key {
            "automatic" => Some(Self::Automatic),
            "light" => Some(Self::Light),
            "dark" => Some(Self::Dark),
            _ => None,
        }
    }

    pub(super) fn color_scheme(self) -> adw::ColorScheme {
        match self {
            Self::Automatic => adw::ColorScheme::Default,
            Self::Light => adw::ColorScheme::ForceLight,
            Self::Dark => adw::ColorScheme::ForceDark,
        }
    }

    pub(super) fn label(self) -> String {
        tr(match self {
            Self::Automatic => "Automatic",
            Self::Light => "Light",
            Self::Dark => "Dark",
        })
    }
}

pub(super) fn draw_appearance_swatch(
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

fn appearance_path() -> std::path::PathBuf {
    glib::user_config_dir().join(APP_ID).join("appearance")
}

pub(super) fn load_appearance() -> Appearance {
    fs::read_to_string(appearance_path())
        .ok()
        .and_then(|value| Appearance::from_key(value.trim()))
        .unwrap_or(Appearance::Automatic)
}

pub(super) fn save_appearance(appearance: Appearance) {
    let path = appearance_path();
    let Some(parent) = path.parent() else {
        return;
    };
    let result = fs::create_dir_all(parent).and_then(|()| fs::write(&path, appearance.key()));
    if let Err(error) = result {
        eprintln!("Could not save appearance preference: {error}");
    }
}
