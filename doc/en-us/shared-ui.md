# Shared UI foundations

MyApps-UI `v0.1.0` is embedded at repository-root `packages/myapps_ui`, using
relative submodule URL `../MyApps-UI.git`. The Flutter application depends on
`../packages/myapps_ui/packages/myapps_ui` from `gui/pubspec.yaml`.
Initialize submodules recursively after cloning.

`gui/lib/app_theme.dart` re-exports `AppUiStyle` and keeps the existing theme
facade, seed, zero-elevation cards and dense fields. `MyAppsTheme.applyStyle`
supplies the Expressive overlay; the Material 3 base remains application-owned.
Stored style values and defaults, navigation and conversion behavior are unchanged.
The Rust engine does not depend on the UI package.

## Updating

Publish the library to both remotes before updating the app pointer. Pin its tagged
commit and validate Flutter and Rust checks. Shared theme declarations are documented
in the library; this repository documents its base theme and application behavior.
