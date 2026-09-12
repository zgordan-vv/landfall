//! Log review helpers that report leak categories without echoing values.

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum LogLeak {
    Address,
    Signature,
    PrivacyField,
}

/// Scans one structured/log-formatted line and returns only leak categories.
#[must_use]
pub fn scan_log_line(line: &str) -> Vec<LogLeak> {
    let lower = line.to_ascii_lowercase();
    let mut leaks = Vec::new();
    if lower.contains("wallet_address") || lower.contains("account_address") {
        leaks.push(LogLeak::Address);
    }
    if lower.contains("signature") || lower.contains("signed_transaction") {
        leaks.push(LogLeak::Signature);
    }
    if lower.contains("privacy_mode")
        || lower.contains("fingerprint")
        || lower.contains("private_key")
    {
        leaks.push(LogLeak::PrivacyField);
    }
    leaks
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn detects_categories_without_returning_sensitive_values() {
        assert_eq!(
            scan_log_line("trace=1 wallet_address=ABC signature=XYZ"),
            vec![LogLeak::Address, LogLeak::Signature]
        );
        assert_eq!(
            scan_log_line("privacy_mode=strict fingerprint=abc"),
            vec![LogLeak::PrivacyField]
        );
        assert!(scan_log_line("trace=1 status=confirmed route=primary").is_empty());
    }
}
