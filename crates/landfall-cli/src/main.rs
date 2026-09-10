//! Landfall command-line interface entry point.

use std::{
    env,
    fs::File,
    io::{self, BufReader},
};

use landfall_cli::stream_ndjson;

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
    let accepted = stream_ndjson(BufReader::new(input), |_| Ok(()))?;
    println!("{{\"accepted_events\":{accepted}}}");
    Ok(())
}
