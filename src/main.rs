use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use ball_simulator::pipeline::{self, Generated};
use ball_simulator::scenario::Scenario;
use ball_simulator::{batch, preview};

#[derive(Parser)]
#[command(version, about = "Generates satisfying ball-bounce Videos")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Generate one Video (`--scenario X --seed N`) or a batch (`--count N`).
    Generate {
        /// Scenario file path, or the name of a file in `scenarios/` (without `.toml`).
        #[arg(long, required_unless_present = "count", requires = "seed")]
        scenario: Option<String>,
        /// The Video's Seed; with `--count`, the batch Seed (default: from the clock).
        #[arg(long)]
        seed: Option<u64>,
        /// Number of Videos to make from Scenarios picked at random.
        #[arg(long, conflicts_with = "scenario")]
        count: Option<usize>,
        /// Folder of Scenario files a batch picks from.
        #[arg(long, default_value = "scenarios", conflicts_with = "scenario")]
        scenarios: PathBuf,
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

type CliResult = Result<(), Box<dyn std::error::Error>>;

/// Accepts either a path to a Scenario file or a shipped Scenario name.
fn resolve_scenario_path(arg: &str) -> PathBuf {
    let path = Path::new(arg);
    if path.exists() {
        path.to_path_buf()
    } else {
        Path::new("scenarios").join(format!("{arg}.toml"))
    }
}

fn report(scenario: &Scenario, generated: &Generated) {
    for r in &generated.rejected {
        eprintln!(
            "{}: Seed {} rejected by the Quality Gate: {}",
            scenario.id, r.seed, r.reason
        );
    }
    println!("{}", generated.dir.display());
}

fn generate_one(scenario: &str, seed: u64, out: &Path) -> CliResult {
    let scenario = Scenario::from_file(&resolve_scenario_path(scenario))?;
    let generated = pipeline::generate(&scenario, seed, out)?;
    report(&scenario, &generated);
    Ok(())
}

fn generate_batch(count: usize, batch_seed: Option<u64>, dir: &Path, out: &Path) -> CliResult {
    let batch_seed = batch_seed.unwrap_or_else(clock_seed);
    eprintln!(
        "Batch Seed {batch_seed} (rerun this batch with --count {count} --seed {batch_seed})"
    );
    let scenarios = batch::load_scenarios(dir)?;
    if scenarios.is_empty() {
        return Err(format!("no Scenario files in {}", dir.display()).into());
    }
    let plan = batch::plan(&scenarios, count, batch_seed);
    for failure in &plan.failures {
        eprintln!("skipping Scenario for the rest of the batch: {failure}");
    }
    let made = plan.videos.len();
    for video in plan.videos {
        let scenario = &scenarios[video.scenario];
        let generated = pipeline::render_accepted(scenario, video.accepted, out)?;
        report(scenario, &generated);
    }
    if made < count {
        return Err(format!(
            "made {made} of {count} Videos: every Scenario failed its Quality Gate"
        )
        .into());
    }
    Ok(())
}

/// A batch Seed for runs that don't pass one; printed so the batch can be rerun.
fn clock_seed() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos() as u64)
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.command {
        Command::Generate {
            scenario: Some(scenario),
            seed,
            out,
            ..
        } => generate_one(&scenario, seed.expect("clap requires --seed"), &out),
        Command::Generate {
            count,
            seed,
            scenarios,
            out,
            ..
        } => generate_batch(
            count.expect("clap requires --count"),
            seed,
            &scenarios,
            &out,
        ),
        Command::Preview { scenario, seed } => {
            preview::window::run(resolve_scenario_path(&scenario), seed).map_err(Into::into)
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
