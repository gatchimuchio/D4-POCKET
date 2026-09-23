use crate::audit_hash::sha256_tagged;
use crate::checkpoint::public_key;
use crate::{helper_error, helper_ok, HelperResponse};
use ring::signature::{UnparsedPublicKey, ED25519};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateCandidate {
    pub update_id: String,
    pub signature: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateVerificationResult {
    pub update_id: String,
    pub signature_present: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedUpdateCandidate {
    pub update_id: String,
    pub signed_bytes: Vec<u8>,
    pub signature: Option<Vec<u8>>,
    pub signer_fingerprint: String,
}

pub fn verify_signed_update_signature(
    candidate: &SignedUpdateCandidate,
    public_key_der: &[u8],
    expected_fingerprint: &str,
) -> HelperResponse<UpdateVerificationResult> {
    let key = match public_key(public_key_der) {
        Ok(key) => key,
        Err(message) => {
            return helper_error(
                "update.verify_signature",
                "update_trusted_key_invalid",
                &message,
                true,
                vec![],
            )
        }
    };
    if expected_fingerprint != sha256_tagged(key)
        || candidate.signer_fingerprint != expected_fingerprint
    {
        return helper_error(
            "update.verify_signature",
            "update_signer_untrusted",
            "update signer fingerprint is not trusted",
            true,
            vec![],
        );
    }
    let signature = match candidate.signature.as_deref() {
        Some(signature) if signature.len() == 64 => signature,
        _ => {
            return helper_error(
                "update.verify_signature",
                "update_signature_required",
                "update signature is required",
                true,
                vec![],
            )
        }
    };
    if UnparsedPublicKey::new(&ED25519, key)
        .verify(&candidate.signed_bytes, signature)
        .is_err()
    {
        return helper_error(
            "update.verify_signature",
            "update_signature_invalid",
            "update signature verification failed",
            true,
            vec![],
        );
    }
    helper_ok(
        "update.verify_signature",
        UpdateVerificationResult {
            update_id: candidate.update_id.clone(),
            signature_present: true,
        },
        vec![],
    )
}

pub fn verify_update_signature(
    candidate: &UpdateCandidate,
) -> HelperResponse<UpdateVerificationResult> {
    if candidate.signature.as_deref().unwrap_or("").is_empty() {
        return helper_error(
            "update.verify_signature",
            "update_signature_required",
            "update signature is required",
            true,
            vec![],
        );
    }

    helper_ok(
        "update.verify_signature",
        UpdateVerificationResult {
            update_id: candidate.update_id.clone(),
            signature_present: true,
        },
        vec![],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use ring::{
        rand::SystemRandom,
        signature::{Ed25519KeyPair, KeyPair},
    };

    #[test]
    fn update_verification_rejects_unsigned_update() {
        let response = verify_update_signature(&UpdateCandidate {
            update_id: "update-1".to_string(),
            signature: None,
        });
        assert!(!response.ok);
        assert_eq!(response.error.unwrap().code, "update_signature_required");
    }

    #[test]
    fn signed_update_requires_trusted_ed25519_signature() {
        let document = br#"{"update_id":"update-1","version":"1.1.0"}"#;
        let pkcs8 = Ed25519KeyPair::generate_pkcs8(&SystemRandom::new()).unwrap();
        let key_pair = Ed25519KeyPair::from_pkcs8(pkcs8.as_ref()).unwrap();
        let public_key_der = [
            0x30, 0x2a, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x03, 0x21, 0x00,
        ]
        .into_iter()
        .chain(key_pair.public_key().as_ref().iter().copied())
        .collect::<Vec<_>>();
        let fingerprint = sha256_tagged(key_pair.public_key().as_ref());
        let signature = key_pair.sign(document);
        let response = verify_signed_update_signature(
            &SignedUpdateCandidate {
                update_id: "update-1".into(),
                signed_bytes: document.to_vec(),
                signature: Some(signature.as_ref().to_vec()),
                signer_fingerprint: fingerprint.clone(),
            },
            &public_key_der,
            &fingerprint,
        );
        assert!(response.ok);
        assert_eq!(response.result.unwrap().update_id, "update-1");
    }
}
