# cresset-tools/wick

[wick](https://github.com/cresset-tools/wick) — an **unconfigurable**, Laravel
[Pint](https://laravel.com/docs/pint)-style PHP formatter — packaged for
Composer.

```bash
composer require --dev cresset-tools/wick
vendor/bin/wick            # format every .php under the current dir
vendor/bin/wick --check    # CI: non-zero if anything is unformatted
vendor/bin/wick --diff     # show what would change
```

wick is a single Rust binary (powered by [Mago](https://github.com/carthage-software/mago)).
This package ships **only a thin PHP launcher** — no Rust source. On first run
it downloads the prebuilt `wick` binary matching this package's version for
your platform, caches it (`$XDG_CACHE_HOME/wick/<version>/`), verifies its
SHA-256, and execs it. The package version maps 1:1 to the wick release:
`cresset-tools/wick:0.2.1` runs `wick-v0.2.1`.

Prebuilt targets: Linux x86_64 (gnu/musl), macOS arm64, Windows x64. Intel
macOS and Linux arm64 are not currently shipped. `ext-curl` is recommended;
`ext-zip` is required on Windows.

This is the Composer distribution branch of the wick repo — it is generated
from `packaging/composer/` on `main` and contains no application code of its
own. EUPL-1.2.
