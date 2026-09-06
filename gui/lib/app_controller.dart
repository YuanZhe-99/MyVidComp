import 'dart:async';

import 'package:flutter/foundation.dart';

import 'app_settings.dart';
import 'core_ffi.dart';
import 'media_tools.dart';

/// Where a run currently is, in terms a person would use.
enum RunStage {
  idle,
  starting,
  scanning,
  tuning,
  converting,
  measuring,
  finishing,
  done,
  stopped,
  failed,
}

/// What happened to one file.
enum FileOutcome { converted, skipped, failed, needsReview }

/// One line in the progress list.
class FileRecord {
  const FileRecord({
    required this.name,
    required this.outcome,
    required this.detail,
    this.score,
    this.sourceBytes,
    this.outputBytes,
  });

  final String name;
  final FileOutcome outcome;
  final String detail;

  /// Measured quality in hundredths, when there is one.
  final int? score;
  final int? sourceBytes;
  final int? outputBytes;
}

/// Holds every piece of interface state and talks to the conversion engine.
///
/// The interface reads from here and calls the methods below; nothing else
/// knows about isolates, native calls or the engine's wire format.
class AppController extends ChangeNotifier {
  AppController({
    this._settingsStore = const SettingsStore(),
    this._reviewStore = const ReviewStore(),
    Future<MediaTools> Function({
      String ffmpegOverride,
      String ffprobeOverride,
    })?
    detectTools,
  }) : _detectTools = detectTools ?? MediaTools.detect;

  final SettingsStore _settingsStore;
  final ReviewStore _reviewStore;
  final Future<MediaTools> Function({
    String ffmpegOverride,
    String ffprobeOverride,
  })
  _detectTools;

  AppSettings _settings = AppSettings.defaults;
  MediaTools _tools = const MediaTools.unknown();
  WorkerController? _worker;
  Timer? _saveTimer;

  RunStage _stage = RunStage.idle;
  bool _running = false;
  bool _stopping = false;
  String _currentFile = '';
  int? _fileIndex;
  int? _totalFiles;
  double _fileProgress = 0;
  String? _lastError;
  String? _notice;

  int _converted = 0;
  int _skipped = 0;
  int _failed = 0;
  int _reviewCount = 0;
  int _sourceBytes = 0;
  int _outputBytes = 0;

  final List<FileRecord> _records = [];
  final List<String> _log = [];
  List<PendingReview> _reviews = const [];
  bool _loadingReviews = false;

  AppSettings get settings => _settings;
  MediaTools get tools => _tools;
  RunStage get stage => _stage;
  bool get isRunning => _running;
  bool get isStopping => _stopping;
  String get currentFile => _currentFile;
  int? get fileIndex => _fileIndex;
  int? get totalFiles => _totalFiles;
  double get fileProgress => _fileProgress;
  String? get lastError => _lastError;
  String? get notice => _notice;
  int get converted => _converted;
  int get skipped => _skipped;
  int get failed => _failed;
  int get reviewCount => _reviewCount;
  int get sourceBytes => _sourceBytes;
  int get outputBytes => _outputBytes;
  List<FileRecord> get records => List.unmodifiable(_records);
  List<String> get log => List.unmodifiable(_log);
  List<PendingReview> get reviews => List.unmodifiable(_reviews);
  bool get loadingReviews => _loadingReviews;

  /// True once a folder is chosen and the tools were found.
  bool get canStart =>
      !_running && _settings.targetFolder.trim().isNotEmpty && _tools.areReady;

  // AI-FUNC-SUMMARY: Loads stored settings and finds the video tools; returns when both are done; side effects: reads the settings file and the filesystem.
  Future<void> initialise() async {
    _settings = await _settingsStore.load();
    notifyListeners();
    await refreshTools();
    await refreshReviews();
  }

  // AI-FUNC-SUMMARY: Looks for the video tools again; returns when the search finishes; side effects: reads the filesystem.
  Future<void> refreshTools() async {
    _tools = await _detectTools(
      ffmpegOverride: _settings.ffmpeg,
      ffprobeOverride: _settings.ffprobe,
    );
    notifyListeners();
  }

  // AI-FUNC-SUMMARY: Replaces the settings and saves them shortly afterwards; returns none; side effects: schedules a write and may re-check the tools.
  void updateSettings(AppSettings settings) {
    final toolsChanged =
        settings.ffmpeg != _settings.ffmpeg ||
        settings.ffprobe != _settings.ffprobe;
    final folderChanged = settings.targetFolder != _settings.targetFolder;
    _settings = settings;
    notifyListeners();

    _saveTimer?.cancel();
    _saveTimer = Timer(const Duration(milliseconds: 400), _save);

    if (toolsChanged) {
      unawaited(refreshTools());
    }
    if (folderChanged) {
      unawaited(refreshReviews());
    }
  }

  // AI-FUNC-SUMMARY: Writes the settings to the user's profile; returns when the write finishes; side effects: replaces the settings file.
  Future<void> _save() async {
    try {
      await _settingsStore.save(_settings);
    } catch (error) {
      _notice = error.toString();
      notifyListeners();
    }
  }

  // AI-FUNC-SUMMARY: Clears the one-off notice shown to the user; returns none; side effects: updates state.
  void clearNotice() {
    if (_notice == null) {
      return;
    }
    _notice = null;
    notifyListeners();
  }

  // AI-FUNC-SUMMARY: Reads the conversions waiting for a decision; returns when the list is refreshed; side effects: reads the chosen folder.
  Future<void> refreshReviews() async {
    if (_settings.targetFolder.trim().isEmpty) {
      _reviews = const [];
      _reviewCount = 0;
      notifyListeners();
      return;
    }

    _loadingReviews = true;
    notifyListeners();
    try {
      _reviews = await _reviewStore.list(_settings.targetFolder);
    } catch (error) {
      // The engine is what reads this list, and it can be missing or unusable.
      // The screen that reports missing tools covers that case, so the list is
      // left empty here instead of stopping the rest of the application.
      _reviews = const [];
      _notice = error.toString();
    }
    _reviewCount = _reviews.length;
    _loadingReviews = false;
    notifyListeners();
  }

  // AI-FUNC-SUMMARY: Applies one decision to one pending conversion; returns an error message, or null when it worked; side effects: renames or deletes files and refreshes the list.
  Future<String?> resolveReview(PendingReview review, String decision) async {
    final error = await _reviewStore.resolve(review.sidecarPath, decision);
    await refreshReviews();
    return error;
  }

  // AI-FUNC-SUMMARY: Applies one decision to every pending conversion; returns the first error, or null when all worked; side effects: renames or deletes files and refreshes the list.
  Future<String?> resolveAllReviews(String decision) async {
    String? firstError;
    for (final review in List<PendingReview>.from(_reviews)) {
      final error = await _reviewStore.resolve(review.sidecarPath, decision);
      firstError ??= error;
    }
    await refreshReviews();
    return firstError;
  }

  // AI-FUNC-SUMMARY: Starts converting the chosen folder; returns when the run has begun; side effects: saves settings and spawns the conversion worker.
  Future<void> start() async {
    if (_running || !canStart) {
      return;
    }

    await _save();
    _resetRunState();
    _running = true;
    _stage = RunStage.starting;
    notifyListeners();

    final request = RunRequest(
      targetFolder: _settings.targetFolder,
      count: _settings.wireCount,
      keepOriginal: _settings.keepOriginal,
      dryRun: _settings.previewOnly,
      ffmpeg: _tools.ffmpeg.path,
      ffprobe: _tools.ffprobe.path,
      tmpDir: _settings.tmpDir,
      encoder: _settings.encoder,
      encoderPreference: _settings.encoderPreference,
      outputFormat: _settings.container,
      targetCodec: _settings.codec,
      preservation: _settings.preservation,
      qualityMode: _settings.qualityMode,
      qualityCheck: _settings.qualityCheck,
      qualityTarget: _settings.qualityTarget,
      reviewMargin: _settings.reviewMargin,
    );

    try {
      _worker = await WorkerController.start(
        request,
        onEvent: _handleEvent,
        onDone: _handleDone,
        onLog: _pushLog,
      );
    } catch (error) {
      _running = false;
      _stage = RunStage.failed;
      _lastError = error.toString();
      notifyListeners();
    }
  }

  // AI-FUNC-SUMMARY: Asks the run to stop once it finishes the file it is on; returns none; side effects: signals the worker.
  void stop() {
    if (!_running || _stopping) {
      return;
    }
    _stopping = true;
    _worker?.requestStop();
    notifyListeners();
  }

  // AI-FUNC-SUMMARY: Clears everything recorded about the previous run; returns none; side effects: updates state.
  void _resetRunState() {
    _stopping = false;
    _currentFile = '';
    _fileIndex = null;
    _totalFiles = null;
    _fileProgress = 0;
    _lastError = null;
    _converted = 0;
    _skipped = 0;
    _failed = 0;
    _sourceBytes = 0;
    _outputBytes = 0;
    _records.clear();
    _log.clear();
  }

  // AI-FUNC-SUMMARY: Applies one engine event to the interface state; returns none; side effects: updates state and notifies listeners.
  void _handleEvent(WorkerEvent event) {
    final data = event.data;

    switch (event.kind) {
      case WorkerEventType.log:
        _pushLog(event.message ?? '');
      case WorkerEventType.capabilityMissing:
        _notice = data['detail'] as String?;
        _pushLog(data['detail'] as String? ?? '');
      case WorkerEventType.scanStarted:
        _stage = RunStage.scanning;
      case WorkerEventType.scanFinished:
        _totalFiles = data['candidates'] as int?;
        _skipped = data['skipped'] as int? ?? 0;
      case WorkerEventType.dryRun:
        _totalFiles = (data['candidates'] as List<Object?>?)?.length;
        _stage = RunStage.done;
      case WorkerEventType.fileSkipped:
        _skipped += 1;
        _records.add(
          FileRecord(
            name: _fileName(data['path'] as String? ?? ''),
            outcome: FileOutcome.skipped,
            detail: data['reason'] as String? ?? '',
          ),
        );
      case WorkerEventType.fileStarted:
        _stage = RunStage.converting;
        _fileIndex = data['index'] as int?;
        _totalFiles = data['total'] as int? ?? _totalFiles;
        _currentFile = _fileName(data['input_path'] as String? ?? '');
        _fileProgress = 0;
      case WorkerEventType.fileProgress:
        _stage = RunStage.converting;
        _fileProgress = ((data['percent'] as num?) ?? 0).toDouble();
      case WorkerEventType.copyProgress:
        _stage = RunStage.finishing;
        _fileProgress = ((data['percent'] as num?) ?? 0).toDouble();
      case WorkerEventType.stage:
        final stage = (data['stage'] as String? ?? '').toLowerCase();
        if (stage.contains('measuring')) {
          _stage = RunStage.measuring;
        } else if (stage.contains('choosing')) {
          _stage = RunStage.tuning;
        }
      case WorkerEventType.qualitySearch:
        final score = data['score'] as int?;
        if (score != null) {
          _pushLog('${data['quality']}: ${formatScore(score)}');
        }
      case WorkerEventType.qualityMeasured:
        _lastScore = data['score'] as int?;
      case WorkerEventType.reviewPending:
        _reviewCount += 1;
        _records.add(
          FileRecord(
            name: _fileName(data['original_path'] as String? ?? ''),
            outcome: FileOutcome.needsReview,
            detail: ((data['deviations'] as List<Object?>?) ?? const [])
                .whereType<String>()
                .join('\n'),
            score: data['score'] as int?,
            sourceBytes: data['original_bytes'] as int?,
            outputBytes: data['review_bytes'] as int?,
          ),
        );
      case WorkerEventType.fileFinished:
        _recordFinished(data);
      case WorkerEventType.summary:
        final summary = data['summary'] as Map<String, Object?>? ?? const {};
        _converted = summary['converted'] as int? ?? _converted;
        _skipped = summary['skipped'] as int? ?? _skipped;
        _failed = summary['failed'] as int? ?? _failed;
        _sourceBytes = summary['source_bytes'] as int? ?? 0;
        _outputBytes = summary['output_bytes'] as int? ?? 0;
      case WorkerEventType.stopRequested:
        _stage = RunStage.stopped;
      case WorkerEventType.settingsSelected:
      case WorkerEventType.encoderCandidateEvaluated:
      case WorkerEventType.encoderSelected:
      case WorkerEventType.fileAttempt:
      case WorkerEventType.unknown:
        break;
    }

    notifyListeners();
  }

  int? _lastScore;

  // AI-FUNC-SUMMARY: Records the outcome of one finished file; returns none; side effects: updates the record list and counters.
  void _recordFinished(Map<String, Object?> data) {
    final status = data['status'] as String? ?? '';
    final name = _fileName(data['input_path'] as String? ?? '');
    final message = data['message'] as String? ?? '';

    if (status == 'converted') {
      // A conversion kept for review was already recorded when it happened.
      final alreadyRecorded = _records.any(
        (record) =>
            record.name == name && record.outcome == FileOutcome.needsReview,
      );
      if (!alreadyRecorded) {
        _converted += 1;
        _records.add(
          FileRecord(
            name: name,
            outcome: FileOutcome.converted,
            detail: message,
            score: _lastScore,
            sourceBytes: data['source_bytes'] as int?,
            outputBytes: data['output_bytes'] as int?,
          ),
        );
      }
    } else {
      _failed += 1;
      _records.add(
        FileRecord(name: name, outcome: FileOutcome.failed, detail: message),
      );
    }
    _lastScore = null;
  }

  // AI-FUNC-SUMMARY: Finishes a run and records why it ended; returns none; side effects: updates state and refreshes the review list.
  void _handleDone(String? error) {
    _running = false;
    _stopping = false;
    _worker = null;
    _lastError = error;
    if (error != null) {
      _stage = RunStage.failed;
    } else if (_stage != RunStage.stopped) {
      _stage = RunStage.done;
    }
    notifyListeners();
    unawaited(refreshReviews());
  }

  // AI-FUNC-SUMMARY: Adds one line to the details log, keeping it from growing without bound; returns none; side effects: updates the log.
  void _pushLog(String message) {
    if (message.trim().isEmpty) {
      return;
    }
    _log.insert(0, message);
    if (_log.length > 500) {
      _log.removeRange(500, _log.length);
    }
    notifyListeners();
  }

  // AI-FUNC-SUMMARY: Reduces a full path to the file name; returns the name; side effects: none.
  static String _fileName(String path) =>
      path
          .split(RegExp(r'[/\\]'))
          .where((part) => part.isNotEmpty)
          .lastOrNull ??
      path;

  @override
  // AI-FUNC-SUMMARY: Stops any running conversion and releases everything held; returns none; side effects: kills the worker isolate and cancels the save timer.
  void dispose() {
    _saveTimer?.cancel();
    unawaited(_worker?.dispose());
    super.dispose();
  }
}

// AI-FUNC-SUMMARY: Formats a hundredths score for the details log; returns a one-decimal string; side effects: none.
String formatScore(int hundredths) => (hundredths / 100).toStringAsFixed(1);

// AI-FUNC-SUMMARY: Formats a byte count the way a person reads sizes; returns the short text; side effects: none.
String formatBytes(int bytes) {
  if (bytes < 1024) {
    return '$bytes B';
  }
  const units = ['KB', 'MB', 'GB', 'TB'];
  var value = bytes / 1024;
  var unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return '${value.toStringAsFixed(value >= 10 ? 0 : 1)} ${units[unit]}';
}
