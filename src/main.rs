//! wick — an unconfigurable, Laravel Pint-style PHP formatter.
//!
//! There are no options that change the output. There is no `wick.toml`, no
//! `mago.toml`, no presets to pick. Like `gofmt`, you point it at your code
//! and it formats it. The only flags control *what* wick does with the
//! result (write, check, or diff), never *how* it formats.
//!
//! The formatting itself is done by Mago (https://github.com/carthage-software/mago),
//! a PHP toolchain written in Rust, via its `Pint` style preset. wick is a
//! thin opinionated front-end; all credit for the hard part goes to Mago.

mod format;
mod php_version;

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::Parser;
use ignore::WalkBuilder;
use mago_php_version::PHPVersion;
use rayon::prelude::*;
use similar::{ChangeTag, TextDiff};

use crate::format::format_php;

/// An unconfigurable, Laravel Pint-style PHP formatter. Powered by Mago.
///
/// By default, wick formats the given files in place. Pass directories to
/// format every `.php` file beneath them (respecting `.gitignore`). Pass `-`
/// to read from stdin and write the result to stdout.
#[derive(Parser)]
#[command(name = "wick", version, about, long_about = None)]
struct Cli {
    /// Files or directories to format. Defaults to the current directory.
    /// Use `-` to read PHP from stdin and write it to stdout.
    #[arg(value_name = "PATH")]
    paths: Vec<PathBuf>,

    /// Don't write anything; exit non-zero if any file is not already
    /// formatted. Ideal for CI.
    #[arg(long, conflicts_with = "diff")]
    check: bool,

    /// Don't write anything; print a diff of what would change and exit
    /// non-zero if anything would.
    #[arg(long)]
    diff: bool,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    // Resolved once: the lowest PHP version our composer.json allows (or the
    // latest Mago supports). Not a style knob — see `php_version`.
    let php_version = php_version::detect();

    // stdin mode: `wick -` (and only `-`).
    if cli.paths.len() == 1 && cli.paths[0].as_os_str() == "-" {
        return run_stdin(&cli, php_version);
    }

    let roots = if cli.paths.is_empty() {
        vec![PathBuf::from(".")]
    } else {
        cli.paths
    };

    let files = collect_php_files(&roots);
    if files.is_empty() {
        eprintln!("wick: no PHP files found");
        return ExitCode::SUCCESS;
    }

    // Format every file in parallel (each `format_php` is independent — its
    // own arena, no shared state), then report in stable file order so diffs
    // and messages never interleave. rayon uses all cores by default.
    let outcomes: Vec<Outcome> = files
        .par_iter()
        .map(|path| process_file(path, cli.check, cli.diff, php_version))
        .collect();

    let mut changed = 0usize;
    let mut errors = 0usize;
    for (path, outcome) in files.iter().zip(&outcomes) {
        match outcome {
            Outcome::Unchanged => {}
            Outcome::Changed(message) => {
                changed += 1;
                if let Some(message) = message {
                    print!("{message}");
                }
            }
            Outcome::Failed(message) => {
                errors += 1;
                eprintln!("error: {}: {message}", path.display());
            }
        }
    }

    report(files.len(), changed, errors, cli.check, cli.diff);

    if errors > 0 || ((cli.check || cli.diff) && changed > 0) {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

/// What happened to one file. Carries any text to print so the parallel pass
/// stays pure and `main` emits output in deterministic order.
enum Outcome {
    Unchanged,
    /// Changed (or would change). `Some` text is printed verbatim in order
    /// (a `--diff` hunk or a `--check` line); `None` means it was written.
    Changed(Option<String>),
    Failed(String),
}

fn process_file(path: &Path, check: bool, diff: bool, php_version: PHPVersion) -> Outcome {
    let original = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) => return Outcome::Failed(error.to_string()),
    };
    let formatted = match format_php(&path.to_string_lossy(), &original, php_version) {
        Ok(out) => out,
        Err(message) => return Outcome::Failed(message),
    };

    if formatted == original {
        return Outcome::Unchanged;
    }

    if diff {
        Outcome::Changed(Some(render_diff(&path.to_string_lossy(), &original, &formatted)))
    } else if check {
        Outcome::Changed(Some(format!("Would reformat: {}\n", path.display())))
    } else {
        match std::fs::write(path, &formatted) {
            Ok(()) => Outcome::Changed(None),
            Err(error) => Outcome::Failed(error.to_string()),
        }
    }
}

fn run_stdin(cli: &Cli, php_version: PHPVersion) -> ExitCode {
    let mut input = String::new();
    if std::io::stdin().read_to_string(&mut input).is_err() {
        eprintln!("error: failed to read stdin");
        return ExitCode::FAILURE;
    }

    let formatted = match format_php("<stdin>", &input, php_version) {
        Ok(out) => out,
        Err(message) => {
            eprintln!("error: {message}");
            return ExitCode::FAILURE;
        }
    };

    if cli.diff {
        print!("{}", render_diff("<stdin>", &input, &formatted));
        return if formatted == input {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        };
    }

    if cli.check {
        return if formatted == input {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        };
    }

    let _ = std::io::stdout().write_all(formatted.as_bytes());
    ExitCode::SUCCESS
}

/// wick only ever touches `.php` files — including ones passed explicitly
/// (so a pre-commit hook, editor, or shell glob handing wick a `composer.lock`
/// / `.json` / anything else leaves it untouched). Directories are walked
/// respecting `.gitignore`.
fn collect_php_files(roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for root in roots {
        if root.is_file() {
            if is_php(root) {
                files.push(root.clone());
            }
            continue;
        }
        for entry in WalkBuilder::new(root).build().flatten() {
            let path = entry.path();
            if path.is_file() && is_php(path) {
                files.push(path.to_path_buf());
            }
        }
    }
    files.sort();
    files.dedup();
    files
}

fn is_php(path: &Path) -> bool {
    path.extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("php"))
}

fn render_diff(name: &str, original: &str, formatted: &str) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    let _ = writeln!(out, "--- {name}");
    let _ = writeln!(out, "+++ {name} (formatted)");
    let diff = TextDiff::from_lines(original, formatted);
    for change in diff.iter_all_changes() {
        let sign = match change.tag() {
            ChangeTag::Delete => "-",
            ChangeTag::Insert => "+",
            ChangeTag::Equal => " ",
        };
        let _ = write!(out, "{sign}{change}");
    }
    out
}

fn report(total: usize, changed: usize, errors: usize, check: bool, diff: bool) {
    if check || diff {
        if changed == 0 {
            eprintln!("{total} file(s) already formatted");
        } else {
            eprintln!("{changed} of {total} file(s) would be reformatted");
        }
    } else if changed == 0 {
        eprintln!("{total} file(s) left unchanged");
    } else {
        eprintln!("Formatted {changed} file(s) ({total} checked)");
    }
    if errors > 0 {
        eprintln!("{errors} file(s) could not be parsed");
    }
}
