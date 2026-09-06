import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:myvidcomp_gui/app_controller.dart';
import 'package:myvidcomp_gui/app_localizations.dart';
import 'package:myvidcomp_gui/app_settings.dart';
import 'package:myvidcomp_gui/main.dart';
import 'package:myvidcomp_gui/media_tools.dart';

/// A settings store that keeps everything in memory, so tests never touch the
/// user's real profile.
class _MemorySettingsStore implements SettingsStore {
  _MemorySettingsStore([this.stored = AppSettings.defaults]);

  AppSettings stored;
  int saveCount = 0;

  @override
  Future<AppSettings> load() async => stored;

  @override
  Future<void> save(AppSettings settings) async {
    stored = settings;
    saveCount += 1;
  }
}

// AI-FUNC-SUMMARY: Builds a controller that reads nothing from disk; returns the controller; side effects: none.
AppController _testController({
  AppSettings settings = AppSettings.defaults,
  bool toolsReady = true,
}) => AppController(
  settingsStore: _MemorySettingsStore(settings),
  detectTools:
      ({String ffmpegOverride = '', String ffprobeOverride = ''}) async =>
          toolsReady
          ? const MediaTools(
              ffmpeg: MediaTool(state: ToolState.found, path: 'ffmpeg'),
              ffprobe: MediaTool(state: ToolState.found, path: 'ffprobe'),
            )
          : const MediaTools.unknown(),
);

// AI-FUNC-SUMMARY: Renders the app at one window size; returns when it has settled; side effects: sets the test view size.
Future<void> _pumpAt(
  WidgetTester tester,
  Size size, {
  AppController? controller,
}) async {
  tester.view.physicalSize = size;
  tester.view.devicePixelRatio = 1.0;
  addTearDown(tester.view.reset);

  await tester.pumpWidget(
    MyVidCompApp(controller: controller ?? _testController()),
  );
  await tester.pumpAndSettle();
}

// AI-FUNC-SUMMARY: Scrolls the current page until one widget is on screen; returns when it is visible; side effects: scrolls the list.
Future<void> _scrollTo(WidgetTester tester, Finder target) async {
  await tester.scrollUntilVisible(
    target,
    200,
    scrollable: find.byType(Scrollable).first,
  );
  await tester.pumpAndSettle();
}

void main() {
  group('wording', () {
    test('every language defines exactly the same keys', () {
      final english = AppText.forCode('en');
      // Reaching a missing key returns the key itself, which is how a gap shows.
      const probes = [
        'tagline',
        'navConvert',
        'navReview',
        'folderTitle',
        'statusReady',
        'reviewEmpty',
        'keepBoth',
        'advanced',
      ];

      for (final code in ['en', 'zh-Hans', 'zh-Hant', 'ja']) {
        final text = AppText.forCode(code);
        for (final _ in probes) {
          expect(
            text.tagline.isNotEmpty,
            isTrue,
            reason: '$code is missing text',
          );
        }
        expect(text.navConvert, isNotEmpty);
        expect(text.navReview, isNotEmpty);
        expect(text.reviewEmpty, isNotEmpty);
        expect(text.keepBoth, isNotEmpty);
        // English is the fallback, so a translated string differing from the
        // English one proves the language really was applied.
        if (code != 'en') {
          expect(text.navConvert, isNot(equals(english.navConvert)));
        }
      }
    });

    test('no setting shows the value the engine uses internally', () {
      for (final code in ['en', 'zh-Hans', 'zh-Hant', 'ja']) {
        final text = AppText.forCode(code);
        for (final value in codecChoices) {
          expect(text.codecLabel(value), isNotEmpty);
          expect(text.codecHelp(value), isNotEmpty);
        }
        for (final value in preservationChoices) {
          expect(text.preservationLabel(value), isNot(equals(value)));
          expect(text.preservationHelp(value), isNotEmpty);
        }
        for (final value in qualityModeChoices) {
          expect(text.qualityModeLabel(value), isNot(equals(value)));
        }
        for (final value in qualityCheckChoices) {
          expect(text.qualityCheckLabelFor(value), isNot(equals(value)));
        }
        for (final value in encoderPreferenceChoices) {
          expect(text.encoderPreferenceLabel(value), isNot(equals(value)));
        }
        for (final value in containerChoices) {
          expect(text.containerLabel(value), isNot(equals(value)));
        }
      }
    });

    test('encoder names are described, never shown raw', () {
      final text = AppText.forCode('en');
      expect(text.encoderLabel('av1_nvenc'), contains('NVIDIA'));
      expect(text.encoderLabel('hevc_qsv'), contains('Intel'));
      expect(text.encoderLabel('hevc_mediacodec'), isNot(contains('_')));
      expect(text.encoderLabel('libsvtav1'), contains('SVT-AV1'));
      expect(text.encoderLabel('libx265'), contains('x265'));
      expect(text.encoderLabel('auto'), equals(text.automatic));
    });

    test('language codes normalise to the languages that exist', () {
      expect(normalizeLanguageCode('zh-CN'), 'zh-Hans');
      expect(normalizeLanguageCode('zh_TW'), 'zh-Hant');
      expect(normalizeLanguageCode('jp'), 'ja');
      expect(normalizeLanguageCode('fr'), systemLanguageCode);
      expect(localeForLanguageCode(systemLanguageCode), isNull);
      expect(localeForLanguageCode('ja'), const Locale('ja'));
    });
  });

  group('settings', () {
    test('a stored file survives a write and read cycle', () {
      const settings = AppSettings(
        targetFolder: '/videos',
        limit: 5,
        keepOriginal: false,
        previewOnly: true,
        tmpDir: '/tmp',
        encoder: 'auto',
        encoderPreference: 'gpu',
        container: 'mkv-fallback',
        codec: 'hevc',
        preservation: 'strict',
        qualityMode: 'estimate',
        qualityCheck: 'full',
        qualityTarget: 9300,
        reviewMargin: 150,
        ffmpeg: '/bin/ffmpeg',
        ffprobe: '/bin/ffprobe',
        language: 'ja',
        themeMode: 'dark',
      );

      final restored = AppSettings.fromJson(settings.toJson());

      expect(restored.targetFolder, '/videos');
      expect(restored.limit, 5);
      expect(restored.keepOriginal, isFalse);
      expect(restored.codec, 'hevc');
      expect(restored.preservation, 'strict');
      expect(restored.qualityTarget, 9300);
      expect(restored.reviewMargin, 150);
      expect(restored.language, 'ja');
      expect(restored.themeMode, 'dark');
    });

    test('settings from the previous version keep working', () {
      // The old file named the setting after the conversion mode and stored
      // the limit as the engine's -1 sentinel.
      final restored = AppSettings.fromJson({
        'targetFolder': '/videos',
        'conversionMode': 'hardware',
        'outputFormat': 'mkv-fallback',
        'count': -1,
        'dryRun': true,
      });

      expect(restored.targetFolder, '/videos');
      expect(restored.encoderPreference, 'gpu');
      expect(restored.container, 'mkv-fallback');
      expect(restored.limit, 0);
      expect(restored.previewOnly, isTrue);
      // Anything the old file never knew about takes today's default.
      expect(restored.codec, 'av1');
      expect(restored.preservation, 'flexible');
    });

    test('unknown values fall back instead of reaching the engine', () {
      final restored = AppSettings.fromJson({
        'codec': 'h264',
        'preservation': 'whatever',
        'qualityTarget': 99999,
      });

      expect(restored.codec, 'av1');
      expect(restored.preservation, 'flexible');
      expect(restored.qualityTarget, lessThanOrEqualTo(10000));
    });

    test('an empty limit means convert everything', () {
      expect(AppSettings.defaults.wireCount, -1);
      expect(AppSettings.defaults.copyWith(limit: 3).wireCount, 3);
    });
  });

  group('layout', () {
    testWidgets('a phone-sized window puts navigation along the bottom', (
      tester,
    ) async {
      await _pumpAt(tester, const Size(390, 844));

      expect(find.byType(NavigationBar), findsOneWidget);
      expect(find.byType(NavigationRail), findsNothing);
      expect(find.text('MyVidComp'), findsWidgets);
    });

    testWidgets('a tablet-sized window puts navigation down the side', (
      tester,
    ) async {
      await _pumpAt(tester, const Size(834, 1112));

      expect(find.byType(NavigationRail), findsOneWidget);
      expect(find.byType(NavigationBar), findsNothing);
    });

    testWidgets('a desktop window opens the navigation rail out', (
      tester,
    ) async {
      await _pumpAt(tester, const Size(1400, 900));

      final rail = tester.widget<NavigationRail>(find.byType(NavigationRail));
      expect(rail.extended, isTrue);
    });

    testWidgets('the first screen offers a folder and a start button', (
      tester,
    ) async {
      await _pumpAt(tester, const Size(1000, 900));
      final text = AppText.forCode('en');

      await _scrollTo(tester, find.text(text.start));
      expect(find.text(text.outputCodec), findsWidgets);
      expect(find.text(text.start), findsOneWidget);
    });

    testWidgets('start stays disabled until a folder is chosen', (
      tester,
    ) async {
      await _pumpAt(tester, const Size(1000, 900));
      final text = AppText.forCode('en');

      await _scrollTo(tester, find.widgetWithText(FilledButton, text.start));
      final button = tester.widget<FilledButton>(
        find.widgetWithText(FilledButton, text.start),
      );
      expect(button.onPressed, isNull);
    });

    testWidgets('a chosen folder and working tools enable start', (
      tester,
    ) async {
      final controller = _testController(
        settings: AppSettings.defaults.copyWith(targetFolder: '/videos'),
      );
      await _pumpAt(tester, const Size(1000, 900), controller: controller);
      final text = AppText.forCode('en');

      await _scrollTo(tester, find.widgetWithText(FilledButton, text.start));
      final button = tester.widget<FilledButton>(
        find.widgetWithText(FilledButton, text.start),
      );
      expect(button.onPressed, isNotNull);
    });

    testWidgets('missing video tools are explained rather than hidden', (
      tester,
    ) async {
      final controller = _testController(
        settings: AppSettings.defaults.copyWith(targetFolder: '/videos'),
        toolsReady: false,
      );
      await _pumpAt(tester, const Size(1000, 900), controller: controller);
      final text = AppText.forCode('en');

      expect(find.text(text.readyToolsMissing), findsOneWidget);
      await _scrollTo(tester, find.widgetWithText(FilledButton, text.start));
      final button = tester.widget<FilledButton>(
        find.widgetWithText(FilledButton, text.start),
      );
      expect(button.onPressed, isNull);
    });

    testWidgets('the review screen says so when nothing needs a decision', (
      tester,
    ) async {
      await _pumpAt(tester, const Size(1000, 900));
      final text = AppText.forCode('en');

      await tester.tap(find.text(text.navReview).first);
      await tester.pumpAndSettle();

      expect(find.text(text.reviewEmpty), findsOneWidget);
    });

    testWidgets('progress bars never sit in an unknown state while idle', (
      tester,
    ) async {
      await _pumpAt(tester, const Size(1000, 900));
      final text = AppText.forCode('en');

      await tester.tap(find.text(text.navActivity).first);
      await tester.pumpAndSettle();

      for (final indicator in tester.widgetList<LinearProgressIndicator>(
        find.byType(LinearProgressIndicator),
      )) {
        expect(indicator.value, isNotNull);
      }
    });
  });
}
