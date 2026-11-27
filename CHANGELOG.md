# Changelog

## [0.1.1]

### Added

- A macOS Settings test field with sample keys for Telex, Simple Telex, and VNI.
- Menu bar indicators for missing Accessibility permission and keyboard capture restarts.

### Fixed

- `uwo` now produces `ươ`, and sequences such as `ruwouj` produce `rượu`.
- Capitalization no longer changes the horn placement in `thuở`.
- Clicking another field or switching apps clears the unfinished word so later keys act in the new field.

### Changed

- The macOS package takes its release version from Cargo. Settings shows the release version and build number.

[0.1.1]: https://github.com/unrealphong/VNKey/releases/tag/v0.1.1
