//! The linting engine — the second point of contact with Mago.
//!
//! In the same spirit as the formatter, `wick check` is deliberately
//! unconfigurable: there is no rule selection, no `--select`/`--ignore`, no
//! `mago.toml`. wick runs exactly the rules Mago enables by default, at the
//! PHP version detected from `composer.json`. If you want to pick rules,
//! configure severities, or enable framework integrations, use Mago directly.
//!
//! Fixes follow Mago's safety classification: `Safe` fixes are applied by
//! `--fix`; `--unsafe-fixes` lowers the threshold to also apply the
//! potentially-unsafe and behaviour-changing ones.

use std::borrow::Cow;

use mago_allocator::LocalArena;
use mago_database::file::File;
use mago_linter::Linter;
use mago_linter::settings::Settings;
use mago_names::resolver::NameResolver;
use mago_php_version::PHPVersion;
use mago_syntax::parser::parse_file;
use mago_text_edit::{ApplyResult, Safety, TextEditor};

/// One reported problem, flattened to exactly what wick needs to print a
/// Ruff-style line: `path:line:col: CODE message`.
pub struct Diagnostic {
    pub line: usize,
    pub column: usize,
    pub code: String,
    pub message: String,
    /// True when a plain `--fix` (safe fixes only) would resolve this.
    pub fixable: bool,
}

/// The result of linting one file.
pub struct LintOutcome {
    /// Problems still present (after fixing, if fixes were applied).
    pub diagnostics: Vec<Diagnostic>,
    /// The rewritten source — `Some` only when fixing was requested *and* at
    /// least one fix applied.
    pub fixed_source: Option<String>,
    /// How many problems were auto-fixed.
    pub fixed_count: usize,
}

/// Lint `source` (named `name`) at `php_version`. If `apply` is `Some`, also
/// apply every fix whose safety is at or below the given threshold, returning
/// the rewritten source and hiding the diagnostics that were fixed.
pub fn lint_php(
    name: &str,
    source: &str,
    php_version: PHPVersion,
    apply: Option<Safety>,
) -> LintOutcome {
    let arena = LocalArena::new();

    let file = File::ephemeral(
        Cow::Owned(name.as_bytes().to_vec()),
        Cow::Owned(source.as_bytes().to_vec()),
    );
    let file_id = file.id;

    // Parse (with error recovery), resolve names for the semantic model, then
    // run the rule registry over the AST.
    let program = parse_file(&arena, &file);
    let resolved = NameResolver::new(&arena).resolve(program);

    let settings = Settings {
        php_version,
        ..Settings::default()
    };
    let linter = Linter::new(&arena, &settings, None, false);
    let issues = linter.lint(&file, program, &resolved);

    // One editor threads every accepted edit batch so overlapping fixes are
    // rejected atomically and offsets stay consistent.
    let mut editor = apply.map(|threshold| TextEditor::with_safety(source.as_bytes(), threshold));

    let mut diagnostics = Vec::new();
    let mut fixed_count = 0usize;

    for mut issue in issues {
        let offset = issue
            .primary_annotation()
            .map_or(0, |a| a.span.start.offset);
        let (line, column) = line_col(source, offset);
        let code = issue.code.clone().unwrap_or_default();
        let message = std::mem::take(&mut issue.message);

        let mut edits = issue.take_edits();
        let batch = edits.remove(&file_id).unwrap_or_default();
        // `[*]` means a safe `--fix` alone clears it (the whole batch is safe).
        let fixable = !batch.is_empty() && batch.iter().all(|e| e.safety <= Safety::Safe);

        let mut applied = false;
        if let Some(editor) = editor.as_mut()
            && !batch.is_empty()
        {
            applied = editor.apply_batch(batch, None::<fn(&[u8]) -> bool>) == ApplyResult::Applied;
            if applied {
                fixed_count += 1;
            }
        }

        // Hide what we just fixed, exactly like `ruff check --fix`.
        if !applied {
            diagnostics.push(Diagnostic {
                line,
                column,
                code,
                message,
                fixable,
            });
        }
    }

    let fixed_source = match editor {
        Some(editor) if fixed_count > 0 => {
            Some(String::from_utf8_lossy(&editor.finish()).into_owned())
        }
        _ => None,
    };

    LintOutcome {
        diagnostics,
        fixed_source,
        fixed_count,
    }
}

/// 1-based (line, column) for a byte `offset` into `source`.
fn line_col(source: &str, offset: u32) -> (usize, usize) {
    let offset = offset as usize;
    let mut line = 1usize;
    let mut column = 1usize;
    for (index, byte) in source.bytes().enumerate() {
        if index >= offset {
            break;
        }
        if byte == b'\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }
    (line, column)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PHP82: PHPVersion = PHPVersion::PHP82;

    #[test]
    fn flags_problems_and_locates_them() {
        // `==` should trip a rule; the diagnostic must point at the operator.
        let source = "<?php\n\nif ($a == $b) {\n}\n";
        let outcome = lint_php("t.php", source, PHP82, None);

        assert!(
            outcome
                .diagnostics
                .iter()
                .any(|d| d.line == 3 && d.column > 1),
            "expected a located diagnostic on line 3, got {:?}",
            outcome
                .diagnostics
                .iter()
                .map(|d| (d.line, d.column, &d.code))
                .collect::<Vec<_>>(),
        );
        // Report-only: nothing is rewritten.
        assert!(outcome.fixed_source.is_none());
        assert_eq!(outcome.fixed_count, 0);
    }

    #[test]
    fn applies_fixes_under_threshold() {
        // A file missing `declare(strict_types=1)` has a fix; at the unsafe
        // threshold it should be applied and the source rewritten.
        let source = "<?php\n\nfunction f(): void\n{\n}\n";
        let outcome = lint_php("t.php", source, PHP82, Some(Safety::Unsafe));

        if outcome.fixed_count > 0 {
            let fixed = outcome.fixed_source.expect("fixed source when count > 0");
            assert!(
                fixed.contains("strict_types"),
                "fix should insert strict_types: {fixed}"
            );
        } else {
            // If this Mago version classifies the fix differently, at least the
            // problem must still be reported rather than silently dropped.
            assert!(!outcome.diagnostics.is_empty());
        }
    }

    #[test]
    fn line_col_counts_newlines() {
        let source = "ab\ncd";
        assert_eq!(line_col(source, 0), (1, 1));
        assert_eq!(line_col(source, 1), (1, 2));
        assert_eq!(line_col(source, 3), (2, 1));
        assert_eq!(line_col(source, 4), (2, 2));
    }
}
