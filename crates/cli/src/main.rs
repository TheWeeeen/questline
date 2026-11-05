//! `questline` command line: the same core the app will use, without the UI.
//!
//! M0 ships `plan`: read a catalogue file, sanitize it, build batches and print
//! exactly what a run would launch. That is enough to prove the safety rules and
//! the planner on real data, and it is what the CI smoke test runs.

mod catalogue;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};
use questline_core::{BatchPlanner, Platform};
use serde::Serialize;

#[derive(Parser)]
#[command(
    name = "questline",
    version,
    about = "Discord quest runner: plan and run game activity batches",
    disable_help_subcommand = true
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Build batches from a catalogue file and print what would be launched
    Plan(PlanArgs),
}

#[derive(clap::Args)]
struct PlanArgs {
    /// Path to a detectable-applications JSON file
    #[arg(long, value_name = "FILE")]
    catalogue: PathBuf,

    /// Games per batch
    #[arg(long, default_value_t = 8, value_name = "N")]
    batch: usize,

    /// Stop after this many games
    #[arg(long, value_name = "N")]
    limit: Option<usize>,

    /// Target platform rules
    #[arg(long, value_enum, default_value_t = PlatformArg::Host)]
    platform: PlatformArg,

    /// Machine-readable output
    #[arg(long)]
    json: bool,
}

#[derive(Copy, Clone, PartialEq, Eq, ValueEnum)]
enum PlatformArg {
    Host,
    Windows,
    Unix,
}

impl PlatformArg {
    fn resolve(self) -> Platform {
        match self {
            Self::Host => Platform::host(),
            Self::Windows => Platform::Windows,
            Self::Unix => Platform::Unix,
        }
    }
}

#[derive(Serialize)]
struct PlanOutput {
    platform: String,
    usable: usize,
    rejected: Vec<catalogue::Rejected>,
    without_executable: usize,
    batches: Vec<Vec<PlannedGame>>,
}

#[derive(Serialize)]
struct PlannedGame {
    id: String,
    name: String,
    exe: Option<String>,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Plan(args) => plan(&args),
    }
}

fn plan(args: &PlanArgs) -> ExitCode {
    let platform = args.platform.resolve();

    let entries = match catalogue::load_file(&args.catalogue) {
        Ok(entries) => entries,
        Err(err) => {
            eprintln!("error: {err}");
            return ExitCode::from(2);
        }
    };

    let mut catalogue = catalogue::parse_entries(&entries, platform);
    let usable = catalogue.games.len();

    let planner = match BatchPlanner::new(args.batch) {
        Ok(planner) => planner,
        Err(err) => {
            eprintln!("error: {err}");
            return ExitCode::from(2);
        }
    };

    let mut remaining = args.limit;
    let mut batches = Vec::new();
    while !catalogue.games.is_empty() {
        let batch = planner.plan(&mut catalogue.games, remaining);
        if batch.is_empty() {
            break;
        }
        if let Some(left) = remaining.as_mut() {
            *left = left.saturating_sub(batch.len());
        }
        batches.push(batch);
    }

    let rows: Vec<Vec<PlannedGame>> = batches
        .iter()
        .map(|batch| {
            batch
                .iter()
                .map(|game| PlannedGame {
                    id: game.id.as_str().to_owned(),
                    name: game.name.clone(),
                    exe: game.executable.clone(),
                })
                .collect()
        })
        .collect();

    if args.json {
        let output = PlanOutput {
            platform: platform_name(platform).to_owned(),
            usable,
            rejected: catalogue.rejected.clone(),
            without_executable: catalogue.without_executable,
            batches: rows,
        };
        match serde_json::to_string_pretty(&output) {
            Ok(json) => println!("{json}"),
            Err(err) => {
                eprintln!("error: cannot serialise plan: {err}");
                return ExitCode::from(2);
            }
        }
    } else {
        print_human(args, platform, &catalogue, usable, &rows);
    }

    if usable == 0 {
        eprintln!("error: no usable games in {}", args.catalogue.display());
        return ExitCode::from(2);
    }
    ExitCode::SUCCESS
}

fn print_human(
    args: &PlanArgs,
    platform: Platform,
    catalogue: &catalogue::Catalogue,
    usable: usize,
    batches: &[Vec<PlannedGame>],
) {
    println!(
        "questline {} — platform {}, batch {}, {} game(s) usable",
        env!("CARGO_PKG_VERSION"),
        platform_name(platform),
        args.batch,
        usable
    );
    println!(
        "catalogue {}: {} entries, {} without an executable for this platform",
        args.catalogue.display(),
        usable + catalogue.rejected.len() + catalogue.without_executable,
        catalogue.without_executable
    );

    for rejected in &catalogue.rejected {
        println!("  rejected {:?}: {}", rejected.name, rejected.reason);
    }

    // Deduplication can push a game into a later batch, so the plan's own length
    // is the only honest batch count to show.
    let total_batches = batches.len();
    for (index, batch) in batches.iter().enumerate() {
        println!("batch {}/{}:", index + 1, total_batches);
        for game in batch {
            println!(
                "  {:>3}. {:<32} {}",
                game.id,
                truncate(&game.name, 32),
                game.exe.as_deref().unwrap_or("-")
            );
        }
    }
}

fn platform_name(platform: Platform) -> &'static str {
    match platform {
        Platform::Windows => "windows",
        Platform::Unix => "unix",
    }
}

fn truncate(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_owned();
    }
    let kept: String = value.chars().take(max_chars.saturating_sub(1)).collect();
    format!("{kept}…")
}
