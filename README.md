# wick

An **unconfigurable**, Laravel [Pint](https://laravel.com/docs/pint)-style PHP
formatter — fast, single static binary, no PHP runtime required.

`wick` is to PHP what `gofmt` is to Go: one style, no knobs. There is no
config file, no preset selection, no rule toggles. You point it at your code
and it formats it. The only flags decide what to *do* with the result —
write, check, or diff — never *how* the code is formatted.

Part of the [cresset-tools](https://github.com/cresset-tools) family,
alongside [bougie](https://github.com/cresset-tools/bougie).

## Powered by Mago

wick does **not** parse or pretty-print PHP itself. All of that — the lexer,
parser, AST, and the Wadler-style pretty-printer that does the actual
formatting — is [**Mago**](https://github.com/carthage-software/mago), an
excellent PHP toolchain written in Rust by
[Carthage Software](https://carthage.software). wick simply pins Mago's
`Pint` style preset and the latest supported PHP version, and wraps them in a
deliberately minimal CLI.

If you want a configurable formatter, a linter, or a static analyzer, use Mago
directly — it does all of that and more. wick exists only for people who want
"Laravel style, no decisions."

Mago is licensed MIT OR Apache-2.0. wick is grateful for it.

## Install

```console
# Unix
$ curl --proto '=https' --tlsv1.2 -LsSf https://releases.bougie.tools/installers/wick/latest/wick-installer.sh | sh

# Windows (PowerShell)
> irm https://releases.bougie.tools/installers/wick/latest/wick-installer.ps1 | iex

# or, from source
$ cargo install wick
```

Prebuilt binaries (Linux gnu/musl, macOS arm64, Windows x64) are attached to
every [GitHub Release](https://github.com/cresset-tools/wick/releases) and
mirrored to cresset infrastructure.

## Usage

```console
$ wick                  # format every .php file under the current directory
$ wick src tests        # format specific files or directories
$ wick --check          # CI mode: exit non-zero if anything is unformatted
$ wick --diff           # print what would change, write nothing
$ cat a.php | wick -     # format stdin, write to stdout
```

Directories are walked respecting `.gitignore`.

## Compatibility note

wick is a from-scratch **reprinter**: it discards your existing whitespace and
prints the AST anew (like gofmt/Prettier/Black). Laravel Pint is a
**token-fixer** built on PHP-CS-Fixer that only edits what violates a rule.
The output style matches Pint's conventions closely, but wick is **not** a
byte-for-byte Pint drop-in — the first run on an existing Pint-formatted
codebase will still produce a reformat diff.

## License

EUPL-1.2 © Jelle Besseling
