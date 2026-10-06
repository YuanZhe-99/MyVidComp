import 'package:flutter/material.dart';
import 'package:myapps_ui/myapps_ui.dart';

/// A titled block of related settings.
class SectionCard extends StatelessWidget {
  // AI-FUNC-SUMMARY: Purpose: Bind a themed business section; Inputs: title, content and optional heading details; Returns: SectionCard; Side effects: None; Notes: shared headings own typography.
  const SectionCard({
    super.key,
    required this.title,
    required this.children,
    this.subtitle,
    this.leading,
  });

  final String title;
  final String? subtitle;
  final Widget? leading;
  final List<Widget> children;

  @override
  // AI-FUNC-SUMMARY: Purpose: Render a business card with shared section styling; Inputs: context; Returns: Widget; Side effects: None; Notes: shapes and surfaces come from the shared theme.
  Widget build(BuildContext context) {
    return Card(
      margin: const EdgeInsets.only(bottom: 16),
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: MyAppsSettingsSection(
          title: title,
          headingPadding: const EdgeInsets.only(bottom: 16),
          children: [
            if (leading != null || subtitle != null)
              ListTile(
                contentPadding: EdgeInsets.zero,
                leading: leading,
                subtitle: subtitle == null ? null : Text(subtitle!),
              ),
            ...children,
          ],
        ),
      ),
    );
  }
}

/// One choice a person can make, shown as a row of options with the meaning of
/// the selected one spelled out underneath.
class ChoiceField<T> extends StatelessWidget {
  const ChoiceField({
    super.key,
    required this.label,
    required this.value,
    required this.values,
    required this.labelFor,
    required this.onChanged,
    this.helpFor,
    this.enabled = true,
  });

  final String label;
  final T value;
  final List<T> values;
  final String Function(T value) labelFor;
  final String Function(T value)? helpFor;
  final ValueChanged<T> onChanged;
  final bool enabled;

  @override
  // AI-FUNC-SUMMARY: Purpose: Render an inline choice with shared text-fit fallback; Inputs: context; Returns: Widget; Side effects: None; Notes: all option counts prefer segments, including narrow panes.
  Widget build(BuildContext context) {
    return MyAppsSettingsChoice<T>(
      label: label,
      value: value,
      values: values,
      labelFor: labelFor,
      onChanged: onChanged,
      helpFor: helpFor,
      enabled: enabled,
      segmentMinWidth: 0,
      maxSegments: values.length,
    );
  }
}

/// A labelled text box with optional helper text and a trailing action.
class LabelledField extends StatelessWidget {
  const LabelledField({
    super.key,
    required this.label,
    required this.controller,
    this.hint,
    this.help,
    this.enabled = true,
    this.keyboardType,
    this.trailing,
    this.onChanged,
  });

  final String label;
  final TextEditingController controller;
  final String? hint;
  final String? help;
  final bool enabled;
  final TextInputType? keyboardType;
  final Widget? trailing;
  final ValueChanged<String>? onChanged;

  @override
  // AI-FUNC-SUMMARY: Purpose: Render a text field using shared input styling; Inputs: context; Returns: Widget; Side effects: None; Notes: narrow or scaled layouts place trailing actions below the field.
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.only(bottom: 18),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(label, style: theme.textTheme.labelLarge),
          const SizedBox(height: 8),
          LayoutBuilder(
            builder: (context, constraints) {
              final field = TextField(
                controller: controller,
                enabled: enabled,
                keyboardType: keyboardType,
                onChanged: onChanged,
                decoration: InputDecoration(hintText: hint),
              );
              if (trailing != null &&
                  constraints.maxWidth <
                      480 * MediaQuery.textScalerOf(context).scale(1)) {
                return Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [field, const SizedBox(height: 8), trailing!],
                );
              }
              return Row(
                children: [
                  Expanded(child: field),
                  if (trailing != null) ...[
                    const SizedBox(width: 8),
                    trailing!,
                  ],
                ],
              );
            },
          ),
          if (help != null) ...[
            const SizedBox(height: 8),
            Text(
              help!,
              style: theme.textTheme.bodySmall?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
              ),
            ),
          ],
        ],
      ),
    );
  }
}

/// A short coloured summary of a measured quality score.
class QualityBadge extends StatelessWidget {
  const QualityBadge({
    super.key,
    required this.score,
    required this.notMeasuredLabel,
  });

  /// Measured quality in hundredths, or null when it was not measured.
  final int? score;
  final String notMeasuredLabel;

  @override
  // AI-FUNC-SUMMARY: Builds the quality badge; returns the widget; side effects: none.
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final value = score;

    if (value == null) {
      return _chip(
        context,
        notMeasuredLabel,
        theme.colorScheme.surfaceContainerHighest,
        theme.colorScheme.onSurfaceVariant,
      );
    }

    // Green at or above 95, amber down to 90, red below: the same thresholds
    // the quality presets describe.
    final (background, foreground) = switch (value) {
      >= 9500 => (
        theme.colorScheme.primaryContainer,
        theme.colorScheme.onPrimaryContainer,
      ),
      >= 9000 => (
        theme.colorScheme.tertiaryContainer,
        theme.colorScheme.onTertiaryContainer,
      ),
      _ => (
        theme.colorScheme.errorContainer,
        theme.colorScheme.onErrorContainer,
      ),
    };

    return _chip(
      context,
      (value / 100).toStringAsFixed(1),
      background,
      foreground,
    );
  }

  // AI-FUNC-SUMMARY: Builds one small rounded label; returns the widget; side effects: none.
  Widget _chip(
    BuildContext context,
    String text,
    Color background,
    Color foreground,
  ) => Container(
    padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 4),
    decoration: BoxDecoration(
      color: background,
      borderRadius: BorderRadius.circular(999),
    ),
    child: Text(
      text,
      style: Theme.of(
        context,
      ).textTheme.labelMedium?.copyWith(color: foreground),
    ),
  );
}

/// A large number with a caption, used for the run totals.
class CountTile extends StatelessWidget {
  const CountTile({
    super.key,
    required this.label,
    required this.value,
    this.emphasis = false,
  });

  final String label;
  final String value;
  final bool emphasis;

  @override
  // AI-FUNC-SUMMARY: Purpose: Render a themed summary card; Inputs: context; Returns: Widget; Side effects: None; Notes: shared theme owns card shape.
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Card(
      margin: EdgeInsets.zero,
      color: emphasis
          ? theme.colorScheme.primaryContainer
          : theme.colorScheme.surfaceContainerHighest,
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          mainAxisSize: MainAxisSize.min,
          children: [
            Text(
              value,
              style: theme.textTheme.headlineSmall?.copyWith(
                fontWeight: FontWeight.w700,
                color: emphasis ? theme.colorScheme.onPrimaryContainer : null,
              ),
            ),
            Text(
              label,
              style: theme.textTheme.labelMedium?.copyWith(
                color: emphasis
                    ? theme.colorScheme.onPrimaryContainer
                    : theme.colorScheme.onSurfaceVariant,
              ),
            ),
          ],
        ),
      ),
    );
  }
}

/// A friendly placeholder for a screen with nothing on it yet.
class EmptyState extends StatelessWidget {
  const EmptyState({
    super.key,
    required this.icon,
    required this.title,
    required this.message,
    this.action,
  });

  final IconData icon;
  final String title;
  final String message;
  final Widget? action;

  @override
  // AI-FUNC-SUMMARY: Builds the empty-screen placeholder; returns the widget; side effects: none.
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Center(
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 420),
        child: Padding(
          padding: const EdgeInsets.all(32),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(icon, size: 48, color: theme.colorScheme.primary),
              const SizedBox(height: 16),
              Text(
                title,
                textAlign: TextAlign.center,
                style: theme.textTheme.titleMedium,
              ),
              const SizedBox(height: 8),
              Text(
                message,
                textAlign: TextAlign.center,
                style: theme.textTheme.bodyMedium?.copyWith(
                  color: theme.colorScheme.onSurfaceVariant,
                ),
              ),
              if (action != null) ...[const SizedBox(height: 20), action!],
            ],
          ),
        ),
      ),
    );
  }
}

/// Keeps page content readable on very wide windows.
class PageBody extends StatelessWidget {
  const PageBody({super.key, required this.children, this.maxWidth = 720});

  final List<Widget> children;
  final double maxWidth;

  @override
  // AI-FUNC-SUMMARY: Purpose: Render readable scrolling content with common spacing; Inputs: context; Returns: Widget; Side effects: None; Notes: parent SafeArea reserves floating navigation space.
  Widget build(BuildContext context) => Align(
    alignment: Alignment.topCenter,
    child: ConstrainedBox(
      constraints: BoxConstraints(maxWidth: maxWidth),
      child: ListView(padding: const EdgeInsets.all(16), children: children),
    ),
  );
}
