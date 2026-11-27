import 'dart:async';

import 'package:flutter/material.dart';

import '../app_controller.dart';
import '../app_localizations.dart';
import '../formatting.dart';
import '../widgets.dart';

/// The screen that shows what is happening right now and what already happened.
class ProgressPage extends StatefulWidget {
  const ProgressPage({super.key, required this.controller});

  final AppController controller;

  @override
  State<ProgressPage> createState() => _ProgressPageState();
}

class _ProgressPageState extends State<ProgressPage> {
  FileOutcome? _filter;

  @override
  // AI-FUNC-SUMMARY: Builds the progress screen with inline stop control and outcome filters; returns the widget; side effects: none.
  Widget build(BuildContext context) {
    final text = AppText.of(context);
    final controller = widget.controller;

    if (!controller.isActive && controller.records.isEmpty) {
      return EmptyState(
        icon: Icons.timelapse_outlined,
        title: text.noActivityYet,
        message: text.statusReady,
      );
    }

    return PageBody(
      children: [
        _StatusCard(controller: controller),
        _Totals(controller: controller),
        if (controller.records.isNotEmpty)
          _FileList(
            controller: controller,
            filter: _filter,
            onFilterChanged: (next) => setState(() => _filter = next),
          ),
        if (controller.log.isNotEmpty) _DetailsLog(controller: controller),
      ],
    );
  }
}

class _StatusCard extends StatelessWidget {
  const _StatusCard({required this.controller});

  final AppController controller;

  @override
  // AI-FUNC-SUMMARY: Builds the current-status card with its progress bar; returns the widget; side effects: none.
  Widget build(BuildContext context) {
    final text = AppText.of(context);
    final theme = Theme.of(context);

    final label = switch (controller.stage) {
      RunStage.idle => text.statusReady,
      RunStage.starting => text.statusStarting,
      RunStage.scanning => text.statusScanning,
      RunStage.tuning => _phaseLabel(text, controller) ?? text.statusTuning,
      RunStage.converting || RunStage.measuring || RunStage.finishing =>
        _phaseLabel(text, controller) ??
            switch (controller.stage) {
              RunStage.converting => text.statusConverting,
              RunStage.measuring => text.statusMeasuring,
              _ => text.statusFinishing,
            },
      RunStage.done => text.statusDone,
      RunStage.stopped => text.statusStopped,
      RunStage.failed => text.statusFailed,
    };

    // Every phase of a file now reports where it has got to. Only the scan, and
    // the moment before the first file starts, have nothing to count.
    final indeterminate =
        controller.isActive &&
        (controller.stage == RunStage.scanning ||
            controller.stage == RunStage.starting);

    return SectionCard(
      title: label,
      subtitle: controller.fileIndex == null
          ? null
          : text.fileOfTotal(controller.fileIndex!, controller.totalFiles),
      leading: Icon(
        controller.isRunning
            ? Icons.autorenew
            : controller.stage == RunStage.failed
            ? Icons.error_outline
            : Icons.check_circle_outline,
      ),
      trailing: controller.isRunning
          ? OutlinedButton.icon(
              onPressed: controller.isStopping ? null : controller.stop,
              icon: const Icon(Icons.stop_rounded, size: 18),
              label: Text(controller.isStopping ? text.stopping : text.stop),
            )
          : null,
      children: [
        if (controller.currentFile.isNotEmpty) ...[
          Text(
            controller.currentFile,
            style: theme.textTheme.bodyMedium,
            maxLines: 2,
            overflow: TextOverflow.ellipsis,
          ),
          if (controller.isActive && controller.filePhases.isNotEmpty) ...[
            const SizedBox(height: 10),
            _PhaseSteps(controller: controller),
          ],
          const SizedBox(height: 12),
        ],
        LinearProgressIndicator(
          value: indeterminate
              ? null
              : (controller.fileProgress / 100).clamp(0.0, 1.0),
          minHeight: 8,
          borderRadius: BorderRadius.circular(999),
        ),
        if (controller.isActive) ...[
          const SizedBox(height: 6),
          _EverySecond(builder: (_) => _ProgressDetail(controller: controller)),
          const SizedBox(height: 14),
          Text(text.overallProgress, style: theme.textTheme.labelMedium),
          const SizedBox(height: 6),
          LinearProgressIndicator(
            value: (controller.overallProgress / 100).clamp(0.0, 1.0),
            minHeight: 4,
            borderRadius: BorderRadius.circular(999),
          ),
          const SizedBox(height: 6),
          _EverySecond(
            builder: (_) => Text(
              _runTimes(text, controller),
              style: theme.textTheme.bodySmall,
            ),
          ),
        ],
        if (controller.lastError != null) ...[
          const SizedBox(height: 14),
          Text(
            controller.lastError!,
            style: theme.textTheme.bodySmall?.copyWith(
              color: theme.colorScheme.error,
            ),
          ),
        ],
      ],
    );
  }
}

class _Totals extends StatelessWidget {
  const _Totals({required this.controller});

  final AppController controller;

  @override
  // AI-FUNC-SUMMARY: Builds the run totals; returns the widget; side effects: none.
  Widget build(BuildContext context) {
    final text = AppText.of(context);
    final saved = controller.sourceBytes - controller.outputBytes;

    return Padding(
      padding: const EdgeInsets.only(bottom: 16),
      child: Wrap(
        spacing: 10,
        runSpacing: 10,
        children: [
          CountTile(label: text.converted, value: '${controller.converted}'),
          CountTile(label: text.skipped, value: '${controller.skipped}'),
          if (controller.failed > 0)
            CountTile(label: text.failed, value: '${controller.failed}'),
          if (controller.reviewCount > 0)
            CountTile(
              label: text.needsReview,
              value: '${controller.reviewCount}',
              emphasis: true,
            ),
          if (saved > 0)
            CountTile(label: text.spaceSaved, value: formatBytes(saved)),
          if (!controller.isActive && controller.runElapsed != null)
            CountTile(
              label: text.totalTime,
              value: formatDuration(controller.runElapsed!),
            ),
        ],
      ),
    );
  }
}

class _FileList extends StatelessWidget {
  const _FileList({
    required this.controller,
    required this.filter,
    required this.onFilterChanged,
  });

  final AppController controller;
  final FileOutcome? filter;
  final ValueChanged<FileOutcome?> onFilterChanged;

  @override
  // AI-FUNC-SUMMARY: Builds the filterable list of files this run has finished with; returns the widget; side effects: none.
  Widget build(BuildContext context) {
    final text = AppText.of(context);
    final filtered = controller.records.reversed
        .where((record) => filter == null || record.outcome == filter)
        .take(200);

    return SectionCard(
      title: text.navActivity,
      leading: const Icon(Icons.list_alt_outlined),
      children: [
        Padding(
          padding: const EdgeInsets.only(bottom: 12),
          child: Wrap(
            spacing: 8,
            runSpacing: 8,
            children: [
              FilterChip(
                label: Text(text.filterAll),
                selected: filter == null,
                onSelected: (_) => onFilterChanged(null),
              ),
              FilterChip(
                label: Text(text.converted),
                selected: filter == FileOutcome.converted,
                onSelected: (_) => onFilterChanged(
                  filter == FileOutcome.converted
                      ? null
                      : FileOutcome.converted,
                ),
              ),
              FilterChip(
                label: Text(text.needsReview),
                selected: filter == FileOutcome.needsReview,
                onSelected: (_) => onFilterChanged(
                  filter == FileOutcome.needsReview
                      ? null
                      : FileOutcome.needsReview,
                ),
              ),
              FilterChip(
                label: Text(text.skipped),
                selected: filter == FileOutcome.skipped,
                onSelected: (_) => onFilterChanged(
                  filter == FileOutcome.skipped ? null : FileOutcome.skipped,
                ),
              ),
              FilterChip(
                label: Text(text.failed),
                selected: filter == FileOutcome.failed,
                onSelected: (_) => onFilterChanged(
                  filter == FileOutcome.failed ? null : FileOutcome.failed,
                ),
              ),
            ],
          ),
        ),
        for (final record in filtered) _FileRow(record: record, text: text),
      ],
    );
  }
}

class _FileRow extends StatelessWidget {
  const _FileRow({required this.record, required this.text});

  final FileRecord record;
  final AppText text;

  @override
  // AI-FUNC-SUMMARY: Builds one finished-file row; returns the widget; side effects: none.
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    final (icon, colour) = switch (record.outcome) {
      FileOutcome.converted => (
        Icons.check_circle_outline,
        theme.colorScheme.primary,
      ),
      FileOutcome.skipped => (
        Icons.remove_circle_outline,
        theme.colorScheme.onSurfaceVariant,
      ),
      FileOutcome.failed => (Icons.error_outline, theme.colorScheme.error),
      FileOutcome.needsReview => (
        Icons.help_outline,
        theme.colorScheme.tertiary,
      ),
    };

    return Padding(
      padding: const EdgeInsets.only(bottom: 12),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Icon(icon, size: 20, color: colour),
          const SizedBox(width: 10),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(record.name, style: theme.textTheme.bodyMedium),
                if (_detail(text, record).isNotEmpty)
                  Text(
                    _detail(text, record),
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: theme.colorScheme.onSurfaceVariant,
                    ),
                  ),
              ],
            ),
          ),
          if (record.score != null) ...[
            const SizedBox(width: 8),
            QualityBadge(
              score: record.score,
              notMeasuredLabel: text.qualityNotMeasured,
            ),
          ],
        ],
      ),
    );
  }
}

// AI-FUNC-SUMMARY: Writes what happened to one file in the reader's language; returns the sentence, falling back to the engine's words; side effects: none.
String _detail(AppText text, FileRecord record) {
  final source = record.sourceBytes;
  final output = record.outputBytes;
  if (record.outcome == FileOutcome.converted &&
      source != null &&
      output != null) {
    return text.sizeChange(source, output);
  }
  if (record.outcome == FileOutcome.skipped) {
    return text.skipReasonLabel(record.detailCode, record.detail);
  }
  return record.detail;
}

class _DetailsLog extends StatelessWidget {
  const _DetailsLog({required this.controller});

  final AppController controller;

  @override
  // AI-FUNC-SUMMARY: Purpose: Render the collapsible log with common card spacing; Inputs: context; Returns: Widget; Side effects: None; Notes: the visible log remains bounded.
  Widget build(BuildContext context) {
    final text = AppText.of(context);

    return Card(
      margin: const EdgeInsets.only(bottom: 16),
      child: ExpansionTile(
        title: Text(text.details),
        shape: const Border(),
        childrenPadding: const EdgeInsets.fromLTRB(16, 0, 16, 16),
        children: [
          ConstrainedBox(
            constraints: const BoxConstraints(maxHeight: 280),
            child: ListView.separated(
              shrinkWrap: true,
              itemCount: controller.log.length,
              separatorBuilder: (_, _) => const Divider(height: 12),
              itemBuilder: (context, index) =>
                  _LogLine(entry: controller.log[index]),
            ),
          ),
        ],
      ),
    );
  }
}

/// The numbers beside the current file's bar: which phase, how fast, how long.
class _ProgressDetail extends StatelessWidget {
  const _ProgressDetail({required this.controller});

  final AppController controller;

  @override
  // AI-FUNC-SUMMARY: Builds the line of live figures under the file's bar; returns the widget; side effects: none.
  Widget build(BuildContext context) {
    final text = AppText.of(context);
    final theme = Theme.of(context);

    final copied = controller.copiedBytes;
    final copyTotal = controller.copyTotalBytes;
    final parts = <String>[
      formatPercent(controller.fileProgress),
      if (controller.speed != null && controller.speed != 'N/A')
        formatSpeed(controller.speed!),
      if (copied != null && copyTotal != null && copyTotal > 0)
        text.copiedOfTotal(formatBytes(copied), formatBytes(copyTotal)),
      if (controller.fileStartedAt != null)
        text.elapsed(
          formatDuration(DateTime.now().difference(controller.fileStartedAt!)),
        ),
      if (controller.fileRemaining != null)
        text.timeLeft(formatDuration(controller.fileRemaining!)),
    ];

    return Text(
      parts.join('  ·  '),
      style: theme.textTheme.bodySmall?.copyWith(
        color: theme.colorScheme.onSurfaceVariant,
      ),
    );
  }
}

/// One line of the details log, written in the reader's own language.
class _LogLine extends StatelessWidget {
  const _LogLine({required this.entry});

  final LogEntry entry;

  @override
  // AI-FUNC-SUMMARY: Builds one details line with an icon for how serious it is; returns the widget; side effects: none.
  Widget build(BuildContext context) {
    final text = AppText.of(context);
    final theme = Theme.of(context);

    final message = switch (entry.kind) {
      LogKind.trial =>
        entry.setting == null
            ? text.tryingSetting(formatScore(entry.score ?? 0))
            : '${text.tryingSetting(formatScore(entry.score ?? 0))} '
                  '(${entry.setting})',
      LogKind.message => entry.message,
      LogKind.fileStarted => text.logFileStarted(
        entry.name ?? '',
        entry.index,
        entry.total,
      ),
      LogKind.phaseFinished => text.logPhaseFinished(
        _stepName(text, entry.phase ?? ''),
        formatDuration(entry.duration ?? Duration.zero),
      ),
      LogKind.fileFinished => _fileFinishedLine(text, entry),
      LogKind.runFinished => text.logRunFinished(
        formatDuration(entry.duration ?? Duration.zero),
        converted: entry.counts.converted,
        skipped: entry.counts.skipped,
        failed: entry.counts.failed,
      ),
    };

    final (icon, colour) = switch (entry.level) {
      LogLevel.error => (Icons.error_outline, theme.colorScheme.error),
      LogLevel.warning => (
        Icons.warning_amber_outlined,
        theme.colorScheme.tertiary,
      ),
      LogLevel.info => (Icons.info_outline, theme.colorScheme.onSurfaceVariant),
    };

    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Padding(
          padding: const EdgeInsets.only(top: 2, right: 8),
          child: Icon(icon, size: 14, color: colour),
        ),
        Padding(
          padding: const EdgeInsets.only(right: 8),
          child: Text(
            _clock(entry.time),
            style: theme.textTheme.bodySmall?.copyWith(
              color: theme.colorScheme.onSurfaceVariant,
              fontFeatures: const [FontFeature.tabularFigures()],
            ),
          ),
        ),
        Expanded(
          child: Text(
            message,
            style: theme.textTheme.bodySmall?.copyWith(
              color: entry.level == LogLevel.error ? colour : null,
            ),
          ),
        ),
      ],
    );
  }
}

/// The phases of the current file in order, marking what is done, what is
/// under way and what is still to come.
class _PhaseSteps extends StatelessWidget {
  const _PhaseSteps({required this.controller});

  final AppController controller;

  @override
  // AI-FUNC-SUMMARY: Builds the row of phase markers for the current file; returns the widget; side effects: none.
  Widget build(BuildContext context) {
    final text = AppText.of(context);
    final theme = Theme.of(context);
    final phases = controller.filePhases;
    final current = phases.indexOf(controller.phase ?? '');

    return Wrap(
      spacing: 12,
      runSpacing: 6,
      children: [
        for (var index = 0; index < phases.length; index++)
          _PhaseStep(
            label: _stepName(text, phases[index]),
            state: current < 0 || index > current
                ? _StepState.pending
                : index < current
                ? _StepState.done
                : _StepState.current,
            theme: theme,
          ),
      ],
    );
  }
}

enum _StepState { done, current, pending }

/// One marker in the row of phases.
class _PhaseStep extends StatelessWidget {
  const _PhaseStep({
    required this.label,
    required this.state,
    required this.theme,
  });

  final String label;
  final _StepState state;
  final ThemeData theme;

  @override
  // AI-FUNC-SUMMARY: Builds one phase marker with an icon for its state; returns the widget; side effects: none.
  Widget build(BuildContext context) {
    final scheme = theme.colorScheme;
    final (icon, colour) = switch (state) {
      _StepState.done => (Icons.check_circle, scheme.primary),
      _StepState.current => (Icons.radio_button_checked, scheme.primary),
      _StepState.pending => (Icons.radio_button_unchecked, scheme.outline),
    };
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        Icon(icon, size: 16, color: colour),
        const SizedBox(width: 4),
        Text(
          label,
          style: theme.textTheme.labelMedium?.copyWith(
            color: state == _StepState.pending
                ? scheme.onSurfaceVariant
                : scheme.onSurface,
            fontWeight: state == _StepState.current ? FontWeight.w700 : null,
          ),
        ),
      ],
    );
  }
}

// AI-FUNC-SUMMARY: Names what the current file is doing from the engine's phase, with the trial count while choosing; returns the label, or none when no phase is known; side effects: none.
String? _phaseLabel(AppText text, AppController controller) {
  final label = switch (controller.phase) {
    'choosing' => text.phaseChoosing,
    'encoding' => text.phaseEncoding,
    'measuring' => text.phaseMeasuring,
    'validating' => text.phaseValidating,
    'committing' =>
      controller.copyTotalBytes != null
          ? text.phaseCopying
          : text.phaseCommitting,
    _ => null,
  };
  if (label != null &&
      controller.phase == 'choosing' &&
      controller.trials > 0) {
    return '$label  ·  ${text.trialOfTotal(controller.trial, controller.trials)}';
  }
  return label;
}

// AI-FUNC-SUMMARY: Gives the short name of one phase for the phase row and the log; returns the name, or the engine's word for an unknown phase; side effects: none.
String _stepName(AppText text, String phase) => switch (phase) {
  'choosing' => text.stepChoosing,
  'encoding' => text.stepEncoding,
  'measuring' => text.stepMeasuring,
  'validating' => text.stepValidating,
  'committing' => text.stepCommitting,
  _ => phase,
};

// AI-FUNC-SUMMARY: Writes the log line for a finished file with its time in each phase; returns the sentence; side effects: none.
String _fileFinishedLine(AppText text, LogEntry entry) {
  final name = entry.name ?? '';
  final total = formatDuration(entry.duration ?? Duration.zero);
  final sentence = switch (entry.outcome) {
    FileOutcome.failed => text.logFileFailed(name, total),
    FileOutcome.needsReview => text.logFileKept(name, total),
    _ => text.logFileConverted(name, total),
  };
  if (entry.phaseTimes.isEmpty) {
    return sentence;
  }
  final parts = [
    for (final (phase, duration) in entry.phaseTimes)
      '${_stepName(text, phase)} ${formatDuration(duration)}',
  ];
  return '$sentence\n${parts.join('  ·  ')}';
}

// AI-FUNC-SUMMARY: Writes the run's time so far and time left; returns the line, or empty when neither is known; side effects: reads the clock.
String _runTimes(AppText text, AppController controller) {
  final started = controller.runStartedAt;
  return [
    if (started != null)
      text.elapsed(formatDuration(DateTime.now().difference(started))),
    if (controller.overallRemaining != null)
      text.timeLeft(formatDuration(controller.overallRemaining!)),
  ].join('  ·  ');
}

// AI-FUNC-SUMMARY: Formats the time of day a log line was written; returns hours, minutes and seconds; side effects: none.
String _clock(DateTime time) =>
    '${time.hour.toString().padLeft(2, '0')}:'
    '${time.minute.toString().padLeft(2, '0')}:'
    '${time.second.toString().padLeft(2, '0')}';

/// Rebuilds its child once a second, so the time spent keeps moving even
/// through a phase that sends no progress of its own.
class _EverySecond extends StatefulWidget {
  const _EverySecond({required this.builder});

  final WidgetBuilder builder;

  @override
  State<_EverySecond> createState() => _EverySecondState();
}

class _EverySecondState extends State<_EverySecond> {
  late final Timer _timer;

  @override
  // AI-FUNC-SUMMARY: Starts the one-second timer; returns none; side effects: schedules rebuilds.
  void initState() {
    super.initState();
    _timer = Timer.periodic(const Duration(seconds: 1), (_) => setState(() {}));
  }

  @override
  // AI-FUNC-SUMMARY: Stops the timer; returns none; side effects: cancels scheduled rebuilds.
  void dispose() {
    _timer.cancel();
    super.dispose();
  }

  @override
  // AI-FUNC-SUMMARY: Builds the child for the current second; returns the widget; side effects: none.
  Widget build(BuildContext context) => widget.builder(context);
}
