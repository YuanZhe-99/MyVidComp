import 'dart:convert';
import 'dart:io';

/// Choices offered for each setting, in the order they appear in the interface.
/// The strings are the values the conversion engine understands.
const List<String> codecChoices = ['av1', 'hevc', 'vvc'];
const List<String> preservationChoices = ['flexible', 'strict'];
const List<String> qualityModeChoices = ['search', 'estimate'];
const List<String> qualityCheckChoices = ['sampled', 'full', 'off'];
const List<String> encoderPreferenceChoices = ['auto', 'gpu', 'cpu'];
const List<String> containerChoices = ['mp4', 'mkv-fallback'];

/// Quality presets, as VMAF scores in hundredths.
const List<int> qualityPresets = [9700, 9500, 9300, 9000];

/// Everything the interface remembers between runs.
class AppSettings {
  const AppSettings({
    required this.targetFolder,
    required this.limit,
    required this.keepOriginal,
    required this.previewOnly,
    required this.tmpDir,
    required this.encoder,
    required this.encoderPreference,
    required this.container,
    required this.codec,
    required this.preservation,
    required this.qualityMode,
    required this.qualityCheck,
    required this.qualityTarget,
    required this.reviewMargin,
    required this.ffmpeg,
    required this.ffprobe,
    required this.language,
    required this.themeMode,
  });

  /// Bumped whenever a stored file needs interpreting differently.
  static const int schemaVersion = 4;

  final String targetFolder;

  /// How many files one run may convert. Zero means every file.
  final int limit;
  final bool keepOriginal;
  final bool previewOnly;
  final String tmpDir;
  final String encoder;
  final String encoderPreference;
  final String container;
  final String codec;
  final String preservation;
  final String qualityMode;
  final String qualityCheck;

  /// Quality target in hundredths of a VMAF point.
  final int qualityTarget;

  /// How far below the target still avoids review, in hundredths.
  final int reviewMargin;

  final String ffmpeg;
  final String ffprobe;
  final String language;

  /// One of `system`, `light` or `dark`.
  final String themeMode;

  static const AppSettings defaults = AppSettings(
    targetFolder: '',
    limit: 0,
    keepOriginal: true,
    previewOnly: false,
    tmpDir: '',
    encoder: 'auto',
    encoderPreference: 'auto',
    container: 'mp4',
    codec: 'av1',
    preservation: 'flexible',
    qualityMode: 'search',
    qualityCheck: 'sampled',
    qualityTarget: 9500,
    reviewMargin: 200,
    ffmpeg: '',
    ffprobe: '',
    language: 'system',
    themeMode: 'system',
  );

  // AI-FUNC-SUMMARY: Produces a copy with selected fields replaced; returns the new settings; side effects: none.
  AppSettings copyWith({
    String? targetFolder,
    int? limit,
    bool? keepOriginal,
    bool? previewOnly,
    String? tmpDir,
    String? encoder,
    String? encoderPreference,
    String? container,
    String? codec,
    String? preservation,
    String? qualityMode,
    String? qualityCheck,
    int? qualityTarget,
    int? reviewMargin,
    String? ffmpeg,
    String? ffprobe,
    String? language,
    String? themeMode,
  }) => AppSettings(
    targetFolder: targetFolder ?? this.targetFolder,
    limit: limit ?? this.limit,
    keepOriginal: keepOriginal ?? this.keepOriginal,
    previewOnly: previewOnly ?? this.previewOnly,
    tmpDir: tmpDir ?? this.tmpDir,
    encoder: encoder ?? this.encoder,
    encoderPreference: encoderPreference ?? this.encoderPreference,
    container: container ?? this.container,
    codec: codec ?? this.codec,
    preservation: preservation ?? this.preservation,
    qualityMode: qualityMode ?? this.qualityMode,
    qualityCheck: qualityCheck ?? this.qualityCheck,
    qualityTarget: qualityTarget ?? this.qualityTarget,
    reviewMargin: reviewMargin ?? this.reviewMargin,
    ffmpeg: ffmpeg ?? this.ffmpeg,
    ffprobe: ffprobe ?? this.ffprobe,
    language: language ?? this.language,
    themeMode: themeMode ?? this.themeMode,
  );

  /// The engine reads -1 as "no limit"; the interface shows an empty box.
  int get wireCount => limit <= 0 ? -1 : limit;

  // AI-FUNC-SUMMARY: Reads settings from a stored document, filling in anything missing; returns the settings; side effects: none.
  static AppSettings fromJson(Map<String, Object?> json) {
    String pick(String key, List<String> allowed, String fallback) {
      final value = json[key];
      return value is String && allowed.contains(value) ? value : fallback;
    }

    int number(String key, int fallback) {
      final value = json[key];
      if (value is int) {
        return value;
      }
      if (value is String) {
        return int.tryParse(value) ?? fallback;
      }
      return fallback;
    }

    String text(String key, String fallback) {
      final value = json[key];
      return value is String ? value : fallback;
    }

    bool flag(String key, bool fallback) {
      final value = json[key];
      return value is bool ? value : fallback;
    }

    return AppSettings(
      targetFolder: text('targetFolder', defaults.targetFolder),
      // Older files stored the limit as the engine's -1 sentinel.
      limit: number('limit', number('count', 0)).clamp(-1, 1 << 30) < 0
          ? 0
          : number('limit', number('count', 0)),
      keepOriginal: flag('keepOriginal', defaults.keepOriginal),
      previewOnly: flag('previewOnly', flag('dryRun', defaults.previewOnly)),
      tmpDir: text('tmpDir', defaults.tmpDir),
      encoder: text('encoder', defaults.encoder),
      encoderPreference: pick(
        'encoderPreference',
        encoderPreferenceChoices,
        // The setting used to be called the conversion mode.
        _migrateConversionMode(json['conversionMode']),
      ),
      container: pick(
        'container',
        containerChoices,
        pick('outputFormat', containerChoices, defaults.container),
      ),
      codec: pick('codec', codecChoices, defaults.codec),
      preservation: pick(
        'preservation',
        preservationChoices,
        defaults.preservation,
      ),
      qualityMode: pick(
        'qualityMode',
        qualityModeChoices,
        defaults.qualityMode,
      ),
      qualityCheck: pick(
        'qualityCheck',
        qualityCheckChoices,
        defaults.qualityCheck,
      ),
      qualityTarget: number(
        'qualityTarget',
        defaults.qualityTarget,
      ).clamp(5000, 10000),
      reviewMargin: number(
        'reviewMargin',
        defaults.reviewMargin,
      ).clamp(0, 5000),
      ffmpeg: text('ffmpeg', defaults.ffmpeg),
      ffprobe: text('ffprobe', defaults.ffprobe),
      language: text('language', defaults.language),
      themeMode: pick('themeMode', const [
        'system',
        'light',
        'dark',
      ], defaults.themeMode),
    );
  }

  // AI-FUNC-SUMMARY: Maps the retired conversion-mode value onto the encoder preference; returns the matching preference; side effects: none.
  static String _migrateConversionMode(Object? value) {
    if (value == 'hardware') {
      return 'gpu';
    }
    if (value == 'consistency') {
      return 'auto';
    }
    return defaults.encoderPreference;
  }

  // AI-FUNC-SUMMARY: Writes the settings out as a stored document; returns the map; side effects: none.
  Map<String, Object?> toJson() => {
    'schemaVersion': schemaVersion,
    'targetFolder': targetFolder,
    'limit': limit,
    'keepOriginal': keepOriginal,
    'previewOnly': previewOnly,
    'tmpDir': tmpDir,
    'encoder': encoder,
    'encoderPreference': encoderPreference,
    'container': container,
    'codec': codec,
    'preservation': preservation,
    'qualityMode': qualityMode,
    'qualityCheck': qualityCheck,
    'qualityTarget': qualityTarget,
    'reviewMargin': reviewMargin,
    'ffmpeg': ffmpeg,
    'ffprobe': ffprobe,
    'language': language,
    'themeMode': themeMode,
  };
}

/// Reads and writes the settings file in the user's profile.
class SettingsStore {
  const SettingsStore();

  static const String _fileName = 'settings.json';

  /// Folder used before the app was renamed, read once so an existing install
  /// keeps its folder, tool paths and language.
  static const String _previousDirName = 'PVAC';
  static const String _previousDirNameLower = 'pvac';

  // AI-FUNC-SUMMARY: Loads stored settings, falling back to the previous version's file; returns the settings, or the defaults when nothing is stored; side effects: reads from disk.
  Future<AppSettings> load() async {
    final file = _settingsFile();
    if (await file.exists()) {
      return _read(file);
    }

    final legacy = _legacyFile();
    if (legacy != null && await legacy.exists()) {
      return _read(legacy);
    }

    return AppSettings.defaults;
  }

  // AI-FUNC-SUMMARY: Reads one settings file; returns the settings, or the defaults when it cannot be parsed; side effects: reads from disk.
  Future<AppSettings> _read(File file) async {
    try {
      final decoded = jsonDecode(await file.readAsString());
      if (decoded is Map<String, Object?>) {
        return AppSettings.fromJson(decoded);
      }
    } on FormatException {
      // A damaged file should not stop the app from starting.
    } on FileSystemException {
      // Same for one that cannot be read.
    }
    return AppSettings.defaults;
  }

  // AI-FUNC-SUMMARY: Writes the settings to the user's profile; returns when the write finishes; side effects: creates the folder and replaces the file.
  Future<void> save(AppSettings settings) async {
    final file = _settingsFile();
    await file.parent.create(recursive: true);
    await file.writeAsString(
      '${const JsonEncoder.withIndent('  ').convert(settings.toJson())}\n',
    );
  }

  // AI-FUNC-SUMMARY: Builds the path of the settings file; returns the file; side effects: reads environment variables.
  File _settingsFile() =>
      File('${_baseDir().path}${Platform.pathSeparator}$_fileName');

  // AI-FUNC-SUMMARY: Builds the path of the previous version's settings file; returns the file, or null when there is nowhere it could be; side effects: reads environment variables.
  File? _legacyFile() {
    final base = _baseDirNamed(_previousDirName, _previousDirNameLower);
    if (base == null) {
      return null;
    }
    return File('${base.path}${Platform.pathSeparator}gui-settings.json');
  }

  // AI-FUNC-SUMMARY: Resolves the folder this app stores settings in; returns the folder; side effects: reads environment variables.
  Directory _baseDir() =>
      _baseDirNamed('MyVidComp', 'myvidcomp') ?? Directory.current;

  // AI-FUNC-SUMMARY: Resolves a per-user settings folder by name for this platform; returns the folder, or null when the profile cannot be found; side effects: reads environment variables.
  Directory? _baseDirNamed(String name, String lowerName) {
    final separator = Platform.pathSeparator;

    if (Platform.isWindows) {
      final appData = Platform.environment['APPDATA'];
      if (appData != null && appData.isNotEmpty) {
        return Directory('$appData$separator$name');
      }
      return null;
    }

    final home =
        Platform.environment['HOME'] ?? Platform.environment['USERPROFILE'];
    if (home == null || home.isEmpty) {
      return null;
    }

    if (Platform.isMacOS) {
      return Directory(
        '$home${separator}Library${separator}Application Support$separator$name',
      );
    }
    return Directory('$home$separator.config$separator$lowerName');
  }
}
