import 'dart:async';
import 'dart:convert';
import 'dart:ffi';
import 'dart:isolate';

import 'package:ffi/ffi.dart';

import 'native_paths.dart';

/// Run options for ABI version 1, mirroring `FfiRunOptionsV1` in the Rust core.
///
/// The field order has to match exactly. Pointers first, then the 64-bit count,
/// then the 32-bit numbers, then the byte flags.
final class FfiRunOptionsV1 extends Struct {
  @Uint32()
  external int abiVersion;
  @Uint32()
  external int structSize;
  external Pointer<Char> targetFolder;
  external Pointer<Char> ffmpeg;
  external Pointer<Char> ffprobe;
  external Pointer<Char> tmpDir;
  external Pointer<Char> encoder;
  external Pointer<Char> encoderPreference;
  external Pointer<Char> outputFormat;
  external Pointer<Char> targetCodec;
  external Pointer<Char> preservation;
  external Pointer<Char> qualityMode;
  external Pointer<Char> qualityCheck;
  @Int64()
  external int count;
  @Uint32()
  external int qualityTarget;
  @Uint32()
  external int reviewMargin;
  @Uint32()
  external int qualityThreads;
  @Uint8()
  external int keepOriginal;
  @Uint8()
  external int dryRun;
}

typedef _NativeEventCallback =
    Void Function(Pointer<Char> json, Pointer<Void> userData);

typedef _RunBlockingNative =
    Pointer<Char> Function(
      Pointer<FfiRunOptionsV1> options,
      Pointer<NativeFunction<_NativeEventCallback>> callback,
      Pointer<Void> userData,
      Pointer<Void> token,
    );
typedef _RunBlockingDart =
    Pointer<Char> Function(
      Pointer<FfiRunOptionsV1> options,
      Pointer<NativeFunction<_NativeEventCallback>> callback,
      Pointer<Void> userData,
      Pointer<Void> token,
    );

typedef _AbiVersionNative = Uint32 Function();
typedef _AbiVersionDart = int Function();

typedef _TokenNewNative = Pointer<Void> Function();
typedef _TokenNewDart = Pointer<Void> Function();

typedef _TokenVoidNative = Void Function(Pointer<Void> token);
typedef _TokenVoidDart = void Function(Pointer<Void> token);

typedef _StringFreeNative = Void Function(Pointer<Char> value);
typedef _StringFreeDart = void Function(Pointer<Char> value);

typedef _ReviewListNative = Pointer<Char> Function(Pointer<Char> folder);
typedef _ReviewListDart = Pointer<Char> Function(Pointer<Char> folder);

typedef _ReviewResolveNative =
    Pointer<Char> Function(Pointer<Char> sidecar, Pointer<Char> decision);
typedef _ReviewResolveDart =
    Pointer<Char> Function(Pointer<Char> sidecar, Pointer<Char> decision);

/// Everything one conversion run needs, in the wire form the core expects.
class RunRequest {
  const RunRequest({
    required this.targetFolder,
    required this.count,
    required this.keepOriginal,
    required this.dryRun,
    required this.ffmpeg,
    required this.ffprobe,
    required this.tmpDir,
    required this.encoder,
    required this.encoderPreference,
    required this.outputFormat,
    required this.targetCodec,
    required this.preservation,
    required this.qualityMode,
    required this.qualityCheck,
    required this.qualityTarget,
    required this.reviewMargin,
  });

  final String targetFolder;
  final int count;
  final bool keepOriginal;
  final bool dryRun;
  final String ffmpeg;
  final String ffprobe;
  final String tmpDir;
  final String encoder;
  final String encoderPreference;
  final String outputFormat;
  final String targetCodec;
  final String preservation;
  final String qualityMode;
  final String qualityCheck;

  /// Quality target in hundredths of a VMAF point.
  final int qualityTarget;

  /// How far below the target still avoids review, in hundredths.
  final int reviewMargin;

  // AI-FUNC-SUMMARY: Packs the request for the worker isolate; returns a plain map; side effects: none.
  Map<String, Object?> toWorkerMessage() => {
    'targetFolder': targetFolder,
    'count': count,
    'keepOriginal': keepOriginal,
    'dryRun': dryRun,
    'ffmpeg': ffmpeg,
    'ffprobe': ffprobe,
    'tmpDir': tmpDir,
    'encoder': encoder,
    'encoderPreference': encoderPreference,
    'outputFormat': outputFormat,
    'targetCodec': targetCodec,
    'preservation': preservation,
    'qualityMode': qualityMode,
    'qualityCheck': qualityCheck,
    'qualityTarget': qualityTarget,
    'reviewMargin': reviewMargin,
  };

  // AI-FUNC-SUMMARY: Rebuilds a request inside the worker isolate; returns the request; side effects: none.
  static RunRequest fromWorkerMessage(Map<String, Object?> message) =>
      RunRequest(
        targetFolder: message['targetFolder']! as String,
        count: message['count']! as int,
        keepOriginal: message['keepOriginal']! as bool,
        dryRun: message['dryRun']! as bool,
        ffmpeg: message['ffmpeg']! as String,
        ffprobe: message['ffprobe']! as String,
        tmpDir: message['tmpDir']! as String,
        encoder: message['encoder']! as String,
        encoderPreference: message['encoderPreference']! as String,
        outputFormat: message['outputFormat']! as String,
        targetCodec: message['targetCodec']! as String,
        preservation: message['preservation']! as String,
        qualityMode: message['qualityMode']! as String,
        qualityCheck: message['qualityCheck']! as String,
        qualityTarget: message['qualityTarget']! as int,
        reviewMargin: message['reviewMargin']! as int,
      );
}

/// One event from the core, still in its raw decoded form.
class WorkerEvent {
  const WorkerEvent(this.type, this.data);

  final String type;
  final Map<String, Object?> data;

  WorkerEventType get kind => WorkerEventType.fromWireName(type);

  String? get message => data['message'] as String?;

  // AI-FUNC-SUMMARY: Decodes one JSON event from the core; returns the event; side effects: none.
  static WorkerEvent fromJson(String text) {
    final decoded = jsonDecode(text) as Map<String, Object?>;
    return WorkerEvent(decoded['type'] as String? ?? 'unknown', decoded);
  }
}

/// Every event the core can send.
///
/// Unrecognised names fall through to [unknown] so a newer core never crashes
/// an older interface.
enum WorkerEventType {
  log('log'),
  settingsSelected('settings_selected'),
  capabilityMissing('capability_missing'),
  encoderCandidateEvaluated('encoder_candidate_evaluated'),
  encoderSelected('encoder_selected'),
  scanStarted('scan_started'),
  scanFinished('scan_finished'),
  dryRun('dry_run'),
  fileSkipped('file_skipped'),
  fileStarted('file_started'),
  fileAttempt('file_attempt'),
  fileProgress('file_progress'),
  copyProgress('copy_progress'),
  stage('stage'),
  qualitySearch('quality_search'),
  qualityMeasured('quality_measured'),
  reviewPending('review_pending'),
  fileFinished('file_finished'),
  stopRequested('stop_requested'),
  summary('summary'),
  unknown('unknown');

  const WorkerEventType(this.wireName);

  final String wireName;

  // AI-FUNC-SUMMARY: Maps a wire event name onto its enum value; returns the matching value or unknown; side effects: none.
  static WorkerEventType fromWireName(String name) {
    for (final value in WorkerEventType.values) {
      if (value.wireName == name) {
        return value;
      }
    }
    return WorkerEventType.unknown;
  }
}

/// Runs one conversion on a worker isolate and forwards its events.
class WorkerController {
  WorkerController._(this._isolate, this._receivePort, this._tokenAddress);

  final Isolate _isolate;
  final ReceivePort _receivePort;
  final int? _tokenAddress;
  bool _finished = false;

  // AI-FUNC-SUMMARY: Starts a conversion run on its own isolate; returns the controller once the run has begun; side effects: spawns an isolate and allocates a native cancellation token.
  static Future<WorkerController> start(
    RunRequest request, {
    required void Function(WorkerEvent event) onEvent,
    required void Function(String? error) onDone,
    required void Function(String message) onLog,
  }) async {
    final receivePort = ReceivePort();
    final ready = Completer<int?>();
    late final WorkerController controller;

    final isolate = await Isolate.spawn(_workerMain, [
      receivePort.sendPort,
      request.toWorkerMessage(),
    ], debugName: 'myvidcomp-runner');

    receivePort.listen((dynamic message) {
      if (message is! Map) {
        return;
      }
      final kind = message['kind'] as String?;
      switch (kind) {
        case 'ready':
          if (!ready.isCompleted) {
            ready.complete(message['token'] as int?);
          }
        case 'event':
          onEvent(WorkerEvent.fromJson(message['json']! as String));
        case 'log':
          onLog(message['message']! as String);
        case 'done':
          controller._finished = true;
          controller._release();
          onDone(message['error'] as String?);
          receivePort.close();
      }
    });

    final tokenAddress = await ready.future;
    controller = WorkerController._(isolate, receivePort, tokenAddress);
    return controller;
  }

  // AI-FUNC-SUMMARY: Asks the run to stop after the file it is working on; returns none; side effects: sets the shared native cancellation flag.
  void requestStop() {
    final address = _tokenAddress;
    if (_finished || address == null) {
      return;
    }
    _CoreLibrary.instance.requestStop(Pointer<Void>.fromAddress(address));
  }

  // AI-FUNC-SUMMARY: Releases the native cancellation token exactly once; returns none; side effects: frees native memory.
  void _release() {
    final address = _tokenAddress;
    if (address == null || _released) {
      return;
    }
    _released = true;
    _CoreLibrary.instance.freeToken(Pointer<Void>.fromAddress(address));
  }

  bool _released = false;

  // AI-FUNC-SUMMARY: Stops the run and frees everything it holds; returns when the isolate is gone; side effects: kills the isolate and frees the native token.
  Future<void> dispose() async {
    if (!_finished) {
      requestStop();
      _isolate.kill(priority: Isolate.immediate);
    }
    // Freed here too, because a killed isolate never sends its done message.
    _release();
    _receivePort.close();
  }
}

/// The loaded native library and the functions it exports.
class _CoreLibrary {
  _CoreLibrary._(DynamicLibrary library)
    : _abiVersion = _guard(
        () => library.lookupFunction<_AbiVersionNative, _AbiVersionDart>(
          'myvidcomp_ffi_abi_version',
        ),
      ),
      _runBlocking = _guard(
        () => library.lookupFunction<_RunBlockingNative, _RunBlockingDart>(
          'myvidcomp_run_blocking_v1',
        ),
      ),
      _tokenNew = _guard(
        () => library.lookupFunction<_TokenNewNative, _TokenNewDart>(
          'myvidcomp_cancellation_token_new',
        ),
      ),
      _tokenStop = _guard(
        () => library.lookupFunction<_TokenVoidNative, _TokenVoidDart>(
          'myvidcomp_cancellation_token_request_stop',
        ),
      ),
      _tokenFree = _guard(
        () => library.lookupFunction<_TokenVoidNative, _TokenVoidDart>(
          'myvidcomp_cancellation_token_free',
        ),
      ),
      _stringFree = _guard(
        () => library.lookupFunction<_StringFreeNative, _StringFreeDart>(
          'myvidcomp_string_free',
        ),
      ),
      _reviewList = _guard(
        () => library.lookupFunction<_ReviewListNative, _ReviewListDart>(
          'myvidcomp_review_list',
        ),
      ),
      _reviewResolve = _guard(
        () => library.lookupFunction<_ReviewResolveNative, _ReviewResolveDart>(
          'myvidcomp_review_resolve',
        ),
      );

  final _AbiVersionDart? _abiVersion;
  final _RunBlockingDart? _runBlocking;
  final _TokenNewDart? _tokenNew;
  final _TokenVoidDart? _tokenStop;
  final _TokenVoidDart? _tokenFree;
  final _StringFreeDart? _stringFree;
  final _ReviewListDart? _reviewList;
  final _ReviewResolveDart? _reviewResolve;

  static final _CoreLibrary instance = _CoreLibrary._(openCoreLibrary());

  /// Every lookup is guarded, so a library built from a different version
  /// leaves the app usable instead of crashing while it starts.
  // AI-FUNC-SUMMARY: Looks up one native symbol without throwing; returns the function or null when it is absent; side effects: none.
  static T? _guard<T>(T Function() lookup) {
    try {
      return lookup();
    } on ArgumentError {
      return null;
    }
  }

  int get abiVersion => _abiVersion?.call() ?? 0;

  bool get isUsable => abiVersion >= 1 && _runBlocking != null;

  // AI-FUNC-SUMMARY: Allocates a cancellation token; returns the token pointer; side effects: allocates native memory.
  Pointer<Void> createToken() =>
      _tokenNew?.call() ?? Pointer<Void>.fromAddress(0);

  // AI-FUNC-SUMMARY: Asks a run to stop; returns none; side effects: sets the token flag.
  void requestStop(Pointer<Void> token) => _tokenStop?.call(token);

  // AI-FUNC-SUMMARY: Frees a cancellation token; returns none; side effects: frees native memory.
  void freeToken(Pointer<Void> token) => _tokenFree?.call(token);

  // AI-FUNC-SUMMARY: Runs one conversion and blocks until it finishes; returns an error message or null on success; side effects: performs the whole run and calls back with events.
  String? runBlocking(
    RunRequest request,
    Pointer<NativeFunction<_NativeEventCallback>> callback,
    Pointer<Void> token,
  ) {
    final run = _runBlocking;
    if (run == null) {
      return 'This copy of MyVidComp is missing its conversion engine.';
    }

    final options = calloc<FfiRunOptionsV1>();
    final strings = <Pointer<Char>>[];

    Pointer<Char> text(String value) {
      final pointer = value.toNativeUtf8().cast<Char>();
      strings.add(pointer);
      return pointer;
    }

    try {
      options.ref
        ..abiVersion = 1
        ..structSize = sizeOf<FfiRunOptionsV1>()
        ..targetFolder = text(request.targetFolder)
        ..ffmpeg = text(request.ffmpeg)
        ..ffprobe = text(request.ffprobe)
        ..tmpDir = text(request.tmpDir)
        ..encoder = text(request.encoder)
        ..encoderPreference = text(request.encoderPreference)
        ..outputFormat = text(request.outputFormat)
        ..targetCodec = text(request.targetCodec)
        ..preservation = text(request.preservation)
        ..qualityMode = text(request.qualityMode)
        ..qualityCheck = text(request.qualityCheck)
        ..count = request.count
        ..qualityTarget = request.qualityTarget
        ..reviewMargin = request.reviewMargin
        ..qualityThreads = 0
        ..keepOriginal = request.keepOriginal ? 1 : 0
        ..dryRun = request.dryRun ? 1 : 0;

      final error = run(options, callback, Pointer<Void>.fromAddress(0), token);
      if (error.address == 0) {
        return null;
      }
      final message = error.cast<Utf8>().toDartString();
      _stringFree?.call(error);
      return message;
    } finally {
      for (final pointer in strings) {
        calloc.free(pointer);
      }
      calloc.free(options);
    }
  }

  // AI-FUNC-SUMMARY: Asks the core which conversions are waiting for a decision; returns the JSON document; side effects: reads the folder.
  String listReviews(String folder) {
    final list = _reviewList;
    if (list == null) {
      return '{"reviews":[]}';
    }
    final path = folder.toNativeUtf8().cast<Char>();
    try {
      final result = list(path);
      if (result.address == 0) {
        return '{"reviews":[]}';
      }
      final text = result.cast<Utf8>().toDartString();
      _stringFree?.call(result);
      return text;
    } finally {
      calloc.free(path);
    }
  }

  // AI-FUNC-SUMMARY: Applies one decision to a pending review; returns an error message or null on success; side effects: renames or deletes the reviewed files.
  String? resolveReview(String sidecar, String decision) {
    final resolve = _reviewResolve;
    if (resolve == null) {
      return 'This copy of MyVidComp cannot apply review decisions.';
    }
    final sidecarPointer = sidecar.toNativeUtf8().cast<Char>();
    final decisionPointer = decision.toNativeUtf8().cast<Char>();
    try {
      final result = resolve(sidecarPointer, decisionPointer);
      if (result.address == 0) {
        return null;
      }
      final text = result.cast<Utf8>().toDartString();
      _stringFree?.call(result);
      return text;
    } finally {
      calloc.free(sidecarPointer);
      calloc.free(decisionPointer);
    }
  }
}

/// A conversion waiting for the user to choose which file to keep.
class PendingReview {
  const PendingReview({
    required this.sidecarPath,
    required this.originalPath,
    required this.reviewPath,
    required this.originalBytes,
    required this.reviewBytes,
    required this.score,
    required this.target,
    required this.deviations,
  });

  final String sidecarPath;
  final String originalPath;
  final String reviewPath;
  final int originalBytes;
  final int reviewBytes;

  /// Measured quality in hundredths, or null when it could not be measured.
  final int? score;
  final int target;
  final List<String> deviations;

  String get fileName => originalPath.split(RegExp(r'[/\\]')).last;

  double? get sizePercent =>
      originalBytes == 0 ? null : reviewBytes / originalBytes * 100;

  // AI-FUNC-SUMMARY: Reads one review entry out of the core's JSON; returns the review; side effects: none.
  static PendingReview fromJson(Map<String, Object?> entry) {
    final item = entry['item'] as Map<String, Object?>? ?? entry;
    final deviations = (item['deviations'] as List<Object?>? ?? [])
        .map(
          (value) => (value as Map<String, Object?>)['detail'] as String? ?? '',
        )
        .where((value) => value.isNotEmpty)
        .toList();
    return PendingReview(
      sidecarPath: entry['sidecar_path'] as String? ?? '',
      originalPath: item['original_path'] as String? ?? '',
      reviewPath: item['review_path'] as String? ?? '',
      originalBytes: item['original_bytes'] as int? ?? 0,
      reviewBytes: item['review_bytes'] as int? ?? 0,
      score: item['score'] as int?,
      target: item['target'] as int? ?? 0,
      deviations: deviations,
    );
  }
}

/// Reads and resolves pending reviews through the core.
class ReviewStore {
  const ReviewStore();

  // AI-FUNC-SUMMARY: Lists the conversions in a folder waiting for a decision; returns the reviews, newest folder scan first; side effects: reads the folder.
  Future<List<PendingReview>> list(String folder) async {
    if (folder.trim().isEmpty) {
      return const [];
    }
    final text = _CoreLibrary.instance.listReviews(folder);
    try {
      final decoded = jsonDecode(text) as Map<String, Object?>;
      final entries = decoded['reviews'] as List<Object?>? ?? const [];
      return entries
          .map((entry) => PendingReview.fromJson(entry as Map<String, Object?>))
          .toList();
    } on FormatException {
      return const [];
    }
  }

  // AI-FUNC-SUMMARY: Applies one decision to one pending review; returns an error message or null on success; side effects: renames or deletes files.
  Future<String?> resolve(String sidecarPath, String decision) async =>
      _CoreLibrary.instance.resolveReview(sidecarPath, decision);
}

// AI-FUNC-SUMMARY: Runs one conversion inside the worker isolate; returns none; side effects: loads the native library, runs the conversion, and reports back over the port.
void _workerMain(List<Object?> message) {
  final sendPort = message[0]! as SendPort;
  final request = RunRequest.fromWorkerMessage(
    Map<String, Object?>.from(message[1]! as Map),
  );

  final core = _CoreLibrary.instance;
  if (!core.isUsable) {
    sendPort.send({
      'kind': 'done',
      'error':
          'This copy of MyVidComp is missing its conversion engine. Reinstall the app.',
    });
    return;
  }

  final token = core.createToken();
  sendPort.send({'kind': 'ready', 'token': token.address});

  final callback = NativeCallable<_NativeEventCallback>.isolateLocal((
    Pointer<Char> json,
    Pointer<Void> _,
  ) {
    // The pointer is only valid for this call, so the text is copied at once.
    sendPort.send({'kind': 'event', 'json': json.cast<Utf8>().toDartString()});
  });

  try {
    final error = core.runBlocking(request, callback.nativeFunction, token);
    sendPort.send({'kind': 'done', 'error': error});
  } catch (error) {
    sendPort.send({'kind': 'done', 'error': error.toString()});
  } finally {
    callback.close();
  }
}
