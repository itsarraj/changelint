use std::path::PathBuf;
use std::process::ExitCode;

use changelint::changelog;
use changelint::generate::scaffold;
use changelint::git::list_tags_with_dates;
use changelint::validate::{lint, Severity};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "changelint",
    about = "Validates a CHANGELOG.md against the Keep a Changelog spec, and scaffolds a skeleton from git tags"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Lint a CHANGELOG.md (default command if none is given).
    Lint {
        #[arg(default_value = "CHANGELOG.md")]
        file: PathBuf,
    },
    /// Print a fresh Keep a Changelog skeleton, one section per git tag.
    Scaffold {
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

fn run_lint(file: PathBuf) -> ExitCode {
    let text = match std::fs::read_to_string(&file) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("changelint: {}: {e}", file.display());
            return ExitCode::from(2);
        }
    };

    let doc = changelog::parse(&text);
    let issues = lint(&doc);

    if issues.is_empty() {
        println!(
            "changelint: {} looks good — no issues found",
            file.display()
        );
        return ExitCode::SUCCESS;
    }

    let mut error_count = 0;
    for issue in &issues {
        let label = match issue.severity {
            Severity::Error => {
                error_count += 1;
                "ERROR"
            }
            Severity::Warning => "WARNING",
        };
        match issue.line {
            Some(line) => println!("{}:{line}: [{label}] {}", file.display(), issue.message),
            None => println!("{}: [{label}] {}", file.display(), issue.message),
        }
    }

    println!(
        "\nchangelint: {} issue(s), {error_count} error(s)",
        issues.len()
    );
    if error_count > 0 {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

fn run_scaffold(output: Option<PathBuf>) -> ExitCode {
    let tags = match list_tags_with_dates() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("changelint: {e}");
            return ExitCode::from(2);
        }
    };

    let text = scaffold(&tags);
    match output {
        Some(path) => {
            if let Err(e) = std::fs::write(&path, &text) {
                eprintln!("changelint: writing {}: {e}", path.display());
                return ExitCode::from(2);
            }
            println!(
                "changelint: wrote {} ({} tag(s) found)",
                path.display(),
                tags.len()
            );
        }
        None => print!("{text}"),
    }
    ExitCode::SUCCESS
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command.unwrap_or(Command::Lint {
        file: PathBuf::from("CHANGELOG.md"),
    }) {
        Command::Lint { file } => run_lint(file),
        Command::Scaffold { output } => run_scaffold(output),
    }
}
