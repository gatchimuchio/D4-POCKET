use crate::{helper_error, helper_ok, HelperResponse};

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

    #[test]
    fn update_verification_rejects_unsigned_update() {
        let response = verify_update_signature(&UpdateCandidate {
            update_id: "update-1".to_string(),
            signature: None,
        });
        assert!(!response.ok);
        assert_eq!(response.error.unwrap().code, "update_signature_required");
    }
}
