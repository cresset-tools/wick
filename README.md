# wick

An **unconfigurable**, Laravel [Pint](https://laravel.com/docs/pint)-style PHP
formatter and linter — fast, single static binary, no PHP runtime required.

`wick` is to PHP what `gofmt` is to Go: one style, no knobs. There is no
config file, no preset selection, no rule toggles. You point it at your code
and it formats it. The only flags decide what to *do* with the result —
write, check, or diff — never *how* the code is formatted.

`wick check` adds linting in the same spirit: it runs Mago's default rule set,
unconfigured, at the PHP version your `composer.json` declares — no
`--select`, no `--ignore`, no severities to tune. It mirrors `ruff check`:
report by default, `--fix` to apply safe fixes, `--fix --unsafe-fixes` for the
rest, `--diff` to preview.

Part of the [cresset-tools](https://github.com/cresset-tools) family,
alongside [bougie](https://github.com/cresset-tools/bougie).

## Powered by Mago

wick does **not** parse or pretty-print PHP itself. All of that — the lexer,
parser, AST, and the Wadler-style pretty-printer that does the actual
formatting — is [**Mago**](https://github.com/carthage-software/mago), an
excellent PHP toolchain written in Rust by
[Carthage Software](https://carthage.software). wick simply pins Mago's
`Pint` style preset and default lint rules, targets the PHP version it detects,
and wraps them in a deliberately minimal CLI.

If you want a *configurable* formatter or linter, rule selection, severities,
framework integrations, or a static analyzer/type checker, use Mago directly —
it does all of that and more. wick exists only for people who want "Laravel
style, no decisions."

Mago is licensed MIT OR Apache-2.0. wick is grateful for it.

## Install

```console
# Unix
$ curl -LsSf https://bougie.tools/wick.sh | sh

# Windows (PowerShell)
> irm https://bougie.tools/wick.ps1 | iex

# or, from source
$ cargo install wick
```

Prebuilt binaries (Linux gnu/musl, macOS arm64, Windows x64) are attached to
every [GitHub Release](https://github.com/cresset-tools/wick/releases) and
mirrored to cresset infrastructure.

### In a PHP project (Composer)

wick is on [Packagist](https://packagist.org/packages/cresset/wick) as
`cresset/wick`. The Composer package ships only a thin PHP launcher; on first
run it downloads the prebuilt `wick` binary for your platform and caches it.

```console
$ composer require --dev cresset/wick
$ vendor/bin/wick format          # format every .php under the current dir
$ vendor/bin/wick format --check  # CI: non-zero if anything is unformatted
$ vendor/bin/wick check           # lint
```

### Without installing (bgx)

[bougie](https://bougie.tools)'s `bgx` (like npx) runs wick in an isolated,
globally-cached environment without adding it to your project. Everything after
the package is forwarded straight to wick:

```console
$ bgx cresset/wick format
$ bgx cresset/wick check src tests
```

## Usage

Like `ruff`, wick has no default action — pick `format` or `check`. Bare
`wick` just prints help.

```console
$ wick format                 # format every .php file under the current directory
$ wick format src tests       # format specific files or directories
$ wick format --check         # CI mode: exit non-zero if anything is unformatted
$ wick format --diff          # print what would change, write nothing
$ cat a.php | wick format -    # format stdin, write to stdout
```

Linting mirrors `ruff check`:

```console
$ wick check                  # lint every .php file under the current directory
$ wick check src tests        # lint specific files or directories
$ wick check --fix            # apply safe fixes in place
$ wick check --fix --unsafe-fixes   # also apply behaviour-changing fixes
$ wick check --diff           # preview the fixes --fix would apply
$ cat a.php | wick check -     # lint stdin
```

Fixable problems are flagged with `[*]`. Directories are walked respecting
`.gitignore`.

## Compatibility note

wick is a from-scratch **reprinter**: it discards your existing whitespace and
prints the AST anew (like gofmt/Prettier/Black). Laravel Pint is a
**token-fixer** built on PHP-CS-Fixer that only edits what violates a rule.
The output style matches Pint's conventions closely, but wick is **not** a
byte-for-byte Pint drop-in — the first run on an existing Pint-formatted
codebase will still produce a reformat diff.

## License

EUPL-1.2 © Jelle Besseling
