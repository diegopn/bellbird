use std::fs::File;
use std::io::Read;
use std::path::Path;

use crate::document::Document;
use crate::error::BellbirdError;

use super::archive::validate_office_archive;
use super::markdown::parse_markdown;
use super::org::convert_org_document;
use super::plain_text::parse_plain_text;
use super::restructured_text::parse_restructured_text;
use super::structured::convert_structured_document;
use super::{MAX_FILE_BYTES, SUPPORTED_EXTENSIONS};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportedFile {
    pub display_name: String,
    pub source: ImportedSource,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImportedSource {
    Document(Document),
    ExistingLatex(String),
}

pub fn import_file(path: &Path) -> Result<ImportedFile, BellbirdError> {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    if !SUPPORTED_EXTENSIONS.contains(&extension.as_str()) {
        return Err(BellbirdError::UnsupportedFormat(extension));
    }
    if !path.metadata()?.is_file() {
        return Err(BellbirdError::UnsupportedFormat(extension));
    }
    let mut file = File::open(path)?;
    if !file.metadata()?.is_file() {
        return Err(BellbirdError::UnsupportedFormat(extension));
    }
    let mut bytes = Vec::new();
    file.by_ref()
        .take((MAX_FILE_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_FILE_BYTES {
        return Err(BellbirdError::FileTooLarge);
    }
    import_bytes(file_name(path), &extension, &bytes)
}

pub fn import_bytes(
    name: impl Into<String>,
    extension: &str,
    bytes: &[u8],
) -> Result<ImportedFile, BellbirdError> {
    let extension = extension.to_ascii_lowercase();
    if !SUPPORTED_EXTENSIONS.contains(&extension.as_str()) {
        return Err(BellbirdError::UnsupportedFormat(extension));
    }
    if bytes.len() > MAX_FILE_BYTES {
        return Err(BellbirdError::FileTooLarge);
    }
    let display_name = name.into();
    let source = match extension.as_str() {
        "md" | "markdown" => ImportedSource::Document(parse_markdown(
            std::str::from_utf8(bytes).map_err(|_| BellbirdError::InvalidUtf8)?,
        )),
        "txt" => ImportedSource::Document(parse_plain_text(
            std::str::from_utf8(bytes).map_err(|_| BellbirdError::InvalidUtf8)?,
        )),
        "tex" => ImportedSource::ExistingLatex(
            std::str::from_utf8(bytes)
                .map_err(|_| BellbirdError::InvalidUtf8)?
                .to_owned(),
        ),
        "rst" => ImportedSource::Document(parse_restructured_text(
            std::str::from_utf8(bytes).map_err(|_| BellbirdError::InvalidUtf8)?,
        )?),
        "org" => {
            let source = std::str::from_utf8(bytes).map_err(|_| BellbirdError::InvalidUtf8)?;
            ImportedSource::Document(convert_org_document(display_name.clone(), source)?)
        }
        "htm" | "html" => ImportedSource::Document(convert_structured_document(
            display_name.clone(),
            &extension,
            bytes,
        )?),
        "doc" => ImportedSource::Document(convert_structured_document(
            display_name.clone(),
            &extension,
            bytes,
        )?),
        "docx" | "odt" | "epub" => {
            validate_office_archive(bytes)?;
            ImportedSource::Document(convert_structured_document(
                display_name.clone(),
                &extension,
                bytes,
            )?)
        }
        _ => return Err(BellbirdError::UnsupportedFormat(extension)),
    };
    Ok(ImportedFile {
        display_name,
        source,
    })
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(str::to_owned)
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

pub fn path_is_supported(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| SUPPORTED_EXTENSIONS.contains(&extension.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}
