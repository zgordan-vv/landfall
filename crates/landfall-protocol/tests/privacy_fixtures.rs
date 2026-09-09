//! Privacy corpus behavior at the typed Rust boundary.

use std::{error::Error, fs, path::PathBuf};

use landfall_protocol::WireEvent;
use serde_json::Value;

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/protocol/privacy")
}

fn read_json(relative: &str) -> Result<Value, Box<dyn Error>> {
    Ok(serde_json::from_str(&fs::read_to_string(
        fixture_root().join(relative),
    )?)?)
}

#[test]
fn rust_types_reject_prohibited_shapes_without_reflecting_canaries() -> Result<(), Box<dyn Error>> {
    let manifest = read_json("manifest.json")?;
    let fixtures = manifest["reject"]
        .as_array()
        .ok_or("privacy reject fixtures are not an array")?;

    for fixture in fixtures {
        let id = fixture["id"].as_str().ok_or("reject fixture has no id")?;
        let input_path = fixture["input"]
            .as_str()
            .ok_or("reject fixture has no input")?;
        let input = read_json(input_path)?;
        let error = match serde_json::from_value::<WireEvent>(input) {
            Ok(_) => return Err(format!("Rust accepted prohibited fixture {id}").into()),
            Err(error) => error.to_string(),
        };
        assert!(
            !error.contains("LF_CANARY_"),
            "Rust error reflected a canary for {id}"
        );
    }
    Ok(())
}

#[test]
fn rust_types_show_why_valid_free_text_needs_a_privacy_layer() -> Result<(), Box<dyn Error>> {
    let manifest = read_json("manifest.json")?;
    let fixtures = manifest["redact"]
        .as_array()
        .ok_or("privacy redact fixtures are not an array")?;

    for fixture in fixtures {
        let id = fixture["id"].as_str().ok_or("redact fixture has no id")?;
        let input_path = fixture["input"]
            .as_str()
            .ok_or("redact fixture has no input")?;
        let expected_path = fixture["expected"]
            .as_str()
            .ok_or("redact fixture has no expected output")?;

        // Structural parsing intentionally accepts bounded text. A separate
        // privacy pass must run before persistence.
        let _: WireEvent = serde_json::from_value(read_json(input_path)?)?;

        let expected = read_json(expected_path)?;
        let decoded: WireEvent = serde_json::from_value(expected.clone())?;
        assert_eq!(serde_json::to_value(decoded)?, expected, "fixture {id}");
        assert!(
            !serde_json::to_string(&expected)?.contains("LF_CANARY_"),
            "redacted output retained a canary for {id}"
        );
    }
    Ok(())
}
