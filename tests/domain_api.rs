use bellbird::{document, latex};

#[test]
fn latex_module_exports_behavior_and_document_keeps_compatibility() {
    let document = document::Document::default();
    let output = latex::render_latex(&document, latex::LatexTemplate::Standard);

    assert_eq!(latex::escape_latex("a_b"), r"a\_b");
    assert!(latex::validate_generated_latex(&output).is_ok());
    assert!(!latex::used_safe_fallback(&output));

    let compatible_output = document::render_latex(&document, document::LatexTemplate::Standard);
    assert_eq!(compatible_output, output);
    assert_eq!(document::escape_latex("a_b"), r"a\_b");
    assert!(document::validate_generated_latex(&compatible_output).is_ok());
    assert!(!document::used_safe_fallback(&compatible_output));
}
