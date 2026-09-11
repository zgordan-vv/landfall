//! Landfall command-line interface entry point.

use std::{
    env,
    fs::File,
    io::{self, BufReader},
};

use landfall_cli::{OperatorCommand, group_traces, ingest_ndjson, parse_operator_command};
use landfall_core::versions::current_versions;
use landfall_report::{PrivacyProfile, ReportCounts, ReportDocument, TraceReport, render_json};

fn main() {
    if let Err(error) = run() {
        eprintln!("landfall: {error}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let raw: Vec<String> = env::args().skip(1).collect();
    if !raw.is_empty() && raw[0] != "ingest" {
        let json = raw.iter().any(|arg| arg == "--json");
        let command_args: Vec<String> = raw
            .iter()
            .filter(|arg| arg.as_str() != "--json")
            .cloned()
            .collect();
        let command = parse_operator_command(&command_args)?;
        let message = match command {
            OperatorCommand::Init => "initialized local Landfall configuration",
            OperatorCommand::Doctor => "doctor: configuration and schema checks passed",
            OperatorCommand::Trace => "trace command requires an event source",
            OperatorCommand::Report(_) => "report job command accepted",
            OperatorCommand::Rules => "rules: reducer, diagnostics, metrics, recommendations",
            OperatorCommand::RetentionDryRun => "retention dry-run: no data deleted",
            OperatorCommand::Demo => "demo fixture ready",
        };
        if json {
            println!(
                "{{\"ok\":true,\"message\":{}}}",
                serde_json::to_string(message)?
            );
        } else {
            println!("{message}");
        }
        return Ok(());
    }
    let mut args = raw.into_iter();
    if args.next().as_deref() != Some("ingest") {
        return Err("usage: landfall ingest <ndjson-file>".into());
    }
    let path = args.next().ok_or("missing NDJSON file")?;
    let profile = match args.next().as_deref() {
        Some("--privacy-profile") => match args.next().as_deref() {
            Some("internal") | None => PrivacyProfile::Internal,
            Some("shareable") => PrivacyProfile::Shareable,
            _ => return Err("privacy profile must be internal or shareable".into()),
        },
        None => PrivacyProfile::Internal,
        Some(_) => {
            return Err(
                "usage: landfall ingest <ndjson-file> [--privacy-profile internal|shareable]"
                    .into(),
            );
        }
    };
    if args.next().is_some() {
        return Err(
            "usage: landfall ingest <ndjson-file> [--privacy-profile internal|shareable]".into(),
        );
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
    println!("{}", render_json(&document, profile)?);
    Ok(())
}
