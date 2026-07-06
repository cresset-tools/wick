# cresset/wick

[wick](https://github.com/cresset-tools/wick) — an **unconfigurable**, Laravel
[Pint](https://laravel.com/docs/pint)-style PHP formatter — packaged for
Composer.

```bash
composer require --dev cresset/wick
vendor/bin/wick            # format every .php under the current dir
vendor/bin/wick --check    # CI: non-zero if anything is unformatted
vendor/bin/wick --diff     # show what would change
```

wick is a single Rust binary (powered by [Mago](https://github.com/carthage-software/mago)).
This package ships **only a thin PHP launcher** — no Rust source. On first run
it downloads the prebuilt `wick` binary matching this package's version for
your platform, caches it (`$XDG_CACHE_HOME/wick/<version>/`), verifies its
SHA-256, and execs it. The package version maps 1:1 to the wick release:
`cresset/wick:0.2.1` runs `wick-v0.2.1`.

Prebuilt targets: Linux x86_64 (gnu/musl), macOS arm64, Windows x64. Intel
macOS and Linux arm64 are not currently shipped. `ext-curl` is recommended;
`ext-zip` is required on Windows.

The download layout is also declared machine-readably in composer.json under
`extra.bougie.native-binary` (spec 1 = the cargo-dist release layout). Tools
that install this package — [bougie](https://github.com/cresset-tools/bougie)'s
`bougie tool install cresset/wick` — use it to prefetch and SHA-256-verify the
binary into the same cache the launcher probes, so the tool works immediately
(and offline) instead of downloading on first run. Composer itself ignores the
block. It must stay in sync with `src/Launcher.php` and with the repo's
dist-workspace.toml target list.

Release archives are additionally signed with Sigstore (cosign keyless via
GitHub Actions OIDC; `<archive>.sig` bundle sidecars, logged in the Rekor
transparency log). The `sigstore` key in that block pins this repository as
the signing identity, and bougie verifies it fail-closed before caching a
prefetched binary — the composer tag is only published after signing
completes, so every Packagist version has signed binaries. The PHP launcher
itself verifies the SHA-256 sidecar only.

This is the Composer distribution branch of the wick repo — it is generated
from `packaging/composer/` on `main` and contains no application code of its
own. EUPL-1.2.
