//! Cross-language compatibility decisions exercised from the shared corpus.

use std::{collections::BTreeSet, error::Error, fs, path::PathBuf};

use landfall_protocol::{
    SUPPORTED_EVENT_TYPES, SUPPORTED_SCHEMA_VERSIONS, check_event_compatibility,
};
use serde_json::Value;

fn repository_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

fn read_json(relative: &str) -> Result<Value, Box<dyn Error>> {
    Ok(serde_json::from_str(&fs::read_to_string(
        repository_path(relative),
    )?)?)
}

#[test]
fn rust_capabilities_exactly_match_the_protocol_manifest() -> Result<(), Box<dyn Error>> {
    let manifest = read_json("schemas/events/v1/manifest.json")?;
    let manifest_versions = manifest["supported_versions"]
        .as_array()
        .ok_or("supported_versions is not an array")?
        .iter()
        .map(|entry| entry["version"].as_str().ok_or("version is not a string"))
        .collect::<Result<Vec<_>, _>>()?;
    assert_eq!(SUPPORTED_SCHEMA_VERSIONS, manifest_versions);

    let manifest_events = manifest["event_types"]
        .as_object()
        .ok_or("event_types is not an object")?
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    assert_eq!(
        SUPPORTED_EVENT_TYPES
            .iter()
            .copied()
            .collect::<BTreeSet<_>>(),
        manifest_events
    );
    Ok(())
}

#[test]
fn rust_matches_every_shared_compatibility_decision() -> Result<(), Box<dyn Error>> {
    let fixtures = read_json("fixtures/protocol/compatibility.json")?;
    let cases = fixtures["cases"]
        .as_array()
        .ok_or("compatibility cases is not an array")?;

    for fixture in cases {
        let id = fixture["id"].as_str().ok_or("fixture id is not a string")?;
        let schema_version = fixture["schema_version"]
            .as_str()
            .ok_or("schema_version is not a string")?;
        let event_type = fixture["event_type"]
            .as_str()
            .ok_or("event_type is not a string")?;
        let expected = &fixture["expected"];
        let actual = check_event_compatibility(schema_version, event_type);

        if expected["supported"] == true {
            assert!(actual.is_ok(), "{id} was unexpectedly rejected");
        } else {
            let code = match actual {
                Ok(()) => return Err(format!("unsupported fixture {id} was accepted").into()),
                Err(error) => error.code().as_str(),
            };
            assert_eq!(Some(code), expected["code"].as_str(), "fixture {id}");
        }
    }
    Ok(())
}

#[test]
fn compatibility_errors_do_not_reflect_rejected_input() -> Result<(), Box<dyn Error>> {
    let rejected_version = "attacker-controlled-version";
    let rejected_event = "attacker-controlled-event";
    let error = match check_event_compatibility(rejected_version, rejected_event) {
        Ok(()) => return Err("unknown compatibility tuple was accepted".into()),
        Err(error) => error.to_string(),
    };

    assert!(!error.contains(rejected_version));
    assert!(!error.contains(rejected_event));
    assert_eq!(error, "LF_UNSUPPORTED_SCHEMA_VERSION");
    Ok(())
}
