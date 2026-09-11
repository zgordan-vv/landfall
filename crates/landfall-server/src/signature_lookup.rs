#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignatureLookupMode {
    Standard,
    Full,
    Strict,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SignatureLookupError {
    #[error("signature lookup is disabled in strict privacy mode")]
    DisabledInStrictMode,
    #[error("signature must not be empty")]
    EmptySignature,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureLookupResult {
    pub signature: String,
    pub status: Option<String>,
    pub logs_present: bool,
}

pub fn lookup_signature(
    mode: SignatureLookupMode,
    signature: &str,
    status: Option<&str>,
    logs_present: bool,
) -> Result<SignatureLookupResult, SignatureLookupError> {
    if signature.trim().is_empty() {
        return Err(SignatureLookupError::EmptySignature);
    }
    if mode == SignatureLookupMode::Strict {
        return Err(SignatureLookupError::DisabledInStrictMode);
    }
    Ok(SignatureLookupResult {
        signature: signature.trim().to_owned(),
        status: status.map(str::to_owned),
        logs_present,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn strict_mode_disables_lookup_and_other_modes_omit_log_contents() {
        assert_eq!(
            lookup_signature(SignatureLookupMode::Strict, "sig", Some("confirmed"), true),
            Err(SignatureLookupError::DisabledInStrictMode)
        );
        let result = lookup_signature(
            SignatureLookupMode::Standard,
            " sig ",
            Some("confirmed"),
            true,
        )
        .expect("lookup");
        assert_eq!(result.signature, "sig");
        assert_eq!(result.status.as_deref(), Some("confirmed"));
        assert!(result.logs_present);
    }
}
