//! Landfall command-line interface entry point.

use std::{
    env,
    fs::File,
    io::{self, BufReader},
};

use landfall_cli::{group_traces, ingest_ndjson};
use landfall_core::versions::current_versions;
use landfall_report::{ReportCounts, ReportDocument, TraceReport};

fn main() {
    if let Err(error) = run() {
        eprintln!("landfall: {error}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    if args.next().as_deref() != Some("ingest") {
        return Err("usage: landfall ingest <ndjson-file>".into());
    }
    let path = args.next().ok_or("missing NDJSON file")?;
    if args.next().is_some() {
        return Err("usage: landfall ingest <ndjson-file>".into());
    }
    let input: Box<dyn io::Read> = if path == "-" {
        Box::new(io::stdin())
    } else {
        Box::new(File::open(path)?)
    };
    let events = ingest_ndjson(BufReader::new(input))?;
    let grouped = group_traces(events)?;
    let traces = grouped
        .analyses
        .iter()
        .map(|(trace_id, analysis)| {
            TraceReport::from_state(
                *trace_id,
                analysis.projection.trace().state(),
                ReportCounts {
                    data_quality_findings: analysis.data_quality_findings,
                    confirmed_diagnostics: analysis.confirmed_diagnostics,
                    probable_diagnostics: analysis.probable_diagnostics,
                    unknown_diagnostics: analysis.unknown_diagnostics,
                    recommendations: analysis.recommendations.len(),
                },
            )
        })
        .collect();
    let document = ReportDocument::new(current_versions(), traces, grouped.aliases.len());
    println!("{}", serde_json::to_string_pretty(&document)?);
    Ok(())
}
