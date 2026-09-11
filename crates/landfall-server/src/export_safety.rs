//! Export-specific redaction and secret scanning.

use serde_json::Value;

const SENSITIVE_KEYS: &[&str] = &[
    "private_key",
    "seed_phrase",
    "signed_transaction_bytes",
    "raw_transaction",
    "authorization",
    "cookie",
    "set_cookie",
    "rpc_url",
    "endpoint_url",
];

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum SecretScanResult {
    Clean,
    Findings,
}

/// Replaces sensitive object fields recursively while preserving shape.
pub fn redact_export(value: &mut Value) {
    match value {
        Value::Object(object) => {
            for (key, child) in object.iter_mut() {
                if SENSITIVE_KEYS.contains(&key.as_str()) {
                    *child = Value::String("[redacted]".into());
                } else {
                    redact_export(child);
                }
            }
        }
        Value::Array(values) => values.iter_mut().for_each(redact_export),
        _ => {}
    }
}

/// Scans keys and scalar values for known credential-bearing material.
#[must_use]
pub fn scan_export(value: &Value) -> SecretScanResult {
    if contains_secret(value) {
        SecretScanResult::Findings
    } else {
        SecretScanResult::Clean
    }
}

fn contains_secret(value: &Value) -> bool {
    match value {
        Value::Object(object) => object.iter().any(|(key, child)| {
            (SENSITIVE_KEYS.contains(&key.as_str()) && child != "[redacted]")
                || contains_secret(child)
        }),
        Value::Array(values) => values.iter().any(contains_secret),
        Value::String(text) => {
            text.starts_with("Bearer ") || text.contains("-----BEGIN") || text.len() > 5000
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn redacts_nested_secrets_and_scan_rejects_before_redaction() {
        let mut value = serde_json::json!({"trace": {"rpc_url": "https://secret", "ok": 1}, "authorization": "Bearer token"});
        assert_eq!(scan_export(&value), SecretScanResult::Findings);
        redact_export(&mut value);
        assert_eq!(value["trace"]["rpc_url"], "[redacted]");
        assert_eq!(value["authorization"], "[redacted]");
        assert_eq!(scan_export(&value), SecretScanResult::Clean);
    }
}
