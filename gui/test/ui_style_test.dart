import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:myvidcomp_gui/app_localizations.dart';
import 'package:myvidcomp_gui/app_settings.dart';
import 'package:myvidcomp_gui/app_theme.dart';

// AI-FUNC-SUMMARY: Runs the interface style tests; returns none; side effects: none.
void main() {
  group('uiStyle setting', () {
    test('defaults to expressive', () {
      expect(AppSettings.defaults.uiStyle, 'expressive');
      expect(AppSettings.fromJson(const {}).uiStyle, 'expressive');
    });

    test('reads both stored values', () {
      expect(
        AppSettings.fromJson({'uiStyle': 'material3'}).uiStyle,
        'material3',
      );
      expect(
        AppSettings.fromJson({'uiStyle': 'expressive'}).uiStyle,
        'expressive',
      );
    });

    test('unknown or mistyped values fall back to the default', () {
      expect(AppSettings.fromJson({'uiStyle': 'bogus'}).uiStyle, 'expressive');
      expect(AppSettings.fromJson({'uiStyle': 3}).uiStyle, 'expressive');
      expect(AppSettings.fromJson({'uiStyle': null}).uiStyle, 'expressive');
    });

    test('round-trips through JSON and copyWith', () {
      final changed = AppSettings.defaults.copyWith(uiStyle: 'material3');
      expect(changed.uiStyle, 'material3');
      expect(changed.themeMode, AppSettings.defaults.themeMode);
      expect(AppSettings.fromJson(changed.toJson()).uiStyle, 'material3');
    });

    test('maps names onto the enum', () {
      expect(uiStyleFromName('material3'), AppUiStyle.material3);
      expect(uiStyleFromName('expressive'), AppUiStyle.expressive);
      expect(uiStyleFromName('unknown'), AppUiStyle.expressive);
      expect(uiStyleChoices, ['material3', 'expressive']);
    });

    test('every language labels both styles', () {
      for (final code in const ['en', 'zh-Hans', 'zh-Hant', 'ja']) {
        final text = AppText.forCode(code);
        expect(text.uiStyleTitle, isNotEmpty);
        for (final name in uiStyleChoices) {
          expect(text.uiStyleLabel(name), isNotEmpty);
          expect(text.uiStyleHelp(name), isNotEmpty);
        }
      }
    });
  });

  group('AppTheme', () {
    for (final brightness in Brightness.values) {
      test('styles share one colour scheme ($brightness)', () {
        final m3 = AppTheme.build(brightness, AppUiStyle.material3);
        final ex = AppTheme.build(brightness, AppUiStyle.expressive);
        expect(ex.colorScheme, m3.colorScheme);
        expect(ex.useMaterial3, isTrue);
        expect(m3.useMaterial3, isTrue);
      });
    }

    test('defaults to expressive', () {
      expect(
        AppTheme.light().dialogTheme.shape,
        AppTheme.light(AppUiStyle.expressive).dialogTheme.shape,
      );
      expect(AppTheme.dark().textTheme.titleLarge?.fontWeight, FontWeight.w600);
    });

    test('material3 uses shared component defaults', () {
      final m3 = AppTheme.light(AppUiStyle.material3);
      expect(m3.cardTheme.elevation, isNull);
      expect(m3.inputDecorationTheme.isDense, isFalse);
      expect(m3.inputDecorationTheme.border, const OutlineInputBorder());
      expect(m3.dialogTheme.shape, isNull);
      expect(m3.snackBarTheme.behavior, isNull);
      expect(m3.filledButtonTheme.style, isNull);
      expect(m3.textTheme.titleLarge?.fontWeight, isNot(FontWeight.w600));
    });

    test('expressive adds shape, weight and component details only', () {
      final m3 = AppTheme.light(AppUiStyle.material3);
      final ex = AppTheme.light(AppUiStyle.expressive);
      expect(ex.dialogTheme.shape, isNotNull);
      expect(ex.snackBarTheme.behavior, SnackBarBehavior.floating);
      expect(ex.filledButtonTheme.style, isNotNull);
      expect(ex.textTheme.titleLarge?.fontWeight, FontWeight.w600);
      expect(
        ex.textTheme.titleLarge?.fontSize,
        m3.textTheme.titleLarge?.fontSize,
      );
      expect(ex.textTheme.bodyMedium, m3.textTheme.bodyMedium);
      expect(ex.cardTheme.elevation, m3.cardTheme.elevation);
      expect(ex.cardTheme.color, m3.cardTheme.color);
      expect(ex.inputDecorationTheme.isDense, isFalse);
      expect(
        ex.pageTransitionsTheme.builders[TargetPlatform.windows],
        isA<FadeForwardsPageTransitionsBuilder>(),
      );
    });

    test('expressive buttons are pills at rest and square when pressed', () {
      final style = AppTheme.light().filledButtonTheme.style!;
      expect(style.shape!.resolve(<WidgetState>{}), isA<StadiumBorder>());
      expect(
        style.shape!.resolve(<WidgetState>{WidgetState.pressed}),
        isA<RoundedRectangleBorder>(),
      );
    });
  });
}
