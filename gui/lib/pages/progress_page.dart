import 'package:flutter/material.dart';

import '../app_controller.dart';
import '../app_localizations.dart';
import '../formatting.dart';
import '../widgets.dart';

/// The screen that shows what is happening right now and what already happened.
class ProgressPage extends StatelessWidget {
  const ProgressPage({super.key, required this.controller});

  final AppController controller;

  @override
  // AI-FUNC-SUMMARY: Builds the progress screen; returns the widget; side effects: none.
  Widget build(BuildContext context) {
    final text = AppText.of(context);

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
        if (controller.records.isNotEmpty) _FileList(controller: controller),
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
      RunStage.tuning => text.statusTuning,
      RunStage.converting => text.statusConverting,
      RunStage.measuring => text.statusMeasuring,
      RunStage.finishing => text.statusFinishing,
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
      children: [
        if (controller.currentFile.isNotEmpty) ...[
          Text(
            controller.currentFile,
            style: theme.textTheme.bodyMedium,
            maxLines: 2,
            overflow: TextOverflow.ellipsis,
          ),
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
          _ProgressDetail(controller: controller),
          const SizedBox(height: 14),
          Text(text.overallProgress, style: theme.textTheme.labelMedium),
          const SizedBox(height: 6),
          LinearProgressIndicator(
            value: (controller.overallProgress / 100).clamp(0.0, 1.0),
            minHeight: 4,
            borderRadius: BorderRadius.circular(999),
          ),
          if (controller.overallRemaining != null) ...[
            const SizedBox(height: 6),
            Text(
              text.timeLeft(formatDuration(controller.overallRemaining!)),
              style: theme.textTheme.bodySmall,
            ),
          ],
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
        ],
      ),
    );
  }
}

class _FileList extends StatelessWidget {
  const _FileList({required this.controller});

  final AppController controller;

  @override
  // AI-FUNC-SUMMARY: Builds the list of files this run has finished with; returns the widget; side effects: none.
  Widget build(BuildContext context) {
    final text = AppText.of(context);

    return SectionCard(
      title: text.navActivity,
      leading: const Icon(Icons.list_alt_outlined),
      children: [
        for (final record in controller.records.reversed.take(200))
          _FileRow(record: record, text: text),
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
  // AI-FUNC-SUMMARY: Builds the collapsible technical log; returns the widget; side effects: none.
  Widget build(BuildContext context) {
    final text = AppText.of(context);

    return Card(
      margin: const EdgeInsets.only(bottom: 16),
      child: ExpansionTile(
        title: Text(text.details),
        shape: const Border(),
        childrenPadding: const EdgeInsets.fromLTRB(20, 0, 20, 16),
        children: [
          ConstrainedBox(
            constraints: const BoxConstraints(maxHeight: 280),
            child: ListView.separated(
              shrinkWrap: true,
              itemCount: controller.log.length.clamp(0, 200),
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

    final parts = <String>[
      formatPercent(controller.fileProgress),
      if (controller.stage == RunStage.tuning && controller.trials > 0)
        text.trialOfTotal(controller.trial, controller.trials),
      if (controller.speed != null && controller.speed != 'N/A')
        formatSpeed(controller.speed!),
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
