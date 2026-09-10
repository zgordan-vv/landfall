//! Semantic-version manifest tests.

use landfall_core::versions::{current_versions, same_semantics};

#[test]
fn current_manifest_is_self_consistent() {
    let versions = current_versions();
    assert_eq!(versions.reducer, "trace-reducer-v1");
    assert_eq!(versions.diagnostics, "diagnostic-rules-v1");
    assert_eq!(versions.metrics, "metric-definitions-v1");
    assert_eq!(versions.recommendations, "recommendation-rules-v1");
    assert!(same_semantics(versions, current_versions()));
}
