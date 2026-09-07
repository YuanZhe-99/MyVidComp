/// Turning the engine's numbers into something a person reads.
///
/// The engine reports scores in hundredths, sizes in bytes, times in whole
/// seconds and speed as ffmpeg prints it. Every one of those is formatted here,
/// so the same number never appears two different ways in two places.
library;

// AI-FUNC-SUMMARY: Formats a hundredths score; returns a one-decimal string; side effects: none.
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
  final digits = value >= 10 ? 0 : 1;
  return '${value.toStringAsFixed(digits)} ${units[unit]}';
}

// AI-FUNC-SUMMARY: Formats a percentage for display beside a bar; returns the text; side effects: none.
String formatPercent(double percent) => '${percent.clamp(0, 100).round()}%';

// AI-FUNC-SUMMARY: Formats a length of time, dropping the hours when there are none; returns the text; side effects: none.
String formatDuration(Duration duration) {
  final total = duration.inSeconds;
  final hours = total ~/ 3600;
  final minutes = (total % 3600) ~/ 60;
  final seconds = total % 60;
  final paddedSeconds = seconds.toString().padLeft(2, '0');
  if (hours > 0) {
    return '$hours:${minutes.toString().padLeft(2, '0')}:$paddedSeconds';
  }
  return '$minutes:$paddedSeconds';
}

// AI-FUNC-SUMMARY: Formats the speed ffmpeg reports, which arrives as text like "1.85x"; returns the text with a multiplication sign; side effects: none.
String formatSpeed(String speed) {
  final trimmed = speed.trim();
  if (trimmed.isEmpty || trimmed == 'N/A') {
    return '';
  }
  final value = double.tryParse(trimmed.replaceAll('x', ''));
  if (value == null) {
    return trimmed;
  }
  final digits = value >= 10 ? 0 : 1;
  return '${value.toStringAsFixed(digits)}×';
}
