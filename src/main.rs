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
mod lint;
mod php_version;

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};
use ignore::WalkBuilder;
use mago_php_version::PHPVersion;
use mago_text_edit::Safety;
use rayon::prelude::*;
use similar::{ChangeTag, TextDiff};

use crate::format::format_php;
use crate::lint::lint_php;

/// An unconfigurable, Laravel Pint-style PHP formatter and linter. Powered by
/// Mago.
///
/// Like `ruff`, wick has no default action — run `wick format` to format or
/// `wick check` to lint. Pass directories to walk every `.php` file beneath
/// them (respecting `.gitignore`), or `-` to read from stdin.
#[derive(Parser)]
#[command(name = "wick", version, about, long_about = None, arg_required_else_help = true)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Format `.php` files in place.
    Format(FormatArgs),
    /// Lint `.php` files with Mago's default rule set.
    Check(CheckArgs),
}

#[derive(Args)]
struct FormatArgs {
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

#[derive(Args)]
struct CheckArgs {
    /// Files or directories to lint. Defaults to the current directory.
    /// Use `-` to read PHP from stdin.
    #[arg(value_name = "PATH")]
    paths: Vec<PathBuf>,

    /// Apply safe fixes to the source automatically.
    #[arg(long)]
    fix: bool,

    /// Widen `--fix`/`--diff` to also include fixes that may change behaviour.
    #[arg(long)]
    unsafe_fixes: bool,

    /// Don't write anything; print a diff of the fixes that `--fix` would
    /// apply. Exits non-zero if anything would change.
    #[arg(long, conflicts_with = "fix")]
    diff: bool,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    // Resolved once: the lowest PHP version our composer.json allows (or the
    // latest Mago supports). Not a style knob — see `php_version`.
    let php_version = php_version::detect();

    match cli.command {
        Command::Check(args) => run_check(args, php_version),
        Command::Format(args) => run_format(args, php_version),
    }
}

fn run_format(args: FormatArgs, php_version: PHPVersion) -> ExitCode {
    // stdin mode: `wick -` (and only `-`).
    if args.paths.len() == 1 && args.paths[0].as_os_str() == "-" {
        return run_stdin(&args, php_version);
    }

    let roots = if args.paths.is_empty() {
        vec![PathBuf::from(".")]
    } else {
        args.paths.clone()
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
        .map(|path| process_file(path, args.check, args.diff, php_version))
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

    report(files.len(), changed, errors, args.check, args.diff);

    if errors > 0 || ((args.check || args.diff) && changed > 0) {
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
        Outcome::Changed(Some(render_diff(
            &path.to_string_lossy(),
            &original,
            &formatted,
            "formatted",
        )))
    } else if check {
        Outcome::Changed(Some(format!("Would reformat: {}\n", path.display())))
    } else {
        match std::fs::write(path, &formatted) {
            Ok(()) => Outcome::Changed(None),
            Err(error) => Outcome::Failed(error.to_string()),
        }
    }
}

fn run_stdin(args: &FormatArgs, php_version: PHPVersion) -> ExitCode {
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

    if args.diff {
        print!(
            "{}",
            render_diff("<stdin>", &input, &formatted, "formatted")
        );
        return if formatted == input {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        };
    }

    if args.check {
        return if formatted == input {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        };
    }

    let _ = std::io::stdout().write_all(formatted.as_bytes());
    ExitCode::SUCCESS
}

/// `wick check`: lint every file and report Ruff-style. With `--fix` the
/// resolved problems are rewritten in place; with `--diff` they're previewed.
fn run_check(args: CheckArgs, php_version: PHPVersion) -> ExitCode {
    // `--fix --unsafe-fixes` lowers the safety bar to `Unsafe` (which accepts
    // every tier); plain `--fix` and `--diff` stay at `Safe`. No flag means
    // "report only" — compute no fixes.
    let apply = if args.fix || args.diff {
        Some(if args.unsafe_fixes {
            Safety::Unsafe
        } else {
            Safety::Safe
        })
    } else {
        None
    };

    if args.paths.len() == 1 && args.paths[0].as_os_str() == "-" {
        return check_stdin(&args, php_version, apply);
    }

    let roots = if args.paths.is_empty() {
        vec![PathBuf::from(".")]
    } else {
        args.paths.clone()
    };

    let files = collect_php_files(&roots);
    if files.is_empty() {
        eprintln!("wick: no PHP files found");
        return ExitCode::SUCCESS;
    }

    // Lint in parallel (each `lint_php` owns its arena); print in stable file
    // order so diagnostics never interleave.
    let outcomes: Vec<CheckOutcome> = files
        .par_iter()
        .map(|path| check_file(path, args.fix, apply, php_version))
        .collect();

    let mut remaining = 0usize;
    let mut fixed = 0usize;
    let mut fixable = 0usize;
    let mut errors = 0usize;
    for (path, outcome) in files.iter().zip(&outcomes) {
        match outcome {
            CheckOutcome::Failed(message) => {
                errors += 1;
                eprintln!("error: {}: {message}", path.display());
            }
            CheckOutcome::Linted {
                diagnostics,
                fixed_count,
            } => {
                fixed += fixed_count;
                for diagnostic in diagnostics {
                    remaining += 1;
                    if diagnostic.fixable {
                        fixable += 1;
                    }
                    println!(
                        "{}:{}:{}: {}{} {}",
                        path.display(),
                        diagnostic.line,
                        diagnostic.column,
                        diagnostic.code,
                        if diagnostic.fixable { " [*]" } else { "" },
                        diagnostic.message,
                    );
                }
            }
        }
    }

    report_check(fixed, remaining, fixable, errors, args.fix);

    if errors > 0 || remaining > 0 || (args.diff && fixed > 0) {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

/// What happened to one file under `wick check`.
enum CheckOutcome {
    Linted {
        diagnostics: Vec<lint::Diagnostic>,
        fixed_count: usize,
    },
    Failed(String),
}

fn check_file(
    path: &Path,
    fix: bool,
    apply: Option<Safety>,
    php_version: PHPVersion,
) -> CheckOutcome {
    let original = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) => return CheckOutcome::Failed(error.to_string()),
    };

    let name = path.to_string_lossy();
    let outcome = lint_php(&name, &original, php_version, apply);

    // `--fix` writes the rewritten source; `--diff` only previews it.
    if let Some(fixed_source) = &outcome.fixed_source
        && fixed_source != &original
    {
        if fix {
            if let Err(error) = std::fs::write(path, fixed_source) {
                return CheckOutcome::Failed(error.to_string());
            }
        } else {
            print!("{}", render_diff(&name, &original, fixed_source, "fixed"));
        }
    }

    CheckOutcome::Linted {
        diagnostics: outcome.diagnostics,
        fixed_count: outcome.fixed_count,
    }
}

fn check_stdin(args: &CheckArgs, php_version: PHPVersion, apply: Option<Safety>) -> ExitCode {
    let mut input = String::new();
    if std::io::stdin().read_to_string(&mut input).is_err() {
        eprintln!("error: failed to read stdin");
        return ExitCode::FAILURE;
    }

    let outcome = lint_php("<stdin>", &input, php_version, apply);

    if args.diff {
        if let Some(fixed) = &outcome.fixed_source {
            print!("{}", render_diff("<stdin>", &input, fixed, "fixed"));
        }
    } else if args.fix {
        // Stream the fixed source (or the untouched input) to stdout.
        let result = outcome.fixed_source.as_deref().unwrap_or(&input);
        let _ = std::io::stdout().write_all(result.as_bytes());
    } else {
        for diagnostic in &outcome.diagnostics {
            println!(
                "<stdin>:{}:{}: {}{} {}",
                diagnostic.line,
                diagnostic.column,
                diagnostic.code,
                if diagnostic.fixable { " [*]" } else { "" },
                diagnostic.message,
            );
        }
    }

    if !outcome.diagnostics.is_empty() || (args.diff && outcome.fixed_count > 0) {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
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

fn render_diff(name: &str, original: &str, updated: &str, label: &str) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    let _ = writeln!(out, "--- {name}");
    let _ = writeln!(out, "+++ {name} ({label})");
    let diff = TextDiff::from_lines(original, updated);
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

fn report_check(fixed: usize, remaining: usize, fixable: usize, errors: usize, fix: bool) {
    if fix && fixed > 0 {
        eprintln!("Fixed {fixed} problem(s)");
    }

    if remaining == 0 {
        eprintln!("No problems found");
    } else {
        eprintln!("Found {remaining} problem(s)");
        // Only advertise the hint when fixes are still on the table.
        if !fix && fixable > 0 {
            eprintln!(
                "[*] {fixable} fixable with the `--fix` option (use `--fix --unsafe-fixes` for more)"
            );
        }
    }

    if errors > 0 {
        eprintln!("{errors} file(s) could not be read");
    }
}
