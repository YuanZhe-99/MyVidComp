import 'dart:io';

import 'native_paths.dart';

/// Whether the FFmpeg tools the app needs were found.
enum ToolState { checking, found, missing }

/// One located tool, and where it came from.
class MediaTool {
  const MediaTool({required this.state, required this.path});

  const MediaTool.missing() : state = ToolState.missing, path = '';

  final ToolState state;
  final String path;

  bool get isReady => state == ToolState.found;
}

/// The pair of tools the conversion engine runs.
class MediaTools {
  const MediaTools({required this.ffmpeg, required this.ffprobe});

  const MediaTools.unknown()
    : ffmpeg = const MediaTool.missing(),
      ffprobe = const MediaTool.missing();

  final MediaTool ffmpeg;
  final MediaTool ffprobe;

  bool get areReady => ffmpeg.isReady && ffprobe.isReady;

  // AI-FUNC-SUMMARY: Finds the FFmpeg tools, preferring anything the user chose; returns what was found; side effects: reads the filesystem and, off Windows, runs each tool once.
  static Future<MediaTools> detect({
    String ffmpegOverride = '',
    String ffprobeOverride = '',
  }) async {
    final ffmpeg = await _find('ffmpeg', ffmpegOverride);
    final ffprobe = await _find('ffprobe', ffprobeOverride);
    return MediaTools(ffmpeg: ffmpeg, ffprobe: ffprobe);
  }

  // AI-FUNC-SUMMARY: Finds one tool by name; returns where it is, or that it is missing; side effects: reads the filesystem and may run the tool.
  static Future<MediaTool> _find(String name, String override) async {
    if (override.trim().isNotEmpty) {
      return _check(override.trim());
    }

    for (final candidate in await _candidates(name)) {
      final found = await _check(candidate);
      if (found.isReady) {
        return found;
      }
    }
    return const MediaTool.missing();
  }

  // AI-FUNC-SUMMARY: Confirms one candidate path really is a usable tool; returns the result; side effects: checks the file exists and, off Windows, runs it.
  static Future<MediaTool> _check(String path) async {
    final file = File(path);
    if (!file.existsSync()) {
      return const MediaTool.missing();
    }

    // On Windows the tool is deliberately not run here: starting a console
    // program from the interface flashes a black window on screen.
    if (Platform.isWindows) {
      return MediaTool(state: ToolState.found, path: file.absolute.path);
    }

    try {
      final result = await Process.run(path, ['-version']);
      if (result.exitCode == 0) {
        return MediaTool(state: ToolState.found, path: file.absolute.path);
      }
    } on ProcessException {
      return const MediaTool.missing();
    }
    return const MediaTool.missing();
  }

  // AI-FUNC-SUMMARY: Lists the places one tool might be, nearest first; returns the candidate paths; side effects: reads the executable path, working directory and PATH.
  static Future<List<String>> _candidates(String name) async {
    final separator = Platform.pathSeparator;
    final paths = <String>[];

    // Android can only run an executable out of the folder the system unpacks
    // native libraries into, where they are shipped under library names.
    if (Platform.isAndroid) {
      final nativeDir = await androidNativeLibraryDir();
      if (nativeDir != null) {
        paths.add('$nativeDir${separator}lib$name.so');
      }
      return paths;
    }

    final fileNames = Platform.isWindows ? ['$name.exe', name] : [name];
    final roots = <String>[
      File(Platform.resolvedExecutable).parent.path,
      '${File(Platform.resolvedExecutable).parent.path}${separator}bin',
      Directory.current.path,
      '${Directory.current.path}${separator}bin',
      ...(Platform.environment['PATH'] ?? '').split(
        Platform.isWindows ? ';' : ':',
      ),
    ];

    final seen = <String>{};
    for (final root in roots) {
      if (root.trim().isEmpty) {
        continue;
      }
      for (final fileName in fileNames) {
        final candidate = '$root$separator$fileName';
        if (seen.add(candidate.toLowerCase())) {
          paths.add(candidate);
        }
      }
    }
    return paths;
  }
}
