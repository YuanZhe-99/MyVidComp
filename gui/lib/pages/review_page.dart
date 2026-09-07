import 'package:flutter/material.dart';

import '../app_controller.dart';
import '../app_localizations.dart';
import '../formatting.dart';
import '../core_ffi.dart';
import '../widgets.dart';

/// The screen where the user decides which copy of a converted video to keep.
class ReviewPage extends StatelessWidget {
  const ReviewPage({super.key, required this.controller});

  final AppController controller;

  @override
  // AI-FUNC-SUMMARY: Builds the review screen; returns the widget; side effects: none.
  Widget build(BuildContext context) {
    final text = AppText.of(context);

    if (controller.loadingReviews && controller.reviews.isEmpty) {
      return const Center(child: CircularProgressIndicator());
    }

    if (controller.reviews.isEmpty) {
      return EmptyState(
        icon: Icons.done_all,
        title: text.reviewEmpty,
        message: text.reviewEmptyHelp,
        action: OutlinedButton.icon(
          onPressed: controller.refreshReviews,
          icon: const Icon(Icons.refresh),
          label: Text(text.refresh),
        ),
      );
    }

    return PageBody(
      children: [
        _Header(controller: controller),
        for (final review in controller.reviews)
          _ReviewCard(controller: controller, review: review),
      ],
    );
  }
}

class _Header extends StatelessWidget {
  const _Header({required this.controller});

  final AppController controller;

  @override
  // AI-FUNC-SUMMARY: Builds the review heading and the apply-to-all actions; returns the widget; side effects: none.
  Widget build(BuildContext context) {
    final text = AppText.of(context);

    return SectionCard(
      title: text.reviewsWaiting(controller.reviews.length),
      subtitle: text.reviewIntro,
      leading: const Icon(Icons.rule),
      children: [
        Wrap(
          spacing: 8,
          runSpacing: 8,
          children: [
            OutlinedButton.icon(
              onPressed: controller.refreshReviews,
              icon: const Icon(Icons.refresh),
              label: Text(text.refresh),
            ),
            OutlinedButton(
              onPressed: () =>
                  _applyToAll(context, controller, 'keep-new', text.keepNew),
              child: Text('${text.applyToAll}: ${text.keepNew}'),
            ),
            OutlinedButton(
              onPressed: () => _applyToAll(
                context,
                controller,
                'keep-original',
                text.keepOriginal,
              ),
              child: Text('${text.applyToAll}: ${text.keepOriginal}'),
            ),
          ],
        ),
      ],
    );
  }
}

// AI-FUNC-SUMMARY: Confirms a bulk decision and applies it to every pending conversion; returns when it finishes; side effects: shows a dialog, then renames or deletes files.
Future<void> _applyToAll(
  BuildContext context,
  AppController controller,
  String decision,
  String label,
) async {
  final text = AppText.of(context);
  final confirmed = await showDialog<bool>(
    context: context,
    builder: (context) => AlertDialog(
      title: Text('${text.applyToAll}: $label'),
      content: Text(text.reviewsWaiting(controller.reviews.length)),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(false),
          child: Text(MaterialLocalizations.of(context).cancelButtonLabel),
        ),
        FilledButton(
          onPressed: () => Navigator.of(context).pop(true),
          child: Text(MaterialLocalizations.of(context).okButtonLabel),
        ),
      ],
    ),
  );

  if (confirmed != true || !context.mounted) {
    return;
  }

  final error = await controller.resolveAllReviews(decision);
  if (error != null && context.mounted) {
    ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(error)));
  }
}

class _ReviewCard extends StatelessWidget {
  const _ReviewCard({required this.controller, required this.review});

  final AppController controller;
  final PendingReview review;

  @override
  // AI-FUNC-SUMMARY: Builds one pending conversion with its three choices; returns the widget; side effects: none.
  Widget build(BuildContext context) {
    final text = AppText.of(context);
    final theme = Theme.of(context);
    final percent = review.sizePercent;

    return Card(
      margin: const EdgeInsets.only(bottom: 16),
      child: Padding(
        padding: const EdgeInsets.fromLTRB(20, 18, 20, 12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Expanded(
                  child: Text(
                    review.fileName,
                    style: theme.textTheme.titleMedium,
                  ),
                ),
                const SizedBox(width: 12),
                QualityBadge(
                  score: review.score,
                  notMeasuredLabel: text.qualityNotMeasured,
                ),
              ],
            ),
            const SizedBox(height: 10),
            Wrap(
              spacing: 16,
              runSpacing: 4,
              children: [
                Text(
                  '${formatBytes(review.originalBytes)} → ${formatBytes(review.reviewBytes)}',
                  style: theme.textTheme.bodySmall?.copyWith(
                    color: theme.colorScheme.onSurfaceVariant,
                  ),
                ),
                if (percent != null)
                  Text(
                    text.percentOfOriginal(percent.toStringAsFixed(0)),
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: theme.colorScheme.onSurfaceVariant,
                    ),
                  ),
              ],
            ),
            if (review.deviations.isNotEmpty) ...[
              const SizedBox(height: 14),
              Text(text.whatChanged, style: theme.textTheme.labelLarge),
              const SizedBox(height: 6),
              for (final change in review.deviations)
                Padding(
                  padding: const EdgeInsets.only(bottom: 4),
                  child: Row(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Icon(
                        Icons.info_outline,
                        size: 16,
                        color: theme.colorScheme.onSurfaceVariant,
                      ),
                      const SizedBox(width: 8),
                      Expanded(
                        child: Text(
                          change,
                          style: theme.textTheme.bodySmall?.copyWith(
                            color: theme.colorScheme.onSurfaceVariant,
                          ),
                        ),
                      ),
                    ],
                  ),
                ),
            ],
            const SizedBox(height: 16),
            _Decisions(controller: controller, review: review),
          ],
        ),
      ),
    );
  }
}

class _Decisions extends StatelessWidget {
  const _Decisions({required this.controller, required this.review});

  final AppController controller;
  final PendingReview review;

  @override
  // AI-FUNC-SUMMARY: Builds the three decision buttons for one conversion; returns the widget; side effects: none.
  Widget build(BuildContext context) {
    final text = AppText.of(context);

    return Wrap(
      spacing: 8,
      runSpacing: 8,
      children: [
        FilledButton(
          onPressed: () => _resolve(context, 'keep-new'),
          child: Text(text.keepNew),
        ),
        OutlinedButton(
          onPressed: () => _resolve(context, 'keep-original'),
          child: Text(text.keepOriginal),
        ),
        TextButton(
          onPressed: () => _resolve(context, 'keep-both'),
          child: Text(text.keepBoth),
        ),
      ],
    );
  }

  // AI-FUNC-SUMMARY: Applies one decision and reports any problem; returns when it finishes; side effects: renames or deletes files.
  Future<void> _resolve(BuildContext context, String decision) async {
    final error = await controller.resolveReview(review, decision);
    if (error != null && context.mounted) {
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text(error)));
    }
  }
}
