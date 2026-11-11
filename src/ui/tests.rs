use super::appearance::Appearance;
use super::result::{
    line_number_text, line_number_text_for_wrapped_rows, selected_template_from_index,
    template_selector_enabled,
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
