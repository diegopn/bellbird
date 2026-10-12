use std::io;

use thiserror::Error;

use crate::i18n::tr;

#[derive(Debug, Error)]
pub enum BellbirdError {
    #[error("{0}")]
    Io(#[from] io::Error),
    #[error("{0}")]
    UnsupportedFormat(String),
    #[error("file exceeds the supported size limit")]
    FileTooLarge,
    #[error("document archive exceeds safe processing limits")]
    ArchiveTooLarge,
    #[error("document text is not valid UTF-8")]
    InvalidUtf8,
    #[error("document conversion failed: {0}")]
    Conversion(String),
}

impl BellbirdError {
    pub fn user_message(&self) -> String {
        match self {
            Self::Io(error) => format!("{}: {error}", tr("Could not read the file")),
            Self::UnsupportedFormat(extension) => {
                format!(
                    "{}: .{extension}",
                    tr("This file format is not supported yet")
                )
            }
            Self::FileTooLarge => tr("Files must be 10 MiB or smaller."),
            Self::ArchiveTooLarge => tr("The document expands beyond the safe processing limit."),
            Self::InvalidUtf8 => tr("The document is not valid UTF-8."),
            Self::Conversion(details) => {
                format!("{}: {details}", tr("Could not convert the document"))
            }
        }
    }
}
