//! Exact kernel identity and integrity checks.

use sha2::{Digest, Sha256};

pub const CANON: &[u8] = include_bytes!("../../../canon/KERNEL.md");
pub const EXPECTED_SHA256: &str =
    "86100dd011cce7132886734e239c2a4ed99b9354130492901c2048821dce5815";

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
    fn embedded_candidate_has_the_manifest_digest() {
        verify().unwrap();
    }
}
