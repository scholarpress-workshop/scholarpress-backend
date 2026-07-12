use clap::Parser;
use std::path::PathBuf;
use std::process;

#[derive(Parser)]
pub struct CalibrateArgs {
    #[arg(short, long, help = "Path to institution spec YAML file")]
    pub spec: PathBuf,

    #[arg(short, long, help = "Path to corpus directory containing PDF files")]
    pub corpus: PathBuf,

    #[arg(short, long, help = "Output results as JSON")]
    pub json: bool,
}

pub fn run(args: &CalibrateArgs) {
    if !args.corpus.exists() {
        eprintln!(
            "Error: corpus directory not found: {}",
            args.corpus.display()
        );
        process::exit(2);
    }

    let cal_report =
        match sp_validate::calibration::run_calibration(&args.spec, &args.corpus) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Error: {}", e);
                process::exit(2);
            }
        };

    if args.json {
        match sp_validate::calibration::format_json(&cal_report) {
            Ok(output) => println!("{}", output),
            Err(e) => {
                eprintln!("Error: {}", e);
                process::exit(2);
            }
        }
    } else {
        println!("{}", sp_validate::calibration::format_text(&cal_report));
    }

    if cal_report.automated_fail_count() > 0 {
        process::exit(1);
    }
}
