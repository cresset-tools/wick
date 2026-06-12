//! Target PHP version detection.
//!
//! This is the *one* thing wick adapts to your project, because it affects
//! what is even valid syntax (e.g. property hooks, `readonly`, enums). It is
//! not a style knob — there is still exactly one style.
//!
//! We read `require.php` from the nearest `composer.json` (searching upward
//! from the current directory) and pick the **lowest** PHP version the
//! constraint allows, so wick never emits syntax your declared floor can't
//! run. With no `composer.json` or no `require.php`, we fall back to the
//! latest version Mago supports.

use std::path::{Path, PathBuf};

use mago_php_version::PHPVersion;

/// Lowest PHP version Mago has a constant for (`PHP70`).
const FLOOR: (u32, u32) = (7, 0);
/// `PHPVersion::LATEST` is PHP 8.5; never resolve above what Mago knows.
const CEIL: (u32, u32) = (8, 5);

/// Resolve the target PHP version for code rooted at the current directory.
pub fn detect() -> PHPVersion {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    detect_from(&cwd)
}

fn detect_from(start: &Path) -> PHPVersion {
    let Some(constraint) = nearest_composer_php_constraint(start) else {
        return PHPVersion::LATEST;
    };
    match lowest_compatible(&constraint) {
        Some((major, minor)) => {
            let (major, minor) = clamp((major, minor));
            PHPVersion::new(major, minor, 0)
        }
        None => PHPVersion::LATEST,
    }
}

/// Walk upward from `start` looking for a `composer.json` with a
/// `require.php` string.
fn nearest_composer_php_constraint(start: &Path) -> Option<String> {
    let mut dir = Some(start);
    while let Some(d) = dir {
        let candidate = d.join("composer.json");
        if candidate.is_file()
            && let Ok(text) = std::fs::read_to_string(&candidate)
            && let Ok(json) = serde_json::from_str::<serde_json::Value>(&text)
            && let Some(php) = json
                .get("require")
                .and_then(|r| r.get("php"))
                .and_then(|p| p.as_str())
        {
            return Some(php.to_owned());
        }
        dir = d.parent();
    }
    None
}

/// The lowest `(major, minor)` satisfying a Composer version constraint.
///
/// Composer joins alternatives with `||` (OR) and conjuncts with whitespace
/// or `,` (AND). For an AND group the effective floor is the *highest* of its
/// terms' floors; across OR groups the floor is the *lowest*. Terms that set
/// only an upper bound (`<`, `<=`, `!=`) contribute no floor. Hyphenated
/// ranges (`7.4 - 8.2`) are uncommon for `require.php` and not special-cased.
fn lowest_compatible(constraint: &str) -> Option<(u32, u32)> {
    let mut overall: Option<(u32, u32)> = None;
    for group in constraint.replace("||", "|").split('|') {
        let mut group_floor: Option<(u32, u32)> = None;
        for term in group.split([',', ' ', '\t']).filter(|s| !s.is_empty()) {
            if let Some(v) = term_floor(term) {
                group_floor = Some(group_floor.map_or(v, |cur| cur.max(v)));
            }
        }
        if let Some(g) = group_floor {
            overall = Some(overall.map_or(g, |cur| cur.min(g)));
        }
    }
    overall
}

/// The lower bound a single constraint term imposes, if any.
fn term_floor(term: &str) -> Option<(u32, u32)> {
    let term = term.trim();
    // Upper-bound-only / exclusion operators impose no floor.
    if term.starts_with('<') || term.starts_with('!') {
        return None;
    }
    // Strip a leading range/comparison operator; `^`, `~`, `>=`, `>`, `=`
    // and an exact version all share the same floor: the stated version.
    let rest = term
        .trim_start_matches(['^', '~', '>', '=', 'v', 'V'])
        .trim();
    parse_floor(rest)
}

/// Parse `major[.minor]` from the head of `s`, treating wildcards as 0.
fn parse_floor(s: &str) -> Option<(u32, u32)> {
    let mut parts = s.split('.');
    let major: u32 = parts.next()?.trim().parse().ok()?;
    let minor = parts
        .next()
        .map(|m| {
            m.trim()
                .chars()
                .take_while(|c| c.is_ascii_digit())
                .collect::<String>()
                .parse()
                .unwrap_or(0)
        })
        .unwrap_or(0);
    Some((major, minor))
}

fn clamp(v: (u32, u32)) -> (u32, u32) {
    v.max(FLOOR).min(CEIL)
}

#[cfg(test)]
mod tests {
    use super::lowest_compatible as low;
    use super::*;

    #[test]
    fn detects_lowest_from_composer_json() {
        use std::fs;
        let dir = std::env::temp_dir().join(format!("wick-detect-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);

        fs::write(dir.join("composer.json"), r#"{"require":{"php":"^8.1"}}"#).unwrap();
        assert_eq!(detect_from(&dir), PHPVersion::new(8, 1, 0));

        fs::write(
            dir.join("composer.json"),
            r#"{"require":{"php":"^7.4 || ^8.0"}}"#,
        )
        .unwrap();
        assert_eq!(detect_from(&dir), PHPVersion::new(7, 4, 0));

        // No require.php -> latest Mago supports.
        fs::write(dir.join("composer.json"), r#"{"name":"a/b"}"#).unwrap();
        assert_eq!(detect_from(&dir), PHPVersion::LATEST);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn caret_and_tilde() {
        assert_eq!(low("^8.1"), Some((8, 1)));
        assert_eq!(low("~8.0"), Some((8, 0)));
        assert_eq!(low("^7.4"), Some((7, 4)));
    }

    #[test]
    fn comparisons_and_wildcards() {
        assert_eq!(low(">=8.2"), Some((8, 2)));
        assert_eq!(low(">8.0"), Some((8, 0)));
        assert_eq!(low("8.1.*"), Some((8, 1)));
        assert_eq!(low("8.2"), Some((8, 2)));
    }

    #[test]
    fn or_takes_lowest_alternative() {
        assert_eq!(low("^7.4 || ^8.0"), Some((7, 4)));
        assert_eq!(low("^8.0|^8.2"), Some((8, 0)));
    }

    #[test]
    fn and_takes_highest_floor() {
        assert_eq!(low(">=8.0 <8.4"), Some((8, 0)));
        assert_eq!(low(">=8.1,<9.0"), Some((8, 1)));
    }

    #[test]
    fn upper_bound_only_has_no_floor() {
        assert_eq!(low("<8.4"), None);
        assert_eq!(low("*"), None);
    }
}
