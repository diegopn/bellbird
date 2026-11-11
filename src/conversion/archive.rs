use std::io::{Cursor, Read};

use crate::error::BellbirdError;

const MAX_ARCHIVE_ENTRIES: usize = 10_000;
const MAX_ARCHIVE_ENTRY_BYTES: u64 = 64 * 1024 * 1024;
const MAX_ARCHIVE_TOTAL_BYTES: u64 = 64 * 1024 * 1024;

pub(super) fn validate_office_archive(bytes: &[u8]) -> Result<(), BellbirdError> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes))
        .map_err(|error| BellbirdError::Conversion(format!("invalid document archive: {error}")))?;
    validate_archive_limits(
        &mut archive,
        MAX_ARCHIVE_ENTRIES,
        MAX_ARCHIVE_ENTRY_BYTES,
        MAX_ARCHIVE_TOTAL_BYTES,
    )
}

pub(super) fn validate_archive_limits<R: Read + std::io::Seek>(
    archive: &mut zip::ZipArchive<R>,
    max_entries: usize,
    max_entry_bytes: u64,
    max_total_bytes: u64,
) -> Result<(), BellbirdError> {
    if archive.len() > max_entries {
        return Err(BellbirdError::ArchiveTooLarge);
    }

    let mut total_bytes = 0_u64;
    let mut buffer = [0_u8; 8192];
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|error| {
            BellbirdError::Conversion(format!("could not inspect document archive: {error}"))
        })?;
        if entry.is_dir() {
            continue;
        }
        if entry.size() > max_entry_bytes
            || total_bytes.saturating_add(entry.size()) > max_total_bytes
        {
            return Err(BellbirdError::ArchiveTooLarge);
        }

        let declared_size = entry.size();
        let mut actual_size = 0_u64;
        loop {
            let read = entry.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            actual_size = actual_size.saturating_add(read as u64);
            if actual_size > max_entry_bytes
                || total_bytes.saturating_add(actual_size) > max_total_bytes
            {
                return Err(BellbirdError::ArchiveTooLarge);
            }
        }
        if actual_size != declared_size {
            return Err(BellbirdError::Conversion(
                "document archive entry size does not match its header".to_owned(),
            ));
        }
        total_bytes = total_bytes.saturating_add(actual_size);
    }
    Ok(())
}
