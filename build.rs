use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

const DOMAIN: &str = "io.github.diegopn.Bellbird";

fn main() {
    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest directory"));
    let locale_dir = env::var_os("BELLBIRD_LOCALE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env::var_os("OUT_DIR").expect("build output directory")).join("locale")
        });

    println!("cargo:rerun-if-changed=po/pt_BR.po");
    println!("cargo:rerun-if-changed=po/es.po");
    println!("cargo:rerun-if-env-changed=BELLBIRD_LOCALE_DIR");

    for (source, locales) in [
        ("pt_BR.po", &["pt_BR", "pt", "pt_PT"][..]),
        ("es.po", &["es", "es_ES", "es_419"][..]),
    ] {
        let source = manifest.join("po").join(source);
        for locale in locales {
            let output_dir = locale_dir.join(locale).join("LC_MESSAGES");
            fs::create_dir_all(&output_dir).expect("create gettext output directory");
            let output = output_dir.join(format!("{DOMAIN}.mo"));
            let status = Command::new("msgfmt")
                .arg("--check")
                .arg("--output-file")
                .arg(output)
                .arg(&source)
                .status()
                .expect("GNU gettext msgfmt is required to build Bellbird");
            assert!(status.success(), "failed to compile {}", source.display());
        }
    }

    println!(
        "cargo:rustc-env=BELLBIRD_LOCALE_DIR={}",
        locale_dir.display()
    );
}
