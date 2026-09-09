//! The same checked-in JSON corpus is consumed by Rust and the Node schema validator.

use std::{collections::BTreeSet, error::Error, fs, path::PathBuf};

use landfall_protocol::{EventBatch, WireEvent};
use serde_json::Value;

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/protocol/v1")
}

fn fixture_manifest() -> Result<Value, Box<dyn Error>> {
    Ok(serde_json::from_str(&fs::read_to_string(
        fixture_root().join("manifest.json"),
    )?)?)
}

fn fixture_document(path: &str) -> Result<Value, Box<dyn Error>> {
    Ok(serde_json::from_str(&fs::read_to_string(
        fixture_root().join(path),
    )?)?)
}

fn is_batch(schema: &str) -> bool {
    schema.ends_with("/batch.schema.json")
}

#[test]
fn rust_accepts_and_round_trips_every_valid_shared_fixture() -> Result<(), Box<dyn Error>> {
    let manifest = fixture_manifest()?;
    let fixtures = manifest["valid"]
        .as_array()
        .ok_or("fixture manifest valid entry is not an array")?;

    for fixture in fixtures {
        let id = fixture["id"].as_str().ok_or("valid fixture has no id")?;
        let path = fixture["path"]
            .as_str()
            .ok_or("valid fixture has no path")?;
        let schema = fixture["schema"]
            .as_str()
            .ok_or("valid fixture has no schema")?;
        let original = fixture_document(path)?;
        let round_trip = if is_batch(schema) {
            let decoded: EventBatch = serde_json::from_value(original.clone())?;
            serde_json::to_value(decoded)?
        } else {
            let decoded: WireEvent = serde_json::from_value(original.clone())?;
            serde_json::to_value(decoded)?
        };
        assert_eq!(round_trip, original, "valid fixture {id} changed");
    }
    Ok(())
}

#[test]
fn rust_rejects_every_invalid_shared_fixture() -> Result<(), Box<dyn Error>> {
    let manifest = fixture_manifest()?;
    let fixtures = manifest["invalid"]
        .as_array()
        .ok_or("fixture manifest invalid entry is not an array")?;
    let stable_categories = BTreeSet::from([
        "collection_out_of_range",
        "contradictory_evidence",
        "invalid_format",
        "missing_required_evidence",
        "missing_required_field",
        "numeric_out_of_range",
        "type_mismatch",
        "unknown_field",
        "unsupported_enum_value",
        "wrong_event_type",
    ]);

    for fixture in fixtures {
        let id = fixture["id"].as_str().ok_or("invalid fixture has no id")?;
        let path = fixture["path"]
            .as_str()
            .ok_or("invalid fixture has no path")?;
        let schema = fixture["schema"]
            .as_str()
            .ok_or("invalid fixture has no schema")?;
        let expected_category = fixture["expected_category"]
            .as_str()
            .ok_or("invalid fixture has no expected category")?;
        assert!(
            stable_categories.contains(expected_category),
            "invalid fixture {id} has an unstable category"
        );

        let document = fixture_document(path)?;
        let rejected = if is_batch(schema) {
            serde_json::from_value::<EventBatch>(document).is_err()
        } else {
            serde_json::from_value::<WireEvent>(document).is_err()
        };
        assert!(rejected, "Rust unexpectedly accepted invalid fixture {id}");
    }
    Ok(())
}
