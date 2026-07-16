mod calibrate;
mod check;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "scholarpress")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "ScholarPress: format and validate scholarly documents")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run checks against a single dissertation PDF
    Check(check::CheckArgs),
    /// Run checks across a corpus of PDFs for calibration
    Calibrate(calibrate::CalibrateArgs),
}

fn main() {
    let cli = Cli::parse();
    match &cli.command {
        Commands::Check(args) => check::run(args),
        Commands::Calibrate(args) => calibrate::run(args),
    }
}
