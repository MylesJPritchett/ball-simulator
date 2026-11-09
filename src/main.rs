use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use ball_simulator::scenario::Scenario;
use ball_simulator::{pipeline, preview};

#[derive(Parser)]
#[command(version, about = "Generates satisfying ball-bounce Videos")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Generate one Video from a Scenario and Seed.
    Generate {
        /// Scenario file path, or the name of a file in `scenarios/` (without `.toml`).
        #[arg(long)]
        scenario: String,
        #[arg(long)]
        seed: u64,
        /// Folder that Video folders are written into.
        #[arg(long, default_value = "output")]
        out: PathBuf,
    },
    /// Open a live preview window that reloads the Scenario file when it changes.
    Preview {
        /// Scenario file path, or the name of a file in `scenarios/` (without `.toml`).
        #[arg(long)]
        scenario: String,
        #[arg(long, default_value_t = 1)]
        seed: u64,
    },
}

/// Accepts either a path to a Scenario file or a shipped Scenario name.
fn resolve_scenario_path(arg: &str) -> PathBuf {
    let path = Path::new(arg);
    if path.exists() {
        path.to_path_buf()
    } else {
        Path::new("scenarios").join(format!("{arg}.toml"))
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result: Result<PathBuf, Box<dyn std::error::Error>> = match cli.command {
        Command::Generate {
            scenario,
            seed,
            out,
        } => Scenario::from_file(&resolve_scenario_path(&scenario))
            .map_err(Into::into)
            .and_then(|s| pipeline::generate(&s, seed, &out).map_err(Into::into))
            .map(|generated| {
                for r in &generated.rejected {
                    eprintln!("Seed {} rejected by the Quality Gate: {}", r.seed, r.reason);
                }
                generated.dir
            }),
        Command::Preview { scenario, seed } => {
            return match preview::window::run(resolve_scenario_path(&scenario), seed) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("error: {e}");
                    ExitCode::FAILURE
                }
            };
        }
    };
    match result {
        Ok(dir) => {
            println!("{}", dir.display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
