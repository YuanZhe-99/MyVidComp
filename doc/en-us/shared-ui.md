# Shared UI foundations

MyVidComp consumes MyApps-UI v0.1.7 from `packages/myapps_ui`. The Flutter dependency
uses `../packages/myapps_ui/packages/myapps_ui`; initialize submodules recursively.
The Rust engine has no UI-package dependency.

## Theme and navigation

`AppTheme.build` delegates to `MyAppsTheme.build`, supplying the blue brand seed.
Cards, fields, typography and Expressive details use the shared defaults. Stored
style names and the default Expressive style remain compatible.
`HomeShell.build` passes the selected style to `MyAppsNavigationShell`, using its
floating Expressive bottom bar and default compact rail geometry. Navigation moves
to a rail at 600 pixels; destinations, review badges and page state remain app-owned.

## Settings and choices

`SettingsPage.build` uses `MyAppsSettingsSection` and complete
`MyAppsSettingsSegmentRow` controls for appearance, style and language.
`MyAppsPaneBody` separates common settings from advanced controls when the actual
content width is at least 900 pixels, with minimum pane widths of 400 pixels.
Narrow layouts include advanced controls in the primary scrolling list. Stable
keys preserve text inputs and expansion state when the layout changes.
`ChoiceField.build` retains its app API and delegates to `MyAppsSettingsChoice`,
with no width or option-count dropdown gate. Shared segments wrap labels to two
lines and fall back to vertical options when necessary, including scaled text.
`SectionCard.build` uses shared section headings within theme-owned cards.
`LabelledField.build` inherits shared input density and border styling.
`CountTile.build` uses theme-owned card shapes and semantic surface colors.
`PageBody.build` supplies readable scrolling bounds and shared 16-pixel page spacing.
Tool availability, review cards and progress logs use the same 16-pixel content
spacing. The specific encoder is displayed as a localized read-only settings row.
Conversion settings, localization, persistence and review semantics remain app-owned.

## Attribution and updating

Settings displays myapps_ui's source and GNU GPL v3 notice. No profile dependency
is required. Publish shared-library updates to both remotes before pinning a tagged
commit here. Validate Flutter and Rust checks, and update both documentation trees.
