use sha2::{Digest, Sha256};

use crate::{helper_ok, HelperResponse};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditHashResult {
    pub hash: String,
}

pub fn sha256_tagged(input: &[u8]) -> String {
    let digest = Sha256::digest(input);
    format!("sha256:{}", hex::encode(digest))
}

pub fn hmac_sha256_tagged(key: &[u8], input: &[u8]) -> String {
    const BLOCK_SIZE: usize = 64;
    let mut normalized_key = [0u8; BLOCK_SIZE];
    if key.len() > BLOCK_SIZE {
        let digest = Sha256::digest(key);
        normalized_key[..digest.len()].copy_from_slice(&digest);
    } else {
        normalized_key[..key.len()].copy_from_slice(key);
    }

    let mut outer_pad = [0x5cu8; BLOCK_SIZE];
    let mut inner_pad = [0x36u8; BLOCK_SIZE];
    for index in 0..BLOCK_SIZE {
        outer_pad[index] ^= normalized_key[index];
        inner_pad[index] ^= normalized_key[index];
    }

    let mut inner = Sha256::new();
    inner.update(inner_pad);
    inner.update(input);
    let inner_digest = inner.finalize();

    let mut outer = Sha256::new();
    outer.update(outer_pad);
    outer.update(inner_digest);
    format!("sha256:{}", hex::encode(outer.finalize()))
}

pub fn audit_hash(input: &[u8]) -> HelperResponse<AuditHashResult> {
    helper_ok(
        "audit_hash.sha256",
        AuditHashResult {
            hash: sha256_tagged(input),
        },
        vec![],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audit_hash_is_deterministic() {
        let first = audit_hash(br#"{"a":1}"#);
        let second = audit_hash(br#"{"a":1}"#);
        assert!(first.ok);
        assert_eq!(first.result, second.result);
        assert_eq!(first.result.unwrap().hash.len(), 71);
    }

    #[test]
    fn hmac_sha256_matches_rfc_4231_case_1() {
        let key = [0x0b; 20];
        let digest = hmac_sha256_tagged(&key, b"Hi There");
        assert_eq!(
            digest,
            "sha256:b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
        );
    }
}
