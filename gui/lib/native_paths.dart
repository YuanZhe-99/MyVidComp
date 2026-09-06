import 'dart:ffi';
import 'dart:io';

import 'package:flutter/services.dart';

/// Where the app finds the conversion engine and the media tools it runs.
///
/// On desktop everything sits beside the executable. On Android an executable
/// can only be launched from the folder the system unpacks native libraries
/// into, so `ffmpeg` and `ffprobe` are shipped as `libffmpeg.so` and
/// `libffprobe.so` and run from there.
const MethodChannel _androidChannel = MethodChannel('myvidcomp/native');

String? _cachedNativeLibraryDir;

// AI-FUNC-SUMMARY: Asks Android where the app unpacked its native libraries; returns the folder or null off Android; side effects: calls into the platform once and caches the answer.
Future<String?> androidNativeLibraryDir() async {
  if (!Platform.isAndroid) {
    return null;
  }
  if (_cachedNativeLibraryDir != null) {
    return _cachedNativeLibraryDir;
  }
  try {
    final dir = await _androidChannel.invokeMethod<String>('nativeLibraryDir');
    _cachedNativeLibraryDir = dir;
    return dir;
  } on PlatformException {
    return null;
  } on MissingPluginException {
    return null;
  }
}

// AI-FUNC-SUMMARY: Reports the file name of the conversion engine on this platform; returns the library file name; side effects: none.
String coreLibraryFileName() {
  if (Platform.isWindows) {
    return 'myvidcomp_core.dll';
  }
  if (Platform.isMacOS) {
    return 'libmyvidcomp_core.dylib';
  }
  return 'libmyvidcomp_core.so';
}

// AI-FUNC-SUMMARY: Loads the conversion engine; returns the opened library; side effects: loads a native library into the process.
DynamicLibrary openCoreLibrary() {
  final fileName = coreLibraryFileName();

  // Android resolves a bare name against the app's own library folder, which
  // is the only place it will load one from.
  if (Platform.isAndroid) {
    return DynamicLibrary.open(fileName);
  }

  final attempted = <String>[];
  for (final candidate in _coreLibraryCandidates(fileName)) {
    attempted.add(candidate);
    try {
      return DynamicLibrary.open(candidate);
    } on ArgumentError {
      continue;
    }
  }

  throw StateError(
    'MyVidComp could not load its conversion engine ($fileName). '
    'Looked in: ${attempted.join(', ')}.',
  );
}

// AI-FUNC-SUMMARY: Lists the places the conversion engine may sit, nearest first; returns candidate paths; side effects: reads the executable and working directory paths.
List<String> _coreLibraryCandidates(String fileName) {
  final separator = Platform.pathSeparator;
  final executableDir = File(Platform.resolvedExecutable).parent.path;
  final current = Directory.current.path;

  return [
    // Installed beside the app, which is how every release is packaged.
    '$executableDir$separator$fileName',
    // A bare name lets the operating system search its own paths.
    fileName,
    '$current$separator$fileName',
    // Running from a source checkout during development.
    '$current${separator}target${separator}release$separator$fileName',
    '$current$separator..${separator}target${separator}release$separator$fileName',
    '$current${separator}target${separator}debug$separator$fileName',
    '$current$separator..${separator}target${separator}debug$separator$fileName',
  ];
}
