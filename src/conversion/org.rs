use crate::document::Document;
use crate::error::BellbirdError;

use super::MAX_FILE_BYTES;
use super::structured::convert_structured_document;

pub(super) fn convert_org_document(
    display_name: String,
    source: &str,
) -> Result<Document, BellbirdError> {
    let mut html = Vec::new();
    orgize::Org::parse(source).write_html(&mut html)?;
    if html.len() > MAX_FILE_BYTES {
        return Err(BellbirdError::FileTooLarge);
    }
    convert_structured_document(display_name, "html", &html)
}
