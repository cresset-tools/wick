# Changelog

## [0.3.0](https://github.com/cresset-tools/wick/compare/wick-v0.2.3...wick-v0.3.0) (2026-06-30)


### ⚠ BREAKING CHANGES

* **composer:** rename Packagist vendor to cresset/wick
* bare `wick <paths>` no longer formats. Use `wick format <paths>`. Update pre-commit hooks and CI accordingly.

### Features

* add `wick check` linter and require a subcommand ([#6](https://github.com/cresset-tools/wick/issues/6)) ([ece02c0](https://github.com/cresset-tools/wick/commit/ece02c057466e09d24b9d519d0cdddf3a915ecc3))
* **composer:** rename Packagist vendor to cresset/wick ([fbcc0b6](https://github.com/cresset-tools/wick/commit/fbcc0b66ecfdcfb8e3f5bf2cce6259345b940d82))


### Bug Fixes

* **composer:** re-add root composer.json so Packagist re-discovers cresset/wick ([5b7cbeb](https://github.com/cresset-tools/wick/commit/5b7cbeb889b5a220fc8f78b511deab6cb23e82e8))

## [0.2.3](https://github.com/cresset-tools/wick/compare/wick-v0.2.2...wick-v0.2.3) (2026-06-13)


### Bug Fixes

* **composer:** add root composer.json so Packagist can discover the package ([2881377](https://github.com/cresset-tools/wick/commit/2881377c20ca5a86b791fd6da8d5a063382cb72c))
* only format .php files; format in parallel ([8627fd9](https://github.com/cresset-tools/wick/commit/8627fd9a6ae6e2a6fb317e799753566dacd2f330))

## [0.2.2](https://github.com/cresset-tools/wick/compare/wick-v0.2.1...wick-v0.2.2) (2026-06-13)


### Bug Fixes

* **dist:** make installers prefer the mirror (hosting=[simple,github]) ([2c16d30](https://github.com/cresset-tools/wick/commit/2c16d303e8c8b0cd3f81c49d682bedfdd89a829f))

## [0.2.1](https://github.com/cresset-tools/wick/compare/wick-v0.2.0...wick-v0.2.1) (2026-06-13)


### Bug Fixes

* **dist:** define dist profile and build linux-gnu natively ([20abe86](https://github.com/cresset-tools/wick/commit/20abe861cb71cfd28f80b6ed7730f5584c4095c3))

## [0.2.0](https://github.com/cresset-tools/wick/compare/wick-v0.1.0...wick-v0.2.0) (2026-06-12)


### Features

* initial wick — unconfigurable Laravel Pint-style PHP formatter ([0f68352](https://github.com/cresset-tools/wick/commit/0f68352cb2169efe407869568e3212b1f661ccfc))

## Changelog

All notable changes to wick are documented here. This file is maintained by
[release-please](https://github.com/googleapis/release-please) from
conventional-commit messages — don't edit it by hand.
