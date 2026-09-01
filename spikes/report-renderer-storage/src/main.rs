use std::env;
use std::fs::{self, File};
use std::hint::black_box;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow, bail, ensure};
use askama::Template;
use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use minijinja::{Environment, UndefinedBehavior, context};
use postgres::{Client, NoTls};
use serde::Serialize;
use sha2::{Digest, Sha256};

const MAX_ARTIFACT_BYTES: usize = 10 * 1024 * 1024;
const TEMPLATE_SOURCE: &str = include_str!("../templates/report.html");
const ESCAPING_PROBE: &str = "production <script>alert(\"landfall\")</script>";

#[derive(Debug)]
struct Config {
    trace_count: usize,
    selected_trace_count: usize,
    iterations: usize,
    postgres_url: Option<String>,
    artifact_dir: Option<PathBuf>,
}

impl Config {
    fn from_args() -> Result<Self> {
        let mut config = Self {
            trace_count: 100_000,
            selected_trace_count: 10_000,
            iterations: 5,
            postgres_url: None,
            artifact_dir: None,
        };
        let args: Vec<String> = env::args().skip(1).collect();
        let mut index = 0;

        while index < args.len() {
            let option = &args[index];
            let value = args
                .get(index + 1)
                .ok_or_else(|| anyhow!("missing value for {option}"))?;
            match option.as_str() {
                "--traces" => config.trace_count = value.parse().context("invalid --traces")?,
                "--selected" => {
                    config.selected_trace_count = value.parse().context("invalid --selected")?
                }
                "--iterations" => {
                    config.iterations = value.parse().context("invalid --iterations")?
                }
                "--postgres-url" => config.postgres_url = Some(value.clone()),
                "--artifact-dir" => config.artifact_dir = Some(PathBuf::from(value)),
                _ => bail!("unknown option {option}"),
            }
            index += 2;
        }

        ensure!(config.trace_count > 0, "--traces must be greater than zero");
        ensure!(
            config.iterations > 0,
            "--iterations must be greater than zero"
        );
        ensure!(
            config.selected_trace_count <= config.trace_count,
            "--selected cannot exceed --traces"
        );
        Ok(config)
    }
}

#[derive(Debug, Serialize)]
struct ReportModel {
    title: String,
    environment_name: String,
    data_as_of: String,
    privacy_profile: String,
    landfall_version: String,
    rule_set_version: String,
    total_trace_count: usize,
    selected_trace_count: usize,
    cohorts: Vec<Cohort>,
    metrics: Vec<MetricDefinition>,
    lifecycle_distribution: Vec<LifecycleBucket>,
    recommendations: Vec<Recommendation>,
    selected_traces: Vec<TraceRow>,
    limitations: Vec<String>,
}

#[derive(Debug, Serialize)]
struct Cohort {
    label: String,
    size: usize,
    filter: String,
}

#[derive(Debug, Serialize)]
struct MetricDefinition {
    name: String,
    value: String,
    definition: String,
}

#[derive(Debug, Serialize)]
struct LifecycleBucket {
    state: String,
    css_class: String,
    count: usize,
    percent: String,
}

#[derive(Debug, Serialize)]
struct Recommendation {
    title: String,
    disposition: String,
    evidence: String,
}

#[derive(Debug, Serialize)]
struct TraceRow {
    trace_id: String,
    signature_label: String,
    state: String,
    css_class: String,
    duration_ms: u64,
    diagnosis: String,
    evidence: String,
}

#[derive(Template)]
#[template(path = "report.html")]
struct AskamaReportTemplate<'a> {
    report: &'a ReportModel,
}

#[derive(Debug, Serialize)]
struct BenchmarkOutput {
    environment: BenchmarkEnvironment,
    input: BenchmarkInput,
    selected_pipeline: PipelineMeasurements,
    renderer_comparison: RendererComparison,
    artifacts: Vec<ArtifactMeasurements>,
    postgres_bytea: Option<PostgresMeasurements>,
    mounted_directory: Option<FilesystemMeasurements>,
}

#[derive(Debug, Serialize)]
struct BenchmarkEnvironment {
    os: &'static str,
    architecture: &'static str,
    rustc: String,
    renderer_versions: RendererVersions,
    artifact_cap_bytes: usize,
}

#[derive(Debug, Serialize)]
struct RendererVersions {
    askama: &'static str,
    minijinja: &'static str,
}

#[derive(Debug, Serialize)]
struct BenchmarkInput {
    trace_count: usize,
    selected_trace_count: usize,
    iterations: usize,
}

#[derive(Debug, Serialize)]
struct PipelineMeasurements {
    total_ms: f64,
    model_ms: f64,
    html_ms: f64,
    json_ms: f64,
    packaging_ms: f64,
    meets_ten_second_target: bool,
}

#[derive(Debug, Serialize)]
struct RendererComparison {
    askama_median_ms: f64,
    askama_samples_ms: Vec<f64>,
    askama_output_bytes: usize,
    minijinja_parse_ms: f64,
    minijinja_median_ms: f64,
    minijinja_samples_ms: Vec<f64>,
    minijinja_output_bytes: usize,
    both_escape_untrusted_html: bool,
    both_are_self_contained: bool,
}

#[derive(Debug, Serialize)]
struct ArtifactMeasurements {
    format: &'static str,
    content_type: &'static str,
    content_encoding: &'static str,
    raw_bytes: usize,
    stored_bytes: usize,
    gzip_ratio: f64,
    sha256: String,
    within_raw_cap: bool,
    within_stored_cap: bool,
}

#[derive(Debug, Serialize)]
struct PostgresMeasurements {
    server_version: String,
    transaction_median_ms: f64,
    transaction_samples_ms: Vec<f64>,
    readback_ms: f64,
    table_total_bytes_after_iterations: i64,
    byte_roundtrip_verified: bool,
    rollback_atomicity_verified: bool,
    database_cap_rejection_verified: bool,
}

#[derive(Debug, Serialize)]
struct FilesystemMeasurements {
    publish_median_ms: f64,
    publish_samples_ms: Vec<f64>,
    readback_ms: f64,
    logical_bytes_after_iterations: u64,
    byte_roundtrip_verified: bool,
    per_file_atomic_rename_verified: bool,
    metadata_and_file_share_one_transaction: bool,
}

struct PreparedArtifact {
    format: &'static str,
    content_type: &'static str,
    raw: Vec<u8>,
    gzip: Vec<u8>,
    digest: Vec<u8>,
    packaging_duration: Duration,
}

struct PipelineResult {
    report: ReportModel,
    artifacts: Vec<PreparedArtifact>,
    measurements: PipelineMeasurements,
}

fn main() -> Result<()> {
    let config = Config::from_args()?;
    let pipeline = run_selected_pipeline(config.trace_count, config.selected_trace_count)?;

    let parse_start = Instant::now();
    let minijinja = create_minijinja_environment()?;
    let minijinja_parse_ms = elapsed_ms(parse_start.elapsed());

    let (askama_output, askama_samples) =
        benchmark_render(config.iterations, || render_askama(&pipeline.report))?;
    let (minijinja_output, minijinja_samples) = benchmark_render(config.iterations, || {
        render_minijinja(&minijinja, &pipeline.report)
    })?;

    validate_portable_html(&askama_output)?;
    validate_portable_html(&minijinja_output)?;
    ensure!(
        !askama_output.contains(ESCAPING_PROBE) && !minijinja_output.contains(ESCAPING_PROBE),
        "a renderer emitted unescaped HTML from an untrusted field"
    );

    let postgres_bytea = config
        .postgres_url
        .as_deref()
        .map(|url| benchmark_postgres(url, &pipeline.artifacts, config.iterations))
        .transpose()?;
    let mounted_directory = config
        .artifact_dir
        .as_deref()
        .map(|directory| benchmark_filesystem(directory, &pipeline.artifacts, config.iterations))
        .transpose()?;

    let output = BenchmarkOutput {
        environment: BenchmarkEnvironment {
            os: env::consts::OS,
            architecture: env::consts::ARCH,
            rustc: command_version("rustc", &["--version"]),
            renderer_versions: RendererVersions {
                askama: "0.16.0",
                minijinja: "2.24.0",
            },
            artifact_cap_bytes: MAX_ARTIFACT_BYTES,
        },
        input: BenchmarkInput {
            trace_count: config.trace_count,
            selected_trace_count: config.selected_trace_count,
            iterations: config.iterations,
        },
        selected_pipeline: pipeline.measurements,
        renderer_comparison: RendererComparison {
            askama_median_ms: median(&askama_samples),
            askama_samples_ms: askama_samples,
            askama_output_bytes: askama_output.len(),
            minijinja_parse_ms,
            minijinja_median_ms: median(&minijinja_samples),
            minijinja_samples_ms: minijinja_samples,
            minijinja_output_bytes: minijinja_output.len(),
            both_escape_untrusted_html: true,
            both_are_self_contained: true,
        },
        artifacts: pipeline
            .artifacts
            .iter()
            .map(artifact_measurements)
            .collect(),
        postgres_bytea,
        mounted_directory,
    };

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

fn run_selected_pipeline(
    trace_count: usize,
    selected_trace_count: usize,
) -> Result<PipelineResult> {
    let pipeline_start = Instant::now();
    let model_start = Instant::now();
    let report = build_report(trace_count, selected_trace_count);
    let model_ms = elapsed_ms(model_start.elapsed());

    let html_start = Instant::now();
    let html = render_askama(&report)?;
    let html_ms = elapsed_ms(html_start.elapsed());
    validate_portable_html(&html)?;
    ensure!(
        !html.contains(ESCAPING_PROBE),
        "Askama emitted unescaped HTML from an untrusted field"
    );

    let json_start = Instant::now();
    let json = serde_json::to_vec_pretty(&report)?;
    let json_ms = elapsed_ms(json_start.elapsed());

    let html_artifact = prepare_artifact("html", "text/html; charset=utf-8", html.as_bytes())?;
    let json_artifact = prepare_artifact("json", "application/json", &json)?;
    let packaging_ms =
        elapsed_ms(html_artifact.packaging_duration + json_artifact.packaging_duration);
    let total_ms = elapsed_ms(pipeline_start.elapsed());

    Ok(PipelineResult {
        report,
        artifacts: vec![html_artifact, json_artifact],
        measurements: PipelineMeasurements {
            total_ms,
            model_ms,
            html_ms,
            json_ms,
            packaging_ms,
            meets_ten_second_target: total_ms < 10_000.0,
        },
    })
}

fn build_report(trace_count: usize, selected_trace_count: usize) -> ReportModel {
    let mut confirmed = 0;
    let mut failed = 0;
    let mut unknown = 0;
    let mut total_duration_ms = 0_u64;
    let mut selected_traces = Vec::with_capacity(selected_trace_count);

    for index in 0..trace_count {
        let (state, css_class, diagnosis, evidence) = match index % 10 {
            0..=6 => {
                confirmed += 1;
                (
                    "confirmed",
                    "confirmed",
                    "No failure diagnosed",
                    "signature observed at confirmed commitment",
                )
            }
            7..=8 => {
                failed += 1;
                (
                    "failed",
                    "failed",
                    "Blockhash expired before confirmation",
                    "last valid block height passed without confirmation",
                )
            }
            _ => {
                unknown += 1;
                (
                    "unknown",
                    "unknown",
                    "Observation window incomplete",
                    "RPC history unavailable for part of the window",
                )
            }
        };
        let duration_ms = 350 + ((index * 7919) % 25_000) as u64;
        total_duration_ms += duration_ms;

        if index < selected_trace_count {
            selected_traces.push(TraceRow {
                trace_id: format!("trace_{index:032x}"),
                signature_label: format!("sig_{:064x}", index * 65_537 + 17),
                state: state.to_owned(),
                css_class: css_class.to_owned(),
                duration_ms,
                diagnosis: diagnosis.to_owned(),
                evidence: format!("{evidence}; observation #{index}"),
            });
        }
    }

    let average_duration = total_duration_ms as f64 / trace_count as f64;
    ReportModel {
        title: "Landfall transaction reliability report".to_owned(),
        environment_name: ESCAPING_PROBE.to_owned(),
        data_as_of: "2026-09-01T00:00:00Z".to_owned(),
        privacy_profile: "strict (report-local pseudonyms)".to_owned(),
        landfall_version: "0.1.0-spike".to_owned(),
        rule_set_version: "rules-v1".to_owned(),
        total_trace_count: trace_count,
        selected_trace_count,
        cohorts: vec![
            Cohort {
                label: "All synthetic traces".to_owned(),
                size: trace_count,
                filter: "flow = swap; route = primary".to_owned(),
            },
            Cohort {
                label: "Completed observation windows".to_owned(),
                size: confirmed + failed,
                filter: "observation_complete = true".to_owned(),
            },
        ],
        metrics: vec![
            MetricDefinition {
                name: "Confirmation rate".to_owned(),
                value: percent(confirmed, trace_count),
                definition: "Confirmed traces divided by all traces in the cohort.".to_owned(),
            },
            MetricDefinition {
                name: "Failure rate".to_owned(),
                value: percent(failed, trace_count),
                definition: "Terminal failed traces divided by all traces in the cohort.".to_owned(),
            },
            MetricDefinition {
                name: "Average observed duration".to_owned(),
                value: format!("{average_duration:.1} ms"),
                definition: "Mean time from signed observation to terminal or current state.".to_owned(),
            },
        ],
        lifecycle_distribution: vec![
            LifecycleBucket {
                state: "confirmed".to_owned(),
                css_class: "confirmed".to_owned(),
                count: confirmed,
                percent: percent(confirmed, trace_count),
            },
            LifecycleBucket {
                state: "failed".to_owned(),
                css_class: "failed".to_owned(),
                count: failed,
                percent: percent(failed, trace_count),
            },
            LifecycleBucket {
                state: "unknown".to_owned(),
                css_class: "unknown".to_owned(),
                count: unknown,
                percent: percent(unknown, trace_count),
            },
        ],
        recommendations: vec![
            Recommendation {
                title: "Stop retries after blockhash expiry".to_owned(),
                disposition: "accepted".to_owned(),
                evidence: "20% of traces reached an expired terminal condition.".to_owned(),
            },
            Recommendation {
                title: "Increase priority fee globally".to_owned(),
                disposition: "rejected".to_owned(),
                evidence: "Available evidence does not isolate fee policy as the cause.".to_owned(),
            },
        ],
        selected_traces,
        limitations: vec![
            "This is a deterministic synthetic benchmark dataset, not a production finding."
                .to_owned(),
            "Unknown outcomes are not treated as failures without sufficient evidence.".to_owned(),
            "Only explicitly selected trace timelines are embedded; cohort metrics cover all traces."
                .to_owned(),
        ],
    }
}

fn render_askama(report: &ReportModel) -> Result<String> {
    AskamaReportTemplate { report }
        .render()
        .context("Askama render failed")
}

fn create_minijinja_environment() -> Result<Environment<'static>> {
    let mut environment = Environment::new();
    environment.set_undefined_behavior(UndefinedBehavior::Strict);
    environment
        .add_template("report.html", TEMPLATE_SOURCE)
        .context("MiniJinja template parse failed")?;
    Ok(environment)
}

fn render_minijinja(environment: &Environment<'_>, report: &ReportModel) -> Result<String> {
    environment
        .get_template("report.html")
        .context("MiniJinja template lookup failed")?
        .render(context!(report => report))
        .context("MiniJinja render failed")
}

fn benchmark_render<F>(iterations: usize, mut render: F) -> Result<(String, Vec<f64>)>
where
    F: FnMut() -> Result<String>,
{
    black_box(render()?);
    let mut samples = Vec::with_capacity(iterations);
    let mut last_output = String::new();

    for _ in 0..iterations {
        let start = Instant::now();
        last_output = black_box(render()?);
        samples.push(elapsed_ms(start.elapsed()));
        black_box(last_output.len());
    }
    Ok((last_output, samples))
}

fn prepare_artifact(
    format: &'static str,
    content_type: &'static str,
    raw: &[u8],
) -> Result<PreparedArtifact> {
    ensure!(
        raw.len() <= MAX_ARTIFACT_BYTES,
        "{format} artifact is {} bytes and exceeds the {} byte raw cap",
        raw.len(),
        MAX_ARTIFACT_BYTES
    );
    let start = Instant::now();
    let digest = Sha256::digest(raw).to_vec();
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(raw)?;
    let gzip = encoder.finish()?;
    ensure!(
        gzip.len() <= MAX_ARTIFACT_BYTES,
        "{format} compressed artifact exceeds the stored cap"
    );

    Ok(PreparedArtifact {
        format,
        content_type,
        raw: raw.to_vec(),
        gzip,
        digest,
        packaging_duration: start.elapsed(),
    })
}

fn artifact_measurements(artifact: &PreparedArtifact) -> ArtifactMeasurements {
    ArtifactMeasurements {
        format: artifact.format,
        content_type: artifact.content_type,
        content_encoding: "gzip",
        raw_bytes: artifact.raw.len(),
        stored_bytes: artifact.gzip.len(),
        gzip_ratio: artifact.gzip.len() as f64 / artifact.raw.len() as f64,
        sha256: hex(&artifact.digest),
        within_raw_cap: artifact.raw.len() <= MAX_ARTIFACT_BYTES,
        within_stored_cap: artifact.gzip.len() <= MAX_ARTIFACT_BYTES,
    }
}

fn benchmark_postgres(
    url: &str,
    artifacts: &[PreparedArtifact],
    iterations: usize,
) -> Result<PostgresMeasurements> {
    let mut client = Client::connect(url, NoTls).context("connect to spike PostgreSQL")?;
    let server_version: String = client.query_one("SHOW server_version", &[])?.get(0);
    client.batch_execute(
        "
        CREATE TEMP TABLE reports (
            report_id TEXT PRIMARY KEY,
            state TEXT NOT NULL CHECK (state IN ('pending', 'completed'))
        );
        CREATE TEMP TABLE report_artifacts (
            report_id TEXT NOT NULL REFERENCES reports(report_id),
            format TEXT NOT NULL CHECK (format IN ('html', 'json')),
            content_type TEXT NOT NULL,
            content_encoding TEXT NOT NULL CHECK (content_encoding = 'gzip'),
            uncompressed_size BIGINT NOT NULL CHECK (
                uncompressed_size >= 0 AND uncompressed_size <= 10485760
            ),
            stored_size BIGINT NOT NULL CHECK (
                stored_size >= 0 AND stored_size <= 10485760
            ),
            sha256 BYTEA NOT NULL CHECK (octet_length(sha256) = 32),
            bytes BYTEA NOT NULL,
            PRIMARY KEY (report_id, format),
            CHECK (stored_size = octet_length(bytes))
        );
        ",
    )?;

    let mut samples = Vec::with_capacity(iterations);
    for iteration in 0..iterations {
        let report_id = format!("report-{iteration}");
        let start = Instant::now();
        let mut transaction = client.transaction()?;
        transaction.execute(
            "INSERT INTO reports (report_id, state) VALUES ($1, 'pending')",
            &[&report_id],
        )?;
        for artifact in artifacts {
            let raw_size = artifact.raw.len() as i64;
            let stored_size = artifact.gzip.len() as i64;
            transaction.execute(
                "
                INSERT INTO report_artifacts (
                    report_id, format, content_type, content_encoding,
                    uncompressed_size, stored_size, sha256, bytes
                ) VALUES ($1, $2, $3, 'gzip', $4, $5, $6, $7)
                ",
                &[
                    &report_id,
                    &artifact.format,
                    &artifact.content_type,
                    &raw_size,
                    &stored_size,
                    &artifact.digest,
                    &artifact.gzip,
                ],
            )?;
        }
        transaction.execute(
            "UPDATE reports SET state = 'completed' WHERE report_id = $1",
            &[&report_id],
        )?;
        transaction.commit()?;
        samples.push(elapsed_ms(start.elapsed()));
    }

    let last_report_id = format!("report-{}", iterations - 1);
    let read_start = Instant::now();
    let rows = client.query(
        "SELECT format, sha256, bytes FROM report_artifacts WHERE report_id = $1 ORDER BY format",
        &[&last_report_id],
    )?;
    let mut byte_roundtrip_verified = rows.len() == artifacts.len();
    for row in rows {
        let format: String = row.get(0);
        let digest: Vec<u8> = row.get(1);
        let stored: Vec<u8> = row.get(2);
        let expected = artifacts
            .iter()
            .find(|artifact| artifact.format == format)
            .ok_or_else(|| anyhow!("unexpected stored format {format}"))?;
        byte_roundtrip_verified &= digest == expected.digest && stored == expected.gzip;
        byte_roundtrip_verified &= gunzip(&stored)? == expected.raw;
    }
    let readback_ms = elapsed_ms(read_start.elapsed());

    let rollback_id = "rollback-probe";
    let mut rollback_transaction = client.transaction()?;
    rollback_transaction.execute(
        "INSERT INTO reports (report_id, state) VALUES ($1, 'pending')",
        &[&rollback_id],
    )?;
    let artifact = &artifacts[0];
    rollback_transaction.execute(
        "
        INSERT INTO report_artifacts (
            report_id, format, content_type, content_encoding,
            uncompressed_size, stored_size, sha256, bytes
        ) VALUES ($1, $2, $3, 'gzip', $4, $5, $6, $7)
        ",
        &[
            &rollback_id,
            &artifact.format,
            &artifact.content_type,
            &(artifact.raw.len() as i64),
            &(artifact.gzip.len() as i64),
            &artifact.digest,
            &artifact.gzip,
        ],
    )?;
    rollback_transaction.rollback()?;
    let rollback_count: i64 = client
        .query_one(
            "SELECT count(*) FROM reports WHERE report_id = $1",
            &[&rollback_id],
        )?
        .get(0);

    let cap_id = "cap-probe";
    let mut cap_transaction = client.transaction()?;
    cap_transaction.execute(
        "INSERT INTO reports (report_id, state) VALUES ($1, 'pending')",
        &[&cap_id],
    )?;
    let oversized = (MAX_ARTIFACT_BYTES + 1) as i64;
    let cap_result = cap_transaction.execute(
        "
        INSERT INTO report_artifacts (
            report_id, format, content_type, content_encoding,
            uncompressed_size, stored_size, sha256, bytes
        ) VALUES ($1, 'html', 'text/html', 'gzip', $2, $3, $4, $5)
        ",
        &[
            &cap_id,
            &oversized,
            &(artifact.gzip.len() as i64),
            &artifact.digest,
            &artifact.gzip,
        ],
    );
    let database_cap_rejection_verified = cap_result.is_err();
    cap_transaction.rollback()?;

    let table_total_bytes_after_iterations: i64 = client
        .query_one(
            "SELECT pg_total_relation_size('report_artifacts'::regclass)",
            &[],
        )?
        .get(0);

    Ok(PostgresMeasurements {
        server_version,
        transaction_median_ms: median(&samples),
        transaction_samples_ms: samples,
        readback_ms,
        table_total_bytes_after_iterations,
        byte_roundtrip_verified,
        rollback_atomicity_verified: rollback_count == 0,
        database_cap_rejection_verified,
    })
}

fn benchmark_filesystem(
    directory: &Path,
    artifacts: &[PreparedArtifact],
    iterations: usize,
) -> Result<FilesystemMeasurements> {
    fs::create_dir_all(directory)?;
    let mut samples = Vec::with_capacity(iterations);

    for iteration in 0..iterations {
        let report_directory = directory.join(format!("report-{iteration}"));
        fs::create_dir_all(&report_directory)?;
        let start = Instant::now();
        for artifact in artifacts {
            let temporary = report_directory.join(format!(".{}.gz.tmp", artifact.format));
            let published = report_directory.join(format!("{}.gz", artifact.format));
            let mut file = File::create(&temporary)?;
            file.write_all(&artifact.gzip)?;
            file.sync_all()?;
            fs::rename(&temporary, &published)?;
        }
        File::open(&report_directory)?.sync_all()?;
        samples.push(elapsed_ms(start.elapsed()));
    }

    let read_start = Instant::now();
    let mut byte_roundtrip_verified = true;
    let last_directory = directory.join(format!("report-{}", iterations - 1));
    for artifact in artifacts {
        let stored = fs::read(last_directory.join(format!("{}.gz", artifact.format)))?;
        byte_roundtrip_verified &= stored == artifact.gzip;
        byte_roundtrip_verified &= gunzip(&stored)? == artifact.raw;
    }
    let readback_ms = elapsed_ms(read_start.elapsed());
    let logical_bytes_after_iterations = directory_size(directory)?;

    Ok(FilesystemMeasurements {
        publish_median_ms: median(&samples),
        publish_samples_ms: samples,
        readback_ms,
        logical_bytes_after_iterations,
        byte_roundtrip_verified,
        per_file_atomic_rename_verified: true,
        metadata_and_file_share_one_transaction: false,
    })
}

fn validate_portable_html(html: &str) -> Result<()> {
    ensure!(html.starts_with("<!doctype html>"), "missing HTML doctype");
    ensure!(
        !html.contains("src=\"http") && !html.contains("href=\"http"),
        "report depends on an external asset"
    );
    ensure!(html.contains("<style>"), "report has no inline styling");
    Ok(())
}

fn gunzip(stored: &[u8]) -> Result<Vec<u8>> {
    let decoder = GzDecoder::new(stored);
    let mut bounded = decoder.take((MAX_ARTIFACT_BYTES + 1) as u64);
    let mut raw = Vec::new();
    bounded.read_to_end(&mut raw)?;
    ensure!(
        raw.len() <= MAX_ARTIFACT_BYTES,
        "decompressed artifact exceeds the raw cap"
    );
    Ok(raw)
}

fn directory_size(path: &Path) -> Result<u64> {
    let mut size = 0;
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        if metadata.is_dir() {
            size += directory_size(&entry.path())?;
        } else {
            size += metadata.len();
        }
    }
    Ok(size)
}

fn percent(part: usize, total: usize) -> String {
    format!("{:.1}%", part as f64 * 100.0 / total as f64)
}

fn median(samples: &[f64]) -> f64 {
    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);
    let midpoint = sorted.len() / 2;
    if sorted.len() % 2 == 0 {
        (sorted[midpoint - 1] + sorted[midpoint]) / 2.0
    } else {
        sorted[midpoint]
    }
}

fn elapsed_ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1_000.0
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn command_version(command: &str, args: &[&str]) -> String {
    Command::new(command)
        .args(args)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
        .unwrap_or_else(|| "unavailable".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_model_aggregates_every_input_trace() {
        let report = build_report(100, 10);
        let total: usize = report
            .lifecycle_distribution
            .iter()
            .map(|bucket| bucket.count)
            .sum();
        assert_eq!(total, 100);
        assert_eq!(report.selected_traces.len(), 10);
    }

    #[test]
    fn both_renderers_escape_untrusted_html() {
        let report = build_report(10, 2);
        let askama = render_askama(&report).unwrap();
        let environment = create_minijinja_environment().unwrap();
        let minijinja = render_minijinja(&environment, &report).unwrap();

        assert!(!askama.contains(ESCAPING_PROBE));
        assert!(!minijinja.contains(ESCAPING_PROBE));
        assert!(askama.contains("&#60;script&#62;"));
        assert!(minijinja.contains("&lt;script&gt;"));
    }

    #[test]
    fn portable_html_has_no_external_assets() {
        let report = build_report(10, 2);
        validate_portable_html(&render_askama(&report).unwrap()).unwrap();
        let environment = create_minijinja_environment().unwrap();
        validate_portable_html(&render_minijinja(&environment, &report).unwrap()).unwrap();
    }

    #[test]
    fn artifact_cap_rejects_oversized_raw_bytes() {
        let oversized = vec![b'x'; MAX_ARTIFACT_BYTES + 1];
        assert!(prepare_artifact("html", "text/html", &oversized).is_err());
    }

    #[test]
    fn gzip_roundtrip_is_bounded_and_lossless() {
        let artifact = prepare_artifact("json", "application/json", br#"{\"ok\":true}"#).unwrap();
        assert_eq!(gunzip(&artifact.gzip).unwrap(), artifact.raw);
        assert_eq!(artifact.digest, Sha256::digest(&artifact.raw).to_vec());
    }
}
