import 'dart:convert';
import 'dart:io';

import 'package:flutter/services.dart';

/// Picks a folder using whatever the platform offers.
///
/// Windows gets its native folder dialog through a short-lived helper process,
/// which avoids a plugin dependency. Android has no dialog that returns a real
/// path, so the caller falls back to the in-app browser in `folder_browser.dart`.
class FolderPicker {
  const FolderPicker();

  static const MethodChannel _androidChannel = MethodChannel(
    'myvidcomp/native',
  );

  /// True when [pickFolder] can show something. Elsewhere the caller should use
  /// the in-app browser or a typed path.
  bool get hasNativeDialog => Platform.isWindows;

  // AI-FUNC-SUMMARY: Shows the platform folder dialog; returns the chosen folder, or null when there is no dialog or the user cancelled; side effects: runs a helper process on Windows.
  Future<String?> pickFolder({String? startIn}) async {
    if (!Platform.isWindows) {
      return null;
    }
    return _pickOnWindows(startIn);
  }

  // AI-FUNC-SUMMARY: Shows the Windows folder dialog through a helper process; returns the chosen folder or null; side effects: starts PowerShell briefly.
  Future<String?> _pickOnWindows(String? startIn) async {
    final initial = (startIn ?? '').replaceAll("'", "''");
    final script =
        '''
Add-Type -AssemblyName System.Windows.Forms | Out-Null
\$dialog = New-Object System.Windows.Forms.FolderBrowserDialog
\$dialog.Description = 'Choose the folder holding your videos'
\$dialog.ShowNewFolderButton = \$false
if ('$initial' -ne '') { \$dialog.SelectedPath = '$initial' }
if (\$dialog.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {
  [Console]::Out.Write(\$dialog.SelectedPath)
}
''';

    try {
      final result = await Process.run('powershell', [
        '-NoProfile',
        '-STA',
        '-NonInteractive',
        '-ExecutionPolicy',
        'Bypass',
        '-Command',
        script,
      ], stdoutEncoding: const SystemEncoding());
      final path = (result.stdout as String).trim();
      return path.isEmpty ? null : path;
    } on ProcessException {
      return null;
    }
  }

  // AI-FUNC-SUMMARY: Asks Android for permission to read every file, which the engine needs to work with real paths; returns true when it was granted; side effects: may open the system permission screen.
  Future<bool> ensureAndroidFileAccess() async {
    if (!Platform.isAndroid) {
      return true;
    }
    try {
      final granted = await _androidChannel.invokeMethod<bool>(
        'ensureFileAccess',
      );
      return granted ?? false;
    } on PlatformException {
      return false;
    } on MissingPluginException {
      return false;
    }
  }

  // AI-FUNC-SUMMARY: Lists the storage volumes an Android device offers; returns their real paths, most useful first; side effects: asks the platform.
  Future<List<String>> androidStorageRoots() async {
    if (!Platform.isAndroid) {
      return const [];
    }
    try {
      final roots = await _androidChannel.invokeMethod<List<Object?>>(
        'storageRoots',
      );
      return (roots ?? const []).whereType<String>().toList();
    } on PlatformException {
      return const [];
    } on MissingPluginException {
      return const [];
    }
  }
}

/// Reads folders for the in-app browser used where no native dialog exists.
class FolderBrowser {
  const FolderBrowser();

  // AI-FUNC-SUMMARY: Lists the folders directly inside one folder; returns them sorted by name, skipping anything unreadable; side effects: reads the directory.
  Future<List<Directory>> childFolders(String path) async {
    try {
      final entries = await Directory(path).list(followLinks: false).toList();
      final folders = entries.whereType<Directory>().toList();
      folders.sort(
        (left, right) => _name(
          left.path,
        ).toLowerCase().compareTo(_name(right.path).toLowerCase()),
      );
      // Hidden folders are noise for someone looking for their videos.
      return folders
          .where((folder) => !_name(folder.path).startsWith('.'))
          .toList();
    } on FileSystemException {
      return const [];
    }
  }

  // AI-FUNC-SUMMARY: Reports the name of the folder at a path; returns the last path segment; side effects: none.
  static String _name(String path) {
    final trimmed = path.endsWith(Platform.pathSeparator)
        ? path.substring(0, path.length - 1)
        : path;
    final index = trimmed.lastIndexOf(Platform.pathSeparator);
    return index < 0 ? trimmed : trimmed.substring(index + 1);
  }

  // AI-FUNC-SUMMARY: Reports the folder containing another; returns the parent path, or null at the top; side effects: none.
  static String? parentOf(String path) {
    final parent = Directory(path).parent.path;
    return parent == path ? null : parent;
  }

  // AI-FUNC-SUMMARY: Reports the display name of a folder; returns its last segment, or the whole path at the top; side effects: none.
  static String displayName(String path) {
    final name = _name(path);
    return name.isEmpty ? path : name;
  }
}

// AI-FUNC-SUMMARY: Reads a folder path out of a helper process's output; returns the trimmed path or null; side effects: none.
String? decodeFolderResult(String raw) {
  final trimmed = const LineSplitter()
      .convert(raw)
      .map((line) => line.trim())
      .where((line) => line.isNotEmpty)
      .join();
  return trimmed.isEmpty ? null : trimmed;
}
