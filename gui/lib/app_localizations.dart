import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';

import 'formatting.dart';

/// Sentinel meaning "use whatever language the device is set to".
const String systemLanguageCode = 'system';

/// Languages offered in the language picker, in the order shown.
const List<String> languageCodes = [
  systemLanguageCode,
  'en',
  'zh-Hans',
  'zh-Hant',
  'ja',
];

/// Text for the whole interface.
///
/// Every string a person reads lives here. Values the conversion engine uses
/// internally, such as `hevc` or `mkv-fallback`, are never shown directly; each
/// one has a label and usually a sentence explaining what choosing it means.
class AppText {
  const AppText(this.code);

  final String code;

  static const List<Locale> supportedLocales = [
    Locale('en'),
    Locale.fromSubtags(languageCode: 'zh', scriptCode: 'Hans'),
    Locale.fromSubtags(languageCode: 'zh', scriptCode: 'Hant'),
    Locale('ja'),
  ];

  static const LocalizationsDelegate<AppText> delegate = _AppTextDelegate();

  // AI-FUNC-SUMMARY: Reads the text for the active language; returns the text set; side effects: none.
  static AppText of(BuildContext context) =>
      Localizations.of<AppText>(context, AppText) ?? const AppText('en');

  // AI-FUNC-SUMMARY: Builds a text set for one language code; returns the text set; side effects: none.
  static AppText forCode(String code) => AppText(normalizeLanguageCode(code));

  // AI-FUNC-SUMMARY: Looks up one string, falling back to English and then to the key itself; returns the text; side effects: none.
  String _text(String key) =>
      _strings[code]?[key] ?? _strings['en']?[key] ?? key;

  // AI-FUNC-SUMMARY: Looks up one string and fills in its placeholders; returns the text; side effects: none.
  String _format(String key, Map<String, String> values) {
    var text = _text(key);
    values.forEach((name, value) {
      text = text.replaceAll('{$name}', value);
    });
    return text;
  }

  String get appName => 'MyVidComp';
  String get tagline => _text('tagline');

  String get navConvert => _text('navConvert');
  String get navActivity => _text('navActivity');
  String get navReview => _text('navReview');
  String get navSettings => _text('navSettings');

  String get folderTitle => _text('folderTitle');
  String get folderHint => _text('folderHint');
  String get chooseFolder => _text('chooseFolder');
  String get folderRequired => _text('folderRequired');

  String get outputCodec => _text('outputCodec');
  String get quality => _text('quality');
  String get howCareful => _text('howCareful');
  String get start => _text('start');
  String get stop => _text('stop');
  String get stopping => _text('stopping');
  String get previewOnly => _text('previewOnly');
  String get previewOnlyHelp => _text('previewOnlyHelp');

  String get readyToolsMissing => _text('readyToolsMissing');
  String get readyToolsFound => _text('readyToolsFound');
  String get toolsMissingHelp => _text('toolsMissingHelp');

  String get statusReady => _text('statusReady');
  String get statusStarting => _text('statusStarting');
  String get statusScanning => _text('statusScanning');
  String get statusConverting => _text('statusConverting');
  String get statusMeasuring => _text('statusMeasuring');
  String get statusTuning => _text('statusTuning');
  String get statusFinishing => _text('statusFinishing');
  String get statusDone => _text('statusDone');
  String get statusStopped => _text('statusStopped');
  String get statusFailed => _text('statusFailed');

  String get nothingToDo => _text('nothingToDo');
  String get currentFile => _text('currentFile');
  String get overallProgress => _text('overallProgress');
  String get converted => _text('converted');
  String get skipped => _text('skipped');
  String get failed => _text('failed');
  String get needsReview => _text('needsReview');
  String get spaceSaved => _text('spaceSaved');
  String get details => _text('details');
  String get noActivityYet => _text('noActivityYet');

  String get reviewTitle => _text('reviewTitle');
  String get reviewIntro => _text('reviewIntro');
  String get reviewEmpty => _text('reviewEmpty');
  String get reviewEmptyHelp => _text('reviewEmptyHelp');
  String get keepNew => _text('keepNew');
  String get keepOriginal => _text('keepOriginal');
  String get keepBoth => _text('keepBoth');
  String get keepNewHelp => _text('keepNewHelp');
  String get keepOriginalHelp => _text('keepOriginalHelp');
  String get keepBothHelp => _text('keepBothHelp');
  String get whatChanged => _text('whatChanged');
  String get qualityNotMeasured => _text('qualityNotMeasured');
  String get refresh => _text('refresh');
  String get applyToAll => _text('applyToAll');

  String get settingsOriginals => _text('settingsOriginals');
  String get keepOriginalsOn => _text('keepOriginalsOn');
  String get keepOriginalsOff => _text('keepOriginalsOff');
  String get settingsSpeed => _text('settingsSpeed');
  String get whichEncoder => _text('whichEncoder');
  String get container => _text('container');
  String get qualityStrategy => _text('qualityStrategy');
  String get qualityCheckLabel => _text('qualityCheckLabel');
  String get specificEncoder => _text('specificEncoder');
  String get specificEncoderHelp => _text('specificEncoderHelp');
  String get limitLabel => _text('limitLabel');
  String get limitHelp => _text('limitHelp');
  String get workingFolder => _text('workingFolder');
  String get workingFolderHelp => _text('workingFolderHelp');
  String get advanced => _text('advanced');
  String get mediaTools => _text('mediaTools');
  String get mediaToolsHelp => _text('mediaToolsHelp');
  String get language => _text('language');
  String get appearance => _text('appearance');
  String get themeSystem => _text('themeSystem');
  String get themeLight => _text('themeLight');
  String get themeDark => _text('themeDark');
  String get reviewMarginLabel => _text('reviewMarginLabel');
  String get reviewMarginHelp => _text('reviewMarginHelp');
  String get automatic => _text('automatic');
  String get browse => _text('browse');

  // AI-FUNC-SUMMARY: Names one output codec; returns the label; side effects: none.
  String codecLabel(String value) => switch (value) {
    'av1' => _text('codecAv1'),
    'hevc' => _text('codecHevc'),
    'vvc' => _text('codecVvc'),
    _ => value,
  };

  // AI-FUNC-SUMMARY: Explains what choosing one output codec means; returns the sentence; side effects: none.
  String codecHelp(String value) => switch (value) {
    'av1' => _text('codecAv1Help'),
    'hevc' => _text('codecHevcHelp'),
    'vvc' => _text('codecVvcHelp'),
    _ => '',
  };

  // AI-FUNC-SUMMARY: Names one preservation choice; returns the label; side effects: none.
  String preservationLabel(String value) => switch (value) {
    'flexible' => _text('preservationFlexible'),
    'strict' => _text('preservationStrict'),
    _ => value,
  };

  // AI-FUNC-SUMMARY: Explains what one preservation choice means; returns the sentence; side effects: none.
  String preservationHelp(String value) => switch (value) {
    'flexible' => _text('preservationFlexibleHelp'),
    'strict' => _text('preservationStrictHelp'),
    _ => '',
  };

  // AI-FUNC-SUMMARY: Names one quality strategy; returns the label; side effects: none.
  String qualityModeLabel(String value) => switch (value) {
    'search' => _text('qualityModeSearch'),
    'estimate' => _text('qualityModeEstimate'),
    _ => value,
  };

  // AI-FUNC-SUMMARY: Explains what one quality strategy means; returns the sentence; side effects: none.
  String qualityModeHelp(String value) => switch (value) {
    'search' => _text('qualityModeSearchHelp'),
    'estimate' => _text('qualityModeEstimateHelp'),
    _ => '',
  };

  // AI-FUNC-SUMMARY: Names one quality-check choice; returns the label; side effects: none.
  String qualityCheckLabelFor(String value) => switch (value) {
    'sampled' => _text('qualityCheckSampled'),
    'full' => _text('qualityCheckFull'),
    'off' => _text('qualityCheckOff'),
    _ => value,
  };

  // AI-FUNC-SUMMARY: Explains what one quality-check choice means; returns the sentence; side effects: none.
  String qualityCheckHelp(String value) => switch (value) {
    'sampled' => _text('qualityCheckSampledHelp'),
    'full' => _text('qualityCheckFullHelp'),
    'off' => _text('qualityCheckOffHelp'),
    _ => '',
  };

  // AI-FUNC-SUMMARY: Names one encoder preference; returns the label; side effects: none.
  String encoderPreferenceLabel(String value) => switch (value) {
    'auto' => _text('encoderAuto'),
    'gpu' => _text('encoderGpu'),
    'cpu' => _text('encoderCpu'),
    _ => value,
  };

  // AI-FUNC-SUMMARY: Explains what one encoder preference means; returns the sentence; side effects: none.
  String encoderPreferenceHelp(String value) => switch (value) {
    'auto' => _text('encoderAutoHelp'),
    'gpu' => _text('encoderGpuHelp'),
    'cpu' => _text('encoderCpuHelp'),
    _ => '',
  };

  // AI-FUNC-SUMMARY: Names one container choice; returns the label; side effects: none.
  String containerLabel(String value) => switch (value) {
    'mp4' => _text('containerMp4'),
    'mkv-fallback' => _text('containerMkvFallback'),
    _ => value,
  };

  // AI-FUNC-SUMMARY: Explains what one container choice means; returns the sentence; side effects: none.
  String containerHelp(String value) => switch (value) {
    'mp4' => _text('containerMp4Help'),
    'mkv-fallback' => _text('containerMkvFallbackHelp'),
    _ => '',
  };

  // AI-FUNC-SUMMARY: Names one quality preset by its score; returns the label; side effects: none.
  String qualityPresetLabel(int hundredths) => switch (hundredths) {
    9700 => _text('qualityHighest'),
    9500 => _text('qualityHigh'),
    9300 => _text('qualityBalanced'),
    9000 => _text('qualitySmallest'),
    _ => formatScore(hundredths),
  };

  // AI-FUNC-SUMMARY: Explains one quality preset; returns the sentence; side effects: none.
  String qualityPresetHelp(int hundredths) => switch (hundredths) {
    9700 => _text('qualityHighestHelp'),
    9500 => _text('qualityHighHelp'),
    9300 => _text('qualityBalancedHelp'),
    9000 => _text('qualitySmallestHelp'),
    _ => '',
  };

  // AI-FUNC-SUMMARY: Names one specific encoder in words a general reader understands; returns the label; side effects: none.
  String encoderLabel(String value) {
    if (value == 'auto') {
      return _text('automatic');
    }
    final maker = switch (true) {
      _ when value.contains('nvenc') => _text('makerNvidia'),
      _ when value.contains('qsv') => _text('makerIntel'),
      _ when value.contains('amf') => _text('makerAmd'),
      _ when value.contains('_mf') => _text('makerWindows'),
      _ when value.contains('videotoolbox') => _text('makerApple'),
      _ when value.contains('mediacodec') => _text('makerPhone'),
      _ when value.contains('vulkan') => _text('makerVulkan'),
      _ => '',
    };
    if (maker.isNotEmpty) {
      return _format('encoderGraphicsCard', {'maker': maker});
    }
    return _format('encoderProcessor', {'name': _encoderShortName(value)});
  }

  // AI-FUNC-SUMMARY: Shortens a software encoder name for display; returns the short name; side effects: none.
  String _encoderShortName(String value) => switch (value) {
    'libsvtav1' => 'SVT-AV1',
    'libaom-av1' => 'AOM',
    'librav1e' => 'rav1e',
    'libx265' => 'x265',
    'libvvenc' => 'VVenC',
    _ => value,
  };

  // AI-FUNC-SUMMARY: Names one language for the picker; returns the label; side effects: none.
  String languageLabel(String value) => switch (value) {
    systemLanguageCode => _text('languageSystem'),
    'en' => 'English',
    'zh-Hans' => '简体中文',
    'zh-Hant' => '繁體中文',
    'ja' => '日本語',
    _ => value,
  };

  // AI-FUNC-SUMMARY: Names one theme choice; returns the label; side effects: none.
  String themeLabel(String value) => switch (value) {
    'light' => themeLight,
    'dark' => themeDark,
    _ => themeSystem,
  };

  // AI-FUNC-SUMMARY: Describes how many files are queued; returns the sentence; side effects: none.
  String filesFound(int count) => _format('filesFound', {'count': '$count'});

  // AI-FUNC-SUMMARY: Describes progress through the queue; returns the sentence; side effects: none.
  String fileOfTotal(int index, int? total) => total == null
      ? _format('fileNoTotal', {'index': '$index'})
      : _format('fileOfTotal', {'index': '$index', 'total': '$total'});

  // AI-FUNC-SUMMARY: Reports a measured quality score; returns the sentence; side effects: none.
  String qualityScore(int hundredths) =>
      _format('qualityScore', {'score': formatScore(hundredths)});

  // AI-FUNC-SUMMARY: Reports how a converted file compares in size; returns the sentence; side effects: none.
  String sizeComparison(String before, String after, String change) => _format(
    'sizeComparison',
    {'before': before, 'after': after, 'change': change},
  );

  // AI-FUNC-SUMMARY: Reports the size of a review file relative to its original; returns the sentence; side effects: none.
  String percentOfOriginal(String percent) =>
      _format('percentOfOriginal', {'percent': percent});

  // AI-FUNC-SUMMARY: Reports how many conversions await a decision; returns the sentence; side effects: none.
  String reviewsWaiting(int count) =>
      _format('reviewsWaiting', {'count': '$count'});

  // AI-FUNC-SUMMARY: Reports that a file was left alone and why; returns the sentence; side effects: none.
  String skippedFile(String name, String reason) =>
      _format('skippedFile', {'name': name, 'reason': reason});

  // AI-FUNC-SUMMARY: Reports that one file could not be converted; returns the sentence; side effects: none.
  String failedFile(String name, String reason) =>
      _format('failedFile', {'name': name, 'reason': reason});

  // AI-FUNC-SUMMARY: Reports one step of a quality search; returns the sentence; side effects: none.
  String tryingSetting(String score) =>
      _format('tryingSetting', {'score': score});

  // AI-FUNC-SUMMARY: Reports how much longer something has to run; returns the phrase; side effects: none.
  String timeLeft(String duration) => _format('timeLeft', {'time': duration});

  // AI-FUNC-SUMMARY: Reports which trial of a quality search is running; returns the phrase; side effects: none.
  String trialOfTotal(int trial, int total) =>
      _format('trialOfTotal', {'trial': '$trial', 'total': '$total'});

  // AI-FUNC-SUMMARY: Reports a capability this installation lacks; returns the sentence; side effects: none.
  String featureUnavailable(String detail) =>
      _format('featureUnavailable', {'detail': detail});

  // AI-FUNC-SUMMARY: Reports that settings could not be stored; returns the sentence; side effects: none.
  String settingsProblem(String detail) =>
      _format('settingsProblem', {'detail': detail});
}

// AI-FUNC-SUMMARY: Formats a hundredths quality value for display; returns a one-decimal string; side effects: none.

// AI-FUNC-SUMMARY: Turns a stored language value into one the app supports; returns the language code; side effects: none.
String normalizeLanguageCode(String value) {
  final lowered = value.trim().toLowerCase().replaceAll('_', '-');
  return switch (lowered) {
    'en' => 'en',
    'zh' || 'zh-hans' || 'zh-cn' || 'zh-sg' => 'zh-Hans',
    'zh-hant' || 'zh-tw' || 'zh-hk' || 'zh-mo' => 'zh-Hant',
    'ja' || 'jp' => 'ja',
    _ => systemLanguageCode,
  };
}

// AI-FUNC-SUMMARY: Turns a language choice into a locale, or null to follow the device; returns the locale or null; side effects: none.
Locale? localeForLanguageCode(String value) => switch (normalizeLanguageCode(
  value,
)) {
  'en' => const Locale('en'),
  'zh-Hans' => const Locale.fromSubtags(languageCode: 'zh', scriptCode: 'Hans'),
  'zh-Hant' => const Locale.fromSubtags(languageCode: 'zh', scriptCode: 'Hant'),
  'ja' => const Locale('ja'),
  _ => null,
};

class _AppTextDelegate extends LocalizationsDelegate<AppText> {
  const _AppTextDelegate();

  @override
  // AI-FUNC-SUMMARY: Reports whether a locale has translations; returns true for the supported languages; side effects: none.
  bool isSupported(Locale locale) =>
      const {'en', 'zh', 'ja'}.contains(locale.languageCode);

  @override
  // AI-FUNC-SUMMARY: Builds the text set for a locale; returns it without waiting; side effects: none.
  Future<AppText> load(Locale locale) =>
      SynchronousFuture(AppText(_codeForLocale(locale)));

  @override
  // AI-FUNC-SUMMARY: Reports whether the text set must be rebuilt; returns false because it never changes; side effects: none.
  bool shouldReload(_AppTextDelegate old) => false;

  // AI-FUNC-SUMMARY: Chooses the text set for a device locale; returns the language code; side effects: none.
  String _codeForLocale(Locale locale) {
    if (locale.languageCode == 'ja') {
      return 'ja';
    }
    if (locale.languageCode == 'zh') {
      final traditional =
          locale.scriptCode == 'Hant' ||
          const {'TW', 'HK', 'MO'}.contains(locale.countryCode);
      return traditional ? 'zh-Hant' : 'zh-Hans';
    }
    return 'en';
  }
}

const Map<String, Map<String, String>> _strings = {
  'en': {
    'tagline': 'Make your videos smaller without losing what matters.',
    'navConvert': 'Convert',
    'navActivity': 'Progress',
    'navReview': 'Review',
    'navSettings': 'Settings',
    'folderTitle': 'Video folder',
    'folderHint': 'Pick the folder holding the videos you want to shrink.',
    'chooseFolder': 'Choose folder',
    'folderRequired': 'Choose a folder first.',
    'outputCodec': 'Video format',
    'quality': 'Quality',
    'howCareful': 'How careful to be',
    'start': 'Start',
    'stop': 'Stop',
    'stopping': 'Finishing this file, then stopping',
    'previewOnly': 'Preview only',
    'previewOnlyHelp': 'List what would change without converting anything.',
    'readyToolsMissing': 'Video tools not found',
    'readyToolsFound': 'Ready',
    'toolsMissingHelp':
        'MyVidComp needs the FFmpeg tools to convert video. Point to them under Settings.',
    'statusReady': 'Ready when you are',
    'statusStarting': 'Getting ready',
    'statusScanning': 'Looking through your folder',
    'statusConverting': 'Converting',
    'statusMeasuring': 'Checking quality',
    'statusTuning': 'Choosing the best settings',
    'statusFinishing': 'Finishing up',
    'statusDone': 'All done',
    'statusStopped': 'Stopped',
    'statusFailed': 'Something went wrong',
    'nothingToDo': 'No videos here need converting.',
    'currentFile': 'Working on',
    'overallProgress': 'All files',
    'timeLeft': '{time} left',
    'trialOfTotal': 'trial {trial} of at most {total}',
    'converted': 'Converted',
    'skipped': 'Left alone',
    'failed': 'Could not convert',
    'needsReview': 'Waiting for you',
    'spaceSaved': 'Space saved',
    'details': 'Details',
    'noActivityYet': 'Nothing has happened yet.',
    'reviewTitle': 'Your decision',
    'reviewIntro':
        'These conversions changed something small, or scored below your target. Both copies are still here.',
    'reviewEmpty': 'Nothing is waiting for you',
    'reviewEmptyHelp':
        'Conversions that need a decision will appear here. Everything else is handled automatically.',
    'keepNew': 'Keep the new one',
    'keepOriginal': 'Keep the original',
    'keepBoth': 'Keep both',
    'keepNewHelp': 'Replaces the original with the smaller file.',
    'keepOriginalHelp': 'Deletes the new file and leaves the original alone.',
    'keepBothHelp': 'Renames the new file and keeps the original too.',
    'whatChanged': 'What changed',
    'qualityNotMeasured': 'Quality not measured',
    'refresh': 'Refresh',
    'applyToAll': 'Apply to all',
    'settingsOriginals': 'Original files',
    'keepOriginalsOn': 'Keep originals',
    'keepOriginalsOff': 'Replace originals once verified',
    'settingsSpeed': 'Speed',
    'whichEncoder': 'Which hardware to use',
    'container': 'File type',
    'qualityStrategy': 'How to reach the quality target',
    'qualityCheckLabel': 'Quality check',
    'specificEncoder': 'Use one specific encoder',
    'specificEncoderHelp':
        'Leave on Automatic unless you have a reason to force one.',
    'limitLabel': 'Stop after this many files',
    'limitHelp': 'Leave empty to convert everything.',
    'workingFolder': 'Working folder',
    'workingFolderHelp':
        'Where temporary files go. Leave empty to keep them beside each video, which is fastest for network drives.',
    'advanced': 'Advanced',
    'mediaTools': 'Video tools',
    'mediaToolsHelp': 'Leave empty to use the tools that came with MyVidComp.',
    'language': 'Language',
    'appearance': 'Appearance',
    'themeSystem': 'Match my device',
    'themeLight': 'Light',
    'themeDark': 'Dark',
    'reviewMarginLabel': 'Ask me when quality falls below',
    'reviewMarginHelp':
        'How far under your target a result may land before MyVidComp keeps both copies.',
    'automatic': 'Automatic',
    'browse': 'Browse',
    'codecAv1': 'AV1',
    'codecHevc': 'H.265',
    'codecVvc': 'H.266',
    'codecAv1Help': 'Smallest files. Plays on most devices made since 2020.',
    'codecHevcHelp':
        'Slightly larger files, but plays on almost anything, including older phones and TVs.',
    'codecVvcHelp':
        'Experimental. Very slow to make and almost nothing can play it yet.',
    'preservationFlexible': 'Balanced',
    'preservationStrict': 'Exact',
    'preservationFlexibleHelp':
        'Converts more of your videos. When something small has to change, MyVidComp keeps both copies and asks you.',
    'preservationStrictHelp':
        'Converts only when every detail can be reproduced exactly. Some videos will be left untouched.',
    'qualityModeSearch': 'Measure and tune',
    'qualityModeEstimate': 'Quick estimate',
    'qualityModeSearchHelp':
        'Tests short samples of each video to find the smallest size that still looks right. Slower, better results.',
    'qualityModeEstimateHelp':
        'Guesses a setting from the original video. Much faster, less precise.',
    'qualityCheckSampled': 'Spot check',
    'qualityCheckFull': 'Every frame',
    'qualityCheckOff': 'Skip',
    'qualityCheckSampledHelp':
        'Compares one frame in five. Fast and accurate enough to trust.',
    'qualityCheckFullHelp':
        'Compares every frame. The most accurate, and the slowest.',
    'qualityCheckOffHelp':
        'No comparison. Conversions that changed something are still checked.',
    'encoderAuto': 'Balanced',
    'encoderGpu': 'Prefer graphics card',
    'encoderCpu': 'Processor only',
    'encoderAutoHelp':
        'Uses your graphics card when it can do the job properly, and the processor otherwise.',
    'encoderGpuHelp':
        'Fastest. Your graphics card is used even when it means a small change to the picture.',
    'encoderCpuHelp': 'Slowest, and usually produces the smallest files.',
    'containerMp4': 'MP4 only',
    'containerMkvFallback': 'MP4, or MKV when needed',
    'containerMp4Help':
        'The most compatible file type. Videos with unusual audio or subtitles are left alone.',
    'containerMkvFallbackHelp':
        'Falls back to MKV when a video has audio or subtitles that MP4 cannot hold.',
    'qualityHighest': 'Highest',
    'qualityHigh': 'High',
    'qualityBalanced': 'Balanced',
    'qualitySmallest': 'Smallest files',
    'qualityHighestHelp': 'Practically identical to the original.',
    'qualityHighHelp': 'Looks the same to almost everyone. Recommended.',
    'qualityBalancedHelp': 'A good trade for saving more space.',
    'qualitySmallestHelp': 'Noticeably smaller files, some visible softening.',
    'languageSystem': 'Match my device',
    'makerNvidia': 'NVIDIA',
    'makerIntel': 'Intel',
    'makerAmd': 'AMD',
    'makerWindows': 'Windows',
    'makerApple': 'Apple',
    'makerPhone': 'this phone',
    'makerVulkan': 'Vulkan',
    'encoderGraphicsCard': '{maker} graphics card',
    'encoderProcessor': 'Processor ({name})',
    'filesFound': '{count} video(s) to convert',
    'fileOfTotal': 'File {index} of {total}',
    'fileNoTotal': 'File {index}',
    'qualityScore': 'Quality {score} out of 100',
    'sizeComparison': '{before} to {after}, {change}',
    'percentOfOriginal': '{percent}% of the original size',
    'reviewsWaiting': '{count} conversion(s) waiting for your decision',
    'skippedFile': 'Left {name} alone: {reason}',
    'failedFile': 'Could not convert {name}: {reason}',
    'tryingSetting': 'Trial run scored {score}',
    'featureUnavailable': 'Not available: {detail}',
    'settingsProblem': 'Your settings could not be saved: {detail}',
  },
  'zh-Hans': {
    'tagline': '让视频更小，同时保住真正重要的画质。',
    'navConvert': '转换',
    'navActivity': '进度',
    'navReview': '待你决定',
    'navSettings': '设置',
    'folderTitle': '视频文件夹',
    'folderHint': '选择存放待压缩视频的文件夹。',
    'chooseFolder': '选择文件夹',
    'folderRequired': '请先选择一个文件夹。',
    'outputCodec': '视频格式',
    'quality': '画质',
    'howCareful': '严格程度',
    'start': '开始',
    'stop': '停止',
    'stopping': '正在完成当前文件后停止',
    'previewOnly': '仅预览',
    'previewOnlyHelp': '只列出会发生的变化，不实际转换。',
    'readyToolsMissing': '未找到视频工具',
    'readyToolsFound': '准备就绪',
    'toolsMissingHelp': 'MyVidComp 需要 FFmpeg 工具才能转换视频。请在设置中指定它们的位置。',
    'statusReady': '随时可以开始',
    'statusStarting': '正在准备',
    'statusScanning': '正在浏览文件夹',
    'statusConverting': '正在转换',
    'statusMeasuring': '正在检查画质',
    'statusTuning': '正在挑选最佳设置',
    'statusFinishing': '正在收尾',
    'statusDone': '全部完成',
    'statusStopped': '已停止',
    'statusFailed': '出了点问题',
    'nothingToDo': '这里没有需要转换的视频。',
    'currentFile': '正在处理',
    'overallProgress': '全部文件',
    'timeLeft': '剩余 {time}',
    'trialOfTotal': '第 {trial} 次试编码，最多 {total} 次',
    'converted': '已转换',
    'skipped': '保持原样',
    'failed': '无法转换',
    'needsReview': '等你决定',
    'spaceSaved': '节省空间',
    'details': '详情',
    'noActivityYet': '还没有任何动静。',
    'reviewTitle': '由你决定',
    'reviewIntro': '这些转换改动了一些细节，或者分数低于你的目标。两份文件都还在。',
    'reviewEmpty': '没有需要你决定的项目',
    'reviewEmptyHelp': '需要你拿主意的转换会出现在这里，其余的都会自动处理。',
    'keepNew': '保留新文件',
    'keepOriginal': '保留原文件',
    'keepBoth': '两个都留',
    'keepNewHelp': '用更小的新文件替换原文件。',
    'keepOriginalHelp': '删除新文件，原文件保持不变。',
    'keepBothHelp': '重命名新文件，原文件也一并保留。',
    'whatChanged': '改动内容',
    'qualityNotMeasured': '未测量画质',
    'refresh': '刷新',
    'applyToAll': '全部应用',
    'settingsOriginals': '原始文件',
    'keepOriginalsOn': '保留原文件',
    'keepOriginalsOff': '校验通过后替换原文件',
    'settingsSpeed': '速度',
    'whichEncoder': '使用哪种硬件',
    'container': '文件类型',
    'qualityStrategy': '如何达到画质目标',
    'qualityCheckLabel': '画质检查',
    'specificEncoder': '指定某个编码器',
    'specificEncoderHelp': '除非有特别需要，否则保持“自动”。',
    'limitLabel': '处理多少个文件后停止',
    'limitHelp': '留空表示全部转换。',
    'workingFolder': '工作文件夹',
    'workingFolderHelp': '临时文件的存放位置。留空则放在每个视频旁边，处理网络硬盘时这样最快。',
    'advanced': '高级',
    'mediaTools': '视频工具',
    'mediaToolsHelp': '留空则使用 MyVidComp 自带的工具。',
    'language': '语言',
    'appearance': '外观',
    'themeSystem': '跟随系统',
    'themeLight': '浅色',
    'themeDark': '深色',
    'reviewMarginLabel': '画质低于多少时询问我',
    'reviewMarginHelp': '结果比目标低多少时，MyVidComp 会保留两份文件让你选择。',
    'automatic': '自动',
    'browse': '浏览',
    'codecAv1': 'AV1',
    'codecHevc': 'H.265',
    'codecVvc': 'H.266',
    'codecAv1Help': '文件最小。2020 年以后的设备大多能播放。',
    'codecHevcHelp': '文件略大，但几乎所有设备都能播放，包括较旧的手机和电视。',
    'codecVvcHelp': '实验性。制作非常慢，而且目前几乎没有播放器支持。',
    'preservationFlexible': '平衡',
    'preservationStrict': '完全一致',
    'preservationFlexibleHelp': '能转换更多视频。当某些细节不得不改变时，MyVidComp 会保留两份文件并询问你。',
    'preservationStrictHelp': '只有每个细节都能完整保留时才转换，部分视频会保持原样。',
    'qualityModeSearch': '实测并调整',
    'qualityModeEstimate': '快速估算',
    'qualityModeSearchHelp': '截取片段试编码，找出既能保住观感又最小的设置。较慢，但效果更好。',
    'qualityModeEstimateHelp': '根据原视频推算设置。快得多，但不够精准。',
    'qualityCheckSampled': '抽样检查',
    'qualityCheckFull': '逐帧检查',
    'qualityCheckOff': '跳过',
    'qualityCheckSampledHelp': '每五帧比对一帧。速度快，准确度足够可信。',
    'qualityCheckFullHelp': '比对每一帧。最准确，也最慢。',
    'qualityCheckOffHelp': '不做比对。有改动的转换仍然会被检查。',
    'encoderAuto': '平衡',
    'encoderGpu': '优先用显卡',
    'encoderCpu': '仅用处理器',
    'encoderAutoHelp': '显卡能胜任时就用显卡，否则用处理器。',
    'encoderGpuHelp': '最快。即使画面会有细微变化，也优先使用显卡。',
    'encoderCpuHelp': '最慢，但通常文件也最小。',
    'containerMp4': '仅 MP4',
    'containerMkvFallback': 'MP4，必要时用 MKV',
    'containerMp4Help': '兼容性最好的文件类型。音轨或字幕特殊的视频会保持原样。',
    'containerMkvFallbackHelp': '当视频的音轨或字幕无法装进 MP4 时，改用 MKV。',
    'qualityHighest': '最高',
    'qualityHigh': '高',
    'qualityBalanced': '平衡',
    'qualitySmallest': '文件最小',
    'qualityHighestHelp': '与原片几乎完全一致。',
    'qualityHighHelp': '几乎没人看得出差别。推荐。',
    'qualityBalancedHelp': '为多省空间做的合理取舍。',
    'qualitySmallestHelp': '文件明显更小，画面会有一些变软。',
    'languageSystem': '跟随系统',
    'makerNvidia': 'NVIDIA',
    'makerIntel': 'Intel',
    'makerAmd': 'AMD',
    'makerWindows': 'Windows',
    'makerApple': 'Apple',
    'makerPhone': '本机',
    'makerVulkan': 'Vulkan',
    'encoderGraphicsCard': '{maker} 显卡',
    'encoderProcessor': '处理器（{name}）',
    'filesFound': '有 {count} 个视频待转换',
    'fileOfTotal': '第 {index} 个，共 {total} 个',
    'fileNoTotal': '第 {index} 个',
    'qualityScore': '画质 {score} 分（满分 100）',
    'sizeComparison': '{before} → {after}，{change}',
    'percentOfOriginal': '为原文件的 {percent}%',
    'reviewsWaiting': '有 {count} 项转换等你决定',
    'skippedFile': '{name} 保持原样：{reason}',
    'failedFile': '{name} 无法转换：{reason}',
    'tryingSetting': '试编码得分 {score}',
    'featureUnavailable': '无法使用：{detail}',
    'settingsProblem': '设置未能保存：{detail}',
  },
  'zh-Hant': {
    'tagline': '讓影片更小，同時保住真正重要的畫質。',
    'navConvert': '轉換',
    'navActivity': '進度',
    'navReview': '待你決定',
    'navSettings': '設定',
    'folderTitle': '影片資料夾',
    'folderHint': '選擇存放待壓縮影片的資料夾。',
    'chooseFolder': '選擇資料夾',
    'folderRequired': '請先選擇一個資料夾。',
    'outputCodec': '影片格式',
    'quality': '畫質',
    'howCareful': '嚴格程度',
    'start': '開始',
    'stop': '停止',
    'stopping': '正在完成目前檔案後停止',
    'previewOnly': '僅預覽',
    'previewOnlyHelp': '只列出會發生的變化，不實際轉換。',
    'readyToolsMissing': '找不到影片工具',
    'readyToolsFound': '準備就緒',
    'toolsMissingHelp': 'MyVidComp 需要 FFmpeg 工具才能轉換影片。請在設定中指定它們的位置。',
    'statusReady': '隨時可以開始',
    'statusStarting': '正在準備',
    'statusScanning': '正在瀏覽資料夾',
    'statusConverting': '正在轉換',
    'statusMeasuring': '正在檢查畫質',
    'statusTuning': '正在挑選最佳設定',
    'statusFinishing': '正在收尾',
    'statusDone': '全部完成',
    'statusStopped': '已停止',
    'statusFailed': '出了點問題',
    'nothingToDo': '這裡沒有需要轉換的影片。',
    'currentFile': '正在處理',
    'overallProgress': '全部檔案',
    'timeLeft': '剩餘 {time}',
    'trialOfTotal': '第 {trial} 次試編碼，最多 {total} 次',
    'converted': '已轉換',
    'skipped': '保持原樣',
    'failed': '無法轉換',
    'needsReview': '等你決定',
    'spaceSaved': '節省空間',
    'details': '詳細資料',
    'noActivityYet': '還沒有任何動靜。',
    'reviewTitle': '由你決定',
    'reviewIntro': '這些轉換改動了一些細節，或者分數低於你的目標。兩份檔案都還在。',
    'reviewEmpty': '沒有需要你決定的項目',
    'reviewEmptyHelp': '需要你拿主意的轉換會出現在這裡，其餘的都會自動處理。',
    'keepNew': '保留新檔案',
    'keepOriginal': '保留原檔案',
    'keepBoth': '兩個都留',
    'keepNewHelp': '用更小的新檔案取代原檔案。',
    'keepOriginalHelp': '刪除新檔案，原檔案保持不變。',
    'keepBothHelp': '重新命名新檔案，原檔案也一併保留。',
    'whatChanged': '改動內容',
    'qualityNotMeasured': '未測量畫質',
    'refresh': '重新整理',
    'applyToAll': '全部套用',
    'settingsOriginals': '原始檔案',
    'keepOriginalsOn': '保留原檔案',
    'keepOriginalsOff': '驗證通過後取代原檔案',
    'settingsSpeed': '速度',
    'whichEncoder': '使用哪種硬體',
    'container': '檔案類型',
    'qualityStrategy': '如何達到畫質目標',
    'qualityCheckLabel': '畫質檢查',
    'specificEncoder': '指定某個編碼器',
    'specificEncoderHelp': '除非有特別需要，否則保持「自動」。',
    'limitLabel': '處理多少個檔案後停止',
    'limitHelp': '留空表示全部轉換。',
    'workingFolder': '工作資料夾',
    'workingFolderHelp': '暫存檔案的存放位置。留空則放在每個影片旁邊，處理網路硬碟時這樣最快。',
    'advanced': '進階',
    'mediaTools': '影片工具',
    'mediaToolsHelp': '留空則使用 MyVidComp 內建的工具。',
    'language': '語言',
    'appearance': '外觀',
    'themeSystem': '跟隨系統',
    'themeLight': '淺色',
    'themeDark': '深色',
    'reviewMarginLabel': '畫質低於多少時詢問我',
    'reviewMarginHelp': '結果比目標低多少時，MyVidComp 會保留兩份檔案讓你選擇。',
    'automatic': '自動',
    'browse': '瀏覽',
    'codecAv1': 'AV1',
    'codecHevc': 'H.265',
    'codecVvc': 'H.266',
    'codecAv1Help': '檔案最小。2020 年以後的裝置大多能播放。',
    'codecHevcHelp': '檔案略大，但幾乎所有裝置都能播放，包括較舊的手機和電視。',
    'codecVvcHelp': '實驗性。製作非常慢，而且目前幾乎沒有播放器支援。',
    'preservationFlexible': '平衡',
    'preservationStrict': '完全一致',
    'preservationFlexibleHelp': '能轉換更多影片。當某些細節不得不改變時，MyVidComp 會保留兩份檔案並詢問你。',
    'preservationStrictHelp': '只有每個細節都能完整保留時才轉換，部分影片會保持原樣。',
    'qualityModeSearch': '實測並調整',
    'qualityModeEstimate': '快速估算',
    'qualityModeSearchHelp': '擷取片段試編碼，找出既能保住觀感又最小的設定。較慢，但效果更好。',
    'qualityModeEstimateHelp': '根據原影片推算設定。快得多，但不夠精準。',
    'qualityCheckSampled': '抽樣檢查',
    'qualityCheckFull': '逐格檢查',
    'qualityCheckOff': '略過',
    'qualityCheckSampledHelp': '每五格比對一格。速度快，準確度足夠可信。',
    'qualityCheckFullHelp': '比對每一格。最準確，也最慢。',
    'qualityCheckOffHelp': '不做比對。有改動的轉換仍然會被檢查。',
    'encoderAuto': '平衡',
    'encoderGpu': '優先用顯示卡',
    'encoderCpu': '僅用處理器',
    'encoderAutoHelp': '顯示卡能勝任時就用顯示卡，否則用處理器。',
    'encoderGpuHelp': '最快。即使畫面會有細微變化，也優先使用顯示卡。',
    'encoderCpuHelp': '最慢，但通常檔案也最小。',
    'containerMp4': '僅 MP4',
    'containerMkvFallback': 'MP4，必要時用 MKV',
    'containerMp4Help': '相容性最好的檔案類型。音軌或字幕特殊的影片會保持原樣。',
    'containerMkvFallbackHelp': '當影片的音軌或字幕無法裝進 MP4 時，改用 MKV。',
    'qualityHighest': '最高',
    'qualityHigh': '高',
    'qualityBalanced': '平衡',
    'qualitySmallest': '檔案最小',
    'qualityHighestHelp': '與原片幾乎完全一致。',
    'qualityHighHelp': '幾乎沒人看得出差別。推薦。',
    'qualityBalancedHelp': '為多省空間做的合理取捨。',
    'qualitySmallestHelp': '檔案明顯更小，畫面會有一些變軟。',
    'languageSystem': '跟隨系統',
    'makerNvidia': 'NVIDIA',
    'makerIntel': 'Intel',
    'makerAmd': 'AMD',
    'makerWindows': 'Windows',
    'makerApple': 'Apple',
    'makerPhone': '本機',
    'makerVulkan': 'Vulkan',
    'encoderGraphicsCard': '{maker} 顯示卡',
    'encoderProcessor': '處理器（{name}）',
    'filesFound': '有 {count} 個影片待轉換',
    'fileOfTotal': '第 {index} 個，共 {total} 個',
    'fileNoTotal': '第 {index} 個',
    'qualityScore': '畫質 {score} 分（滿分 100）',
    'sizeComparison': '{before} → {after}，{change}',
    'percentOfOriginal': '為原檔案的 {percent}%',
    'reviewsWaiting': '有 {count} 項轉換等你決定',
    'skippedFile': '{name} 保持原樣：{reason}',
    'failedFile': '{name} 無法轉換：{reason}',
    'tryingSetting': '試編碼得分 {score}',
    'featureUnavailable': '無法使用：{detail}',
    'settingsProblem': '設定未能儲存：{detail}',
  },
  'ja': {
    'tagline': '大切な画質はそのままに、動画を小さくします。',
    'navConvert': '変換',
    'navActivity': '進行状況',
    'navReview': '要確認',
    'navSettings': '設定',
    'folderTitle': '動画フォルダ',
    'folderHint': '小さくしたい動画が入っているフォルダを選んでください。',
    'chooseFolder': 'フォルダを選ぶ',
    'folderRequired': 'まずフォルダを選んでください。',
    'outputCodec': '動画形式',
    'quality': '画質',
    'howCareful': '厳密さ',
    'start': '開始',
    'stop': '停止',
    'stopping': 'このファイルを終えてから停止します',
    'previewOnly': 'プレビューのみ',
    'previewOnlyHelp': '変換はせず、何が変わるかだけを表示します。',
    'readyToolsMissing': '動画ツールが見つかりません',
    'readyToolsFound': '準備完了',
    'toolsMissingHelp': 'MyVidComp の変換には FFmpeg ツールが必要です。設定で場所を指定してください。',
    'statusReady': 'いつでも開始できます',
    'statusStarting': '準備中',
    'statusScanning': 'フォルダを確認しています',
    'statusConverting': '変換中',
    'statusMeasuring': '画質を確認しています',
    'statusTuning': '最適な設定を選んでいます',
    'statusFinishing': '仕上げ中',
    'statusDone': '完了しました',
    'statusStopped': '停止しました',
    'statusFailed': '問題が発生しました',
    'nothingToDo': 'ここに変換が必要な動画はありません。',
    'currentFile': '処理中',
    'overallProgress': 'すべてのファイル',
    'timeLeft': '残り {time}',
    'trialOfTotal': '試験変換 {trial} 回目（最大 {total} 回）',
    'converted': '変換済み',
    'skipped': 'そのまま',
    'failed': '変換できません',
    'needsReview': '確認待ち',
    'spaceSaved': '節約した容量',
    'details': '詳細',
    'noActivityYet': 'まだ何も起きていません。',
    'reviewTitle': 'あなたの判断',
    'reviewIntro': 'これらの変換では細部が変わったか、目標を下回りました。どちらのファイルも残っています。',
    'reviewEmpty': '確認待ちの項目はありません',
    'reviewEmptyHelp': '判断が必要な変換だけがここに表示されます。ほかは自動で処理されます。',
    'keepNew': '新しい方を残す',
    'keepOriginal': '元の方を残す',
    'keepBoth': '両方残す',
    'keepNewHelp': '元のファイルを小さい方に置き換えます。',
    'keepOriginalHelp': '新しいファイルを削除し、元のファイルはそのままにします。',
    'keepBothHelp': '新しいファイルの名前を変えて、元のファイルも残します。',
    'whatChanged': '変わった点',
    'qualityNotMeasured': '画質は未測定',
    'refresh': '更新',
    'applyToAll': 'すべてに適用',
    'settingsOriginals': '元のファイル',
    'keepOriginalsOn': '元のファイルを残す',
    'keepOriginalsOff': '確認後に元のファイルを置き換える',
    'settingsSpeed': '速度',
    'whichEncoder': '使用するハードウェア',
    'container': 'ファイル形式',
    'qualityStrategy': '目標画質への近づけ方',
    'qualityCheckLabel': '画質チェック',
    'specificEncoder': 'エンコーダーを指定する',
    'specificEncoderHelp': '特別な理由がなければ「自動」のままにしてください。',
    'limitLabel': '何ファイルで止めるか',
    'limitHelp': '空欄ならすべて変換します。',
    'workingFolder': '作業フォルダ',
    'workingFolderHelp': '一時ファイルの置き場所です。空欄なら各動画のそばに置かれ、ネットワークドライブでは最も速くなります。',
    'advanced': '詳細設定',
    'mediaTools': '動画ツール',
    'mediaToolsHelp': '空欄なら MyVidComp 付属のツールを使います。',
    'language': '言語',
    'appearance': '外観',
    'themeSystem': '端末に合わせる',
    'themeLight': 'ライト',
    'themeDark': 'ダーク',
    'reviewMarginLabel': '画質がどれだけ下回ったら確認するか',
    'reviewMarginHelp': '目標をどれだけ下回ったときに、MyVidComp が両方のファイルを残して尋ねるかです。',
    'automatic': '自動',
    'browse': '参照',
    'codecAv1': 'AV1',
    'codecHevc': 'H.265',
    'codecVvc': 'H.266',
    'codecAv1Help': 'ファイルが最も小さくなります。2020 年以降の機器ならたいてい再生できます。',
    'codecHevcHelp': 'ファイルはやや大きくなりますが、古いスマートフォンやテレビも含めほぼ何でも再生できます。',
    'codecVvcHelp': '実験的です。作成が非常に遅く、再生できる機器もほとんどありません。',
    'preservationFlexible': 'バランス',
    'preservationStrict': '完全一致',
    'preservationFlexibleHelp':
        'より多くの動画を変換します。細部を変えざるを得ないときは、両方のファイルを残して確認します。',
    'preservationStrictHelp': 'すべての情報をそのまま再現できるときだけ変換します。一部の動画は手つかずのままになります。',
    'qualityModeSearch': '実測して調整',
    'qualityModeEstimate': '簡易見積もり',
    'qualityModeSearchHelp': '短いサンプルを試し、見た目を保てる最小のサイズを探します。遅いぶん結果は良好です。',
    'qualityModeEstimateHelp': '元の動画から設定を推定します。はるかに速いぶん精度は劣ります。',
    'qualityCheckSampled': '抜き取り確認',
    'qualityCheckFull': '全フレーム',
    'qualityCheckOff': '省略',
    'qualityCheckSampledHelp': '5 フレームに 1 枚を比較します。速く、信頼できる精度です。',
    'qualityCheckFullHelp': 'すべてのフレームを比較します。最も正確で、最も遅くなります。',
    'qualityCheckOffHelp': '比較しません。変更があった変換は引き続き確認されます。',
    'encoderAuto': 'バランス',
    'encoderGpu': 'グラフィックスを優先',
    'encoderCpu': 'プロセッサーのみ',
    'encoderAutoHelp': 'グラフィックスで問題なく処理できるときはそちらを、無理ならプロセッサーを使います。',
    'encoderGpuHelp': '最速です。画に多少の変化が出る場合でもグラフィックスを使います。',
    'encoderCpuHelp': '最も遅く、たいていファイルは最も小さくなります。',
    'containerMp4': 'MP4 のみ',
    'containerMkvFallback': 'MP4、必要なら MKV',
    'containerMp4Help': '最も互換性の高い形式です。特殊な音声や字幕を含む動画はそのまま残します。',
    'containerMkvFallbackHelp': '音声や字幕が MP4 に収まらない場合は MKV を使います。',
    'qualityHighest': '最高',
    'qualityHigh': '高',
    'qualityBalanced': 'バランス',
    'qualitySmallest': '最小サイズ',
    'qualityHighestHelp': '元とほぼ見分けがつきません。',
    'qualityHighHelp': 'ほとんどの人には違いが分かりません。おすすめです。',
    'qualityBalancedHelp': '容量をもっと節約するための妥当な折り合いです。',
    'qualitySmallestHelp': 'ファイルは大きく小さくなりますが、多少やわらかく見えます。',
    'languageSystem': '端末に合わせる',
    'makerNvidia': 'NVIDIA',
    'makerIntel': 'Intel',
    'makerAmd': 'AMD',
    'makerWindows': 'Windows',
    'makerApple': 'Apple',
    'makerPhone': 'この端末',
    'makerVulkan': 'Vulkan',
    'encoderGraphicsCard': '{maker} グラフィックス',
    'encoderProcessor': 'プロセッサー（{name}）',
    'filesFound': '変換する動画が {count} 件あります',
    'fileOfTotal': '{total} 件中 {index} 件目',
    'fileNoTotal': '{index} 件目',
    'qualityScore': '画質 {score} 点（100 点満点）',
    'sizeComparison': '{before} → {after}、{change}',
    'percentOfOriginal': '元のサイズの {percent}%',
    'reviewsWaiting': '{count} 件の変換が確認を待っています',
    'skippedFile': '{name} はそのままにしました：{reason}',
    'failedFile': '{name} を変換できませんでした：{reason}',
    'tryingSetting': '試験変換のスコアは {score}',
    'featureUnavailable': '利用できません：{detail}',
    'settingsProblem': '設定を保存できませんでした：{detail}',
  },
};
