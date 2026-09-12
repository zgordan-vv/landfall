//! Cross-surface secret-fixture matrix used by security tests.

use crate::export_safety::{SecretScanResult, scan_export};
use serde_json::Value;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SurfaceFinding {
    pub surface: String,
}

/// Scans representative serialized payloads from every output surface.
#[must_use]
pub fn scan_secret_matrix(
    surfaces: impl IntoIterator<Item = (String, Value)>,
) -> Vec<SurfaceFinding> {
    surfaces
        .into_iter()
        .filter_map(|(surface, payload)| {
            (scan_export(&payload) == SecretScanResult::Findings)
                .then_some(SurfaceFinding { surface })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clean_fixture_passes_all_surfaces_and_secret_is_named() {
        let clean = serde_json::json!({"trace_id": "tr_1", "status": "confirmed"});
        assert!(
            scan_secret_matrix([
                ("sdk".into(), clean.clone()),
                ("api".into(), clean.clone()),
                ("projection".into(), clean.clone()),
                ("logs".into(), clean.clone()),
                ("ui".into(), clean.clone()),
                ("report".into(), clean)
            ])
            .is_empty()
        );
        let findings =
            scan_secret_matrix([("api".into(), serde_json::json!({"private_key": "secret"}))]);
        assert_eq!(
            findings,
            vec![SurfaceFinding {
                surface: "api".into()
            }]
        );
    }
}
