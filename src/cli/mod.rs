//! Command-line interface.

mod commands;

use std::io::Write as _;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

/// Scores how easy TypeScript code is for a person to follow, and reports
/// how a change moved that score.
#[derive(Debug, Parser)]
#[command(name = "stem", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Measure the project and print a summary
    Scan {
        /// Folder to scan (default: the current folder)
        dir: Option<PathBuf>,
        /// Also write the snapshot to this file
        #[arg(short = 'o', long = "output", value_name = "file")]
        output: Option<PathBuf>,
        /// Print machine-readable JSON instead of text
        #[arg(long)]
        json: bool,
        /// Do not measure files matching this pattern (may repeat)
        #[arg(long = "ignore", value_name = "glob")]
        ignore: Vec<String>,
    },
    /// Re-measure and report what got easier or harder to follow
    Compare {
        /// Snapshot written earlier by `stem scan -o`
        baseline: PathBuf,
        /// Folder to scan (default: the current folder)
        dir: Option<PathBuf>,
        /// Print machine-readable JSON instead of text
        #[arg(long)]
        json: bool,
        /// Not accepted: compare always uses the baseline's config
        #[arg(long = "ignore", value_name = "glob", hide = true)]
        ignore: Vec<String>,
    },
    /// Explain a metric or the score
    Explain {
        /// cognitive-complexity, length, parameters, vocabulary, usage-gap or score
        topic: Option<String>,
    },
}

/// Exit status of a finished command.
pub enum Outcome {
    Pass,
    Regression,
}

/// Prints `text` to stdout, ignoring a closed pipe.
fn print(text: &str) {
    let mut stdout = std::io::stdout().lock();
    let _ = stdout.write_all(text.as_bytes());
    let _ = stdout.flush();
}

/// Runs `stem` with the process arguments.
pub fn main() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => {
            let code = if e.use_stderr() { 2 } else { 0 };
            let _ = e.print();
            return ExitCode::from(code);
        }
    };
    let result = match cli.command {
        Command::Scan {
            dir,
            output,
            json,
            ignore,
        } => commands::scan(dir, output, json, &ignore),
        Command::Compare {
            baseline,
            dir,
            json,
            ignore,
        } => commands::compare(&baseline, dir, json, &ignore),
        Command::Explain { topic } => commands::explain(topic.as_deref()),
    };
    match result {
        Ok((text, Outcome::Pass)) => {
            print(&text);
            ExitCode::SUCCESS
        }
        Ok((text, Outcome::Regression)) => {
            print(&text);
            ExitCode::from(1)
        }
        Err(e) => {
            eprintln!("stem: error: {e}");
            ExitCode::from(2)
        }
    }
}
