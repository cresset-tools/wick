//! The single point of contact with the formatting engine.
//!
//! wick is deliberately unconfigurable: there is exactly one style (Mago's
//! `Pint` preset, which mirrors Laravel Pint's conventions) and one target
//! PHP version (the latest Mago supports). Everything that makes wick
//! "wick" rather than "mago format" lives in the constants below.

use std::borrow::Cow;

use bumpalo::Bump;
use mago_formatter::Formatter;
use mago_formatter::presets::FormatterPreset;
use mago_php_version::PHPVersion;

/// The one and only style. Mago names its Laravel/Pint preset `Pint`; it
/// sorts imports, prefers single quotes, drops spaces around `.`, adds a
/// blank line before `return`, spaces `fn (`, etc. — matching Pint's
/// out-of-the-box Laravel conventions.
const STYLE: FormatterPreset = FormatterPreset::Pint;

/// Format `source` as if it lived at `name` (the name only affects parse
/// diagnostics), targeting `php_version`. Returns the formatted source, or a
/// parse error message.
///
/// This is a from-scratch reprint: original whitespace is discarded and the
/// AST is printed anew, exactly like `gofmt`, Prettier, or Black.
pub fn format_php(name: &str, source: &str, php_version: PHPVersion) -> Result<String, String> {
    let arena = Bump::new();
    let formatter = Formatter::new(&arena, php_version, STYLE.settings());

    let name: Cow<'static, [u8]> = Cow::Owned(name.as_bytes().to_vec());
    let code: Cow<'static, [u8]> = Cow::Owned(source.as_bytes().to_vec());

    match formatter.format_code(name, code) {
        Ok(bytes) => Ok(String::from_utf8_lossy(bytes).into_owned()),
        Err(error) => Err(error.to_string()),
    }
}
