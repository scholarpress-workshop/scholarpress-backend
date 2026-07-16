use clap::Parser;
use std::path::PathBuf;
use std::process;

#[derive(Parser)]
pub struct CheckArgs {
    #[arg(short, long, help = "Path to institution spec YAML file")]
    pub spec: PathBuf,

    #[arg(short, long, help = "Output results as JSON")]
    pub json: bool,

    #[arg(short, long, help = "Show only FAIL and ERROR results")]
    pub quiet: bool,

    #[arg(long, help = "Run only this specific check (by check ID)")]
    pub check: Option<String>,

    #[arg(
        short = 'C',
        long,
        help = "Run only checks in this category (layout, typography, structure, content)"
    )]
    pub category: Option<String>,

    #[arg(
        long,
        help = "Dump extracted document intermediate representation as JSON and exit"
    )]
    pub dump_extract: bool,

    #[arg(help = "Path to dissertation PDF")]
    pub pdf: PathBuf,
}

pub fn run(args: &CheckArgs) {
    if !args.pdf.exists() {
        eprintln!("Error: PDF not found: {}", args.pdf.display());
        process::exit(2);
    }

    if args.dump_extract {
        match sp_extract::extract_pdf(&std::fs::read(&args.pdf).unwrap_or_else(|e| {
            eprintln!("Error reading PDF: {}", e);
            process::exit(2);
        })) {
            Ok(doc) => match serde_json::to_string_pretty(&doc) {
                Ok(output) => {
                    println!("{}", output);
                    process::exit(0);
                }
                Err(e) => {
                    eprintln!("Error serializing document: {}", e);
                    process::exit(2);
                }
            },
            Err(e) => {
                eprintln!("Error extracting document: {}", e);
                process::exit(2);
            }
        }
    }

    let spec = match sp_check::spec::load_spec(&args.spec) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error loading spec: {}", e);
            process::exit(2);
        }
    };

    let options = sp_check::engine::CheckOptions {
        check_id: args.check.clone(),
        category: args.category.clone(),
    };

    let results = match sp_check::engine::run_checks(&spec, &args.pdf, &options) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Error running checks: {}", e);
            process::exit(2);
        }
    };

    let report = sp_check::report::build_report(results);

    if args.json {
        match sp_check::report::format_json(&report) {
            Ok(output) => println!("{}", output),
            Err(e) => {
                eprintln!("Error formatting JSON: {}", e);
                process::exit(2);
            }
        }
    } else if args.quiet {
        print!("{}", sp_check::report::format_text_quiet(&report));
    } else {
        println!("{}", sp_check::report::format_text(&report));
    }

    if report.summary.fail > 0 || report.summary.error > 0 {
        process::exit(1);
    }
}
