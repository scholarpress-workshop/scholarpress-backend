use clap::Parser;
use std::path::{Path, PathBuf};
use std::process;

#[derive(Parser)]
pub struct AnnotateArgs {
    #[arg(short, long, help = "Path to dissertation PDF")]
    pub pdf: PathBuf,

    #[arg(short, long, help = "Output path (default: <input>-annotated.pdf)")]
    pub out: Option<PathBuf>,

    #[arg(long, help = "Path to a sp-check JSON report")]
    pub report: Option<PathBuf>,

    #[arg(
        short,
        long,
        help = "Path to institution spec YAML (runs checks inline)"
    )]
    pub spec: Option<PathBuf>,

    #[arg(
        long,
        help = "Run only this specific check (by check ID); with --spec only"
    )]
    pub check: Option<String>,

    #[arg(
        short = 'C',
        long,
        help = "Run only checks in this category; with --spec only"
    )]
    pub category: Option<String>,
}

pub fn run(args: &AnnotateArgs) {
    if args.report.is_some() && (args.check.is_some() || args.category.is_some()) {
        eprintln!("Error: --check and --category require --spec");
        process::exit(2);
    }

    if !args.pdf.exists() {
        eprintln!("Error: PDF not found: {}", args.pdf.display());
        process::exit(2);
    }

    let report = match (&args.report, &args.spec) {
        (Some(_), Some(_)) => {
            eprintln!("Error: --report and --spec are mutually exclusive");
            process::exit(2);
        }
        (None, None) => {
            eprintln!("Error: one of --report or --spec is required");
            process::exit(2);
        }
        (Some(path), None) => {
            let json = std::fs::read_to_string(path).unwrap_or_else(|e| {
                eprintln!("Error reading report: {}", e);
                process::exit(2);
            });
            sp_annotate::parse_report(&json).unwrap_or_else(|e| {
                eprintln!("Error parsing report: {}", e);
                process::exit(2);
            })
        }
        (None, Some(spec_path)) => {
            let spec = sp_check::spec::load_spec(spec_path).unwrap_or_else(|e| {
                eprintln!("Error loading spec: {}", e);
                process::exit(2);
            });
            let options = sp_check::engine::CheckOptions {
                check_ids: args.check.clone().map(|id| vec![id]),
                category: args.category.clone(),
            };
            let results =
                sp_check::engine::run_checks(&spec, &args.pdf, &options).unwrap_or_else(|e| {
                    eprintln!("Error running checks: {}", e);
                    process::exit(2);
                });
            sp_check::report::build_report(results)
        }
    };

    let out = args.out.clone().unwrap_or_else(|| default_out(&args.pdf));
    if let Err(e) = sp_annotate::annotate_file(&args.pdf, &out, &report) {
        eprintln!("Error annotating: {}", e);
        process::exit(2);
    }
    println!("Wrote annotated PDF: {}", out.display());
}

fn default_out(pdf: &Path) -> PathBuf {
    let stem = pdf
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    pdf.with_file_name(format!("{stem}-annotated.pdf"))
}
