import 'package:flutter/material.dart';
import 'package:myapps_ui/myapps_ui.dart';

export 'package:myapps_ui/myapps_ui.dart' show AppUiStyle;

/// The stored names of the styles, in the order the interface offers them.
const List<String> uiStyleChoices = ['material3', 'expressive'];

// AI-FUNC-SUMMARY: Turns a stored style name into the style; returns the matching style, or expressive for anything unknown; side effects: none.
AppUiStyle uiStyleFromName(String name) =>
    name == 'material3' ? AppUiStyle.material3 : AppUiStyle.expressive;

/// Builds the light and dark themes for either interface style.
class AppTheme {
  // AI-FUNC-SUMMARY: Purpose: Prevent facade instantiation; Inputs: none; Returns: none; Side effects: none; Notes: static API retained.
  AppTheme._();

  /// The brand colour. One seed produces a matching light and dark palette.
  static const Color seedColor = Color(0xFF1D6FD0);

  // AI-FUNC-SUMMARY: Purpose: Build the shared theme with the app brand; Inputs: brightness and style; Returns: ThemeData; Side effects: None; Notes: shared defaults own all component styling.
  static ThemeData build(
    Brightness brightness, [
    AppUiStyle style = AppUiStyle.expressive,
  ]) {
    return const MyAppsTheme(
      seedColor: seedColor,
    ).build(brightness, null, style);
  }

  // AI-FUNC-SUMMARY: Returns the light theme for a style; returns the theme; side effects: none.
  static ThemeData light([AppUiStyle style = AppUiStyle.expressive]) =>
      build(Brightness.light, style);

  // AI-FUNC-SUMMARY: Returns the dark theme for a style; returns the theme; side effects: none.
  static ThemeData dark([AppUiStyle style = AppUiStyle.expressive]) =>
      build(Brightness.dark, style);
}
