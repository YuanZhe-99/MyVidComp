import 'package:flutter/cupertino.dart' show CupertinoPageTransitionsBuilder;
import 'package:flutter/material.dart';

/// The two interface styles the user can choose between.
///
/// [expressive] is the default and approximates Material 3 Expressive at the
/// theme level. [material3] is the application's plain Material 3 theme.
enum AppUiStyle {
  /// The application's own Material 3 theme, unchanged.
  material3,

  /// The Material 3 theme with the Expressive layer on top.
  expressive,
}

/// The stored names of the styles, in the order the interface offers them.
const List<String> uiStyleChoices = ['material3', 'expressive'];

// AI-FUNC-SUMMARY: Turns a stored style name into the style; returns the matching style, or expressive for anything unknown; side effects: none.
AppUiStyle uiStyleFromName(String name) =>
    name == 'material3' ? AppUiStyle.material3 : AppUiStyle.expressive;

/// Builds the light and dark themes for either interface style.
class AppTheme {
  AppTheme._();

  /// The brand colour. One seed produces a matching light and dark palette.
  static const Color seedColor = Color(0xFF1D6FD0);

  static const Duration _morphDuration = Duration(milliseconds: 200);

  // AI-FUNC-SUMMARY: Builds the theme for one brightness and style; returns the theme; side effects: none; Notes: both styles share one colour scheme, expressive only adds shape, type weight and component details.
  static ThemeData build(
    Brightness brightness, [
    AppUiStyle style = AppUiStyle.expressive,
  ]) {
    final scheme = ColorScheme.fromSeed(
      seedColor: seedColor,
      brightness: brightness,
    );
    final base = ThemeData(
      colorScheme: scheme,
      useMaterial3: true,
      cardTheme: CardThemeData(
        elevation: 0,
        color: scheme.surfaceContainerLow,
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(20)),
      ),
      inputDecorationTheme: const InputDecorationTheme(
        border: OutlineInputBorder(),
        isDense: true,
      ),
    );
    return style == AppUiStyle.expressive ? _expressive(base) : base;
  }

  // AI-FUNC-SUMMARY: Builds a button style that is a pill at rest and a rounded square while pressed; returns the style; side effects: none.
  static ButtonStyle _morphingButtonStyle() => ButtonStyle(
    animationDuration: _morphDuration,
    shape: WidgetStateProperty.resolveWith(
      (states) => states.contains(WidgetState.pressed)
          ? RoundedRectangleBorder(borderRadius: BorderRadius.circular(12))
          : const StadiumBorder(),
    ),
  );

  // AI-FUNC-SUMMARY: Makes display, headline and title styles heavier; returns the text theme; side effects: none; Notes: weights only, sizes and line heights stay as they were.
  static TextTheme _emphasized(TextTheme text) {
    TextStyle? bold(TextStyle? s, FontWeight w) => s?.copyWith(fontWeight: w);
    return text.copyWith(
      displayLarge: bold(text.displayLarge, FontWeight.w500),
      displayMedium: bold(text.displayMedium, FontWeight.w500),
      displaySmall: bold(text.displaySmall, FontWeight.w500),
      headlineLarge: bold(text.headlineLarge, FontWeight.w600),
      headlineMedium: bold(text.headlineMedium, FontWeight.w600),
      headlineSmall: bold(text.headlineSmall, FontWeight.w600),
      titleLarge: bold(text.titleLarge, FontWeight.w600),
      titleMedium: bold(text.titleMedium, FontWeight.w600),
      titleSmall: bold(text.titleSmall, FontWeight.w600),
    );
  }

  // AI-FUNC-SUMMARY: Layers the Material 3 Expressive approximation onto a theme; returns the theme; side effects: none; Notes: theme level only, so colours and layout never change; the card shape and field density already in the base theme are kept.
  static ThemeData _expressive(ThemeData base) {
    final cs = base.colorScheme;
    final morph = _morphingButtonStyle();
    RoundedRectangleBorder rounded(double r) =>
        RoundedRectangleBorder(borderRadius: BorderRadius.circular(r));
    OutlineInputBorder field(Color color, [double width = 1]) =>
        OutlineInputBorder(
          borderRadius: BorderRadius.circular(12),
          borderSide: BorderSide(color: color, width: width),
        );
    return base.copyWith(
      textTheme: _emphasized(base.textTheme),
      filledButtonTheme: FilledButtonThemeData(style: morph),
      elevatedButtonTheme: ElevatedButtonThemeData(style: morph),
      outlinedButtonTheme: OutlinedButtonThemeData(style: morph),
      textButtonTheme: TextButtonThemeData(style: morph),
      iconButtonTheme: IconButtonThemeData(style: morph),
      segmentedButtonTheme: SegmentedButtonThemeData(
        style: ButtonStyle(animationDuration: _morphDuration),
      ),
      floatingActionButtonTheme: FloatingActionButtonThemeData(
        shape: rounded(20),
      ),
      dialogTheme: DialogThemeData(shape: rounded(32)),
      bottomSheetTheme: const BottomSheetThemeData(
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.vertical(top: Radius.circular(32)),
        ),
      ),
      popupMenuTheme: PopupMenuThemeData(shape: rounded(16)),
      menuTheme: MenuThemeData(
        style: MenuStyle(shape: WidgetStatePropertyAll(rounded(16))),
      ),
      chipTheme: ChipThemeData(shape: rounded(12)),
      snackBarTheme: SnackBarThemeData(
        behavior: SnackBarBehavior.floating,
        shape: rounded(16),
      ),
      inputDecorationTheme: base.inputDecorationTheme.copyWith(
        border: field(cs.outline),
        enabledBorder: field(cs.outline),
        focusedBorder: field(cs.primary, 2),
        errorBorder: field(cs.error),
        focusedErrorBorder: field(cs.error, 2),
        disabledBorder: field(cs.onSurface.withValues(alpha: 0.12)),
      ),
      // `year2023: false` is the only opt-in to the 2024 indicator and slider
      // designs; it is deprecated only because false will become the default.
      // ignore: deprecated_member_use
      progressIndicatorTheme: const ProgressIndicatorThemeData(year2023: false),
      // ignore: deprecated_member_use
      sliderTheme: const SliderThemeData(year2023: false),
      pageTransitionsTheme: const PageTransitionsTheme(
        builders: {
          TargetPlatform.android: FadeForwardsPageTransitionsBuilder(),
          TargetPlatform.fuchsia: FadeForwardsPageTransitionsBuilder(),
          TargetPlatform.linux: FadeForwardsPageTransitionsBuilder(),
          TargetPlatform.windows: FadeForwardsPageTransitionsBuilder(),
          TargetPlatform.iOS: CupertinoPageTransitionsBuilder(),
          TargetPlatform.macOS: CupertinoPageTransitionsBuilder(),
        },
      ),
    );
  }

  // AI-FUNC-SUMMARY: Returns the light theme for a style; returns the theme; side effects: none.
  static ThemeData light([AppUiStyle style = AppUiStyle.expressive]) =>
      build(Brightness.light, style);

  // AI-FUNC-SUMMARY: Returns the dark theme for a style; returns the theme; side effects: none.
  static ThemeData dark([AppUiStyle style = AppUiStyle.expressive]) =>
      build(Brightness.dark, style);
}
