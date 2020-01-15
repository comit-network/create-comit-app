# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Fixed
- `cannot rename` error during start-env has been fixed.

## [0.8.1] - 2020-01-15

### Fixed
- Fix 404 error during start-env.

## [0.8.0] - 2020-01-10

### Changed
- The booting of a development environment, `start-env`, is now isolated to the `comit-scripts` package.
Which means that COMIT App should import dev dependency `comit-scripts` instead of `create-comit-app`.
See `../create/CHANGELOG.md` for historical changes. 

[Unreleased]: https://github.com/comit-network/create-comit-app/compare/comit-scripts-0.8.1...HEAD
[0.8.1]: https://github.com/comit-network/create-comit-app/compare/0.8.0...comit-scripts-0.8.1
[0.8.0]: https://github.com/comit-network/create-comit-app/compare/0.7.0...comit-scripts-0.8.0