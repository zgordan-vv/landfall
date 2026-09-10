//! NDJSON ingestion tests.
#![allow(clippy::expect_used)]

use landfall_cli::{IngestError, ingest_ndjson, stream_ndjson};
use std::io::Cursor;

#[test]
fn accepts_multiple_events_and_skips_blank_lines() {
    let input = include_str!("../../../fixtures/protocol/v1/valid/incidents/success.batch.json");
    let batch: landfall_protocol::EventBatch = serde_json::from_str(input).expect("fixture");
    let lines = batch
        .events
        .iter()
        .map(serde_json::to_string)
        .collect::<Result<Vec<_>, _>>()
        .expect("serialize");
    let ndjson = format!("\n{}\n", lines.join("\n"));
    assert_eq!(ingest_ndjson(Cursor::new(ndjson)).expect("ingest").len(), 2);
    let ndjson = lines.join("\n");
    assert_eq!(
        stream_ndjson(Cursor::new(ndjson), |_| Ok(())).expect("stream"),
        2
    );
}

#[test]
fn reports_invalid_json_line_number() {
    let error = ingest_ndjson(Cursor::new("not-json\n")).expect_err("must fail");
    assert!(matches!(error, IngestError::Json { line: 1, .. }));
}
