use std::{
    fs::File,
    io::{self, Read},
    path::Path,
};

use sha2::{Digest, Sha256};

use crate::store::{Error, Result};

pub(crate) const MAX_EVIDENCE_FILE_BYTES: u64 = 64 * 1024 * 1024;
pub(crate) const MAX_RECEIPT_ARTIFACT_BYTES: u64 = 64 * 1024 * 1024;
pub(crate) const MAX_RECEIPT_ARTIFACT_TOTAL_BYTES: u64 = 128 * 1024 * 1024;
pub(crate) const MAX_RECEIPT_ARTIFACTS: usize = 64;
pub(crate) const MAX_HOOK_INPUT_BYTES: u64 = 1024 * 1024;
pub(crate) const MAX_GIT_OUTPUT_BYTES: usize = 2 * 1024 * 1024;

pub(crate) fn sha256_file_bounded(
    path: &Path,
    max_bytes: u64,
    description: &str,
) -> Result<(String, u64)> {
    let mut file = File::open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() {
        return Err(Error::Invalid(format!(
            "{description} must be a regular file"
        )));
    }
    if metadata.len() > max_bytes {
        return Err(Error::Invalid(format!(
            "{description} exceeds the {max_bytes}-byte limit"
        )));
    }

    let mut hasher = Sha256::new();
    let mut total = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        total = total.saturating_add(read as u64);
        if total > max_bytes {
            return Err(Error::Invalid(format!(
                "{description} exceeds the {max_bytes}-byte limit"
            )));
        }
        hasher.update(&buffer[..read]);
    }
    Ok((format!("{:x}", hasher.finalize()), total))
}

pub(crate) fn read_utf8_bounded(
    reader: &mut impl Read,
    max_bytes: u64,
) -> io::Result<Option<String>> {
    let mut bytes = Vec::new();
    reader
        .take(max_bytes.saturating_add(1))
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > max_bytes {
        return Ok(None);
    }
    String::from_utf8(bytes)
        .map(Some)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_hash_accepts_boundary_and_rejects_larger_files() {
        let area = tempfile::tempdir().unwrap();
        let boundary = area.path().join("boundary.bin");
        std::fs::write(&boundary, [7_u8; 8]).unwrap();
        assert_eq!(sha256_file_bounded(&boundary, 8, "fixture").unwrap().1, 8);

        let oversized = area.path().join("oversized.bin");
        std::fs::write(&oversized, [7_u8; 9]).unwrap();
        assert!(
            sha256_file_bounded(&oversized, 8, "fixture")
                .unwrap_err()
                .to_string()
                .contains("8-byte limit")
        );
    }

    #[test]
    fn bounded_utf8_reader_does_not_return_a_partial_payload() {
        assert_eq!(
            read_utf8_bounded(&mut &b"1234"[..], 4).unwrap(),
            Some("1234".to_owned())
        );
        assert_eq!(read_utf8_bounded(&mut &b"12345"[..], 4).unwrap(), None);
    }
}
