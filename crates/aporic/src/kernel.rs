//! Exact kernel identity and integrity checks.

use sha2::{Digest, Sha256};

pub const CANON: &[u8] = include_bytes!("../../../canon/KERNEL.md");
pub const EXPECTED_SHA256: &str =
    "4eb039da10d9b269fe5b42c8c0e18a8a734c7d8c59197e9b567406e4ffde460e";

pub fn digest() -> String {
    format!("{:x}", Sha256::digest(CANON))
}

pub fn verify() -> Result<(), String> {
    let observed = digest();
    if observed == EXPECTED_SHA256 {
        Ok(())
    } else {
        Err(format!(
            "kernel digest mismatch: expected {EXPECTED_SHA256}, observed {observed}"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_versioned_candidate_has_the_manifest_digest() {
        verify().unwrap();
    }
}
