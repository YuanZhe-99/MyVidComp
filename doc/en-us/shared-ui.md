# Shared UI foundations

## Settings controls

The GUI pins MyApps-UI v0.1.4. ChoiceField delegates rendering to
MyAppsSettingsChoice while retaining its public constructor, labels, help and
disabled behavior. The app retains its 340-pixel/three-option segment policy and
AppText catalog. Rust settings and persistence remain app-owned. The extraction is
complete; library concept docs replace the completed roadmap.

MyApps-UI `v0.1.2` is embedded at repository-root `packages/myapps_ui`, using
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

## P2 navigation and actual space

The application now delegates navigation rendering to `MyAppsNavigationShell`.
App-side shells retain routes, destination filtering, selection persistence and reminder
callbacks. Each page passes `context` to its width and bottom-inset helpers: measured
shell content width is used once, and full-window routes subtract no rail. The legacy
context-free helper remains for callers that explicitly request the old calculation.
The stable content slot preserves page state across resize, style and rail-side changes.
MyVidComp retains classic navigation, extended rails and review badges.

Profile extraction is complete in P3; data formats are unchanged.

## P3 profile and avatar

MyVidComp pins the P3 shared release without adding a profile dependency or UI.
The application still consumes theme and navigation only.
