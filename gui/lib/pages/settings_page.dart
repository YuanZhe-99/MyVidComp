import 'package:flutter/material.dart';

import '../app_controller.dart';
import '../app_localizations.dart';
import '../app_settings.dart';
import '../widgets.dart';

/// Everything that is not part of the everyday flow.
class SettingsPage extends StatefulWidget {
  const SettingsPage({
    super.key,
    required this.controller,
    required this.onLanguageChanged,
  });

  final AppController controller;
  final ValueChanged<String> onLanguageChanged;

  @override
  State<SettingsPage> createState() => _SettingsPageState();
}

class _SettingsPageState extends State<SettingsPage> {
  final TextEditingController _limit = TextEditingController();
  final TextEditingController _workingFolder = TextEditingController();
  final TextEditingController _ffmpeg = TextEditingController();
  final TextEditingController _ffprobe = TextEditingController();

  @override
  // AI-FUNC-SUMMARY: Fills the text boxes from the stored settings; returns none; side effects: updates the text boxes.
  void initState() {
    super.initState();
    final settings = widget.controller.settings;
    _limit.text = settings.limit <= 0 ? '' : '${settings.limit}';
    _workingFolder.text = settings.tmpDir;
    _ffmpeg.text = settings.ffmpeg;
    _ffprobe.text = settings.ffprobe;
  }

  @override
  // AI-FUNC-SUMMARY: Releases the text boxes; returns none; side effects: frees the controllers.
  void dispose() {
    _limit.dispose();
    _workingFolder.dispose();
    _ffmpeg.dispose();
    _ffprobe.dispose();
    super.dispose();
  }

  @override
  // AI-FUNC-SUMMARY: Builds the settings screen; returns the widget; side effects: none.
  Widget build(BuildContext context) {
    final text = AppText.of(context);
    final controller = widget.controller;
    final settings = controller.settings;
    final running = controller.isRunning;

    void update(AppSettings next) => controller.updateSettings(next);

    return PageBody(
      children: [
        SectionCard(
          title: text.settingsOriginals,
          leading: const Icon(Icons.inventory_2_outlined),
          children: [
            SwitchListTile(
              contentPadding: EdgeInsets.zero,
              value: settings.keepOriginal,
              onChanged: running
                  ? null
                  : (value) => update(settings.copyWith(keepOriginal: value)),
              title: Text(
                settings.keepOriginal
                    ? text.keepOriginalsOn
                    : text.keepOriginalsOff,
              ),
            ),
            ChoiceField<String>(
              label: text.container,
              value: settings.container,
              values: containerChoices,
              labelFor: text.containerLabel,
              helpFor: text.containerHelp,
              enabled: !running,
              onChanged: (value) => update(settings.copyWith(container: value)),
            ),
          ],
        ),
        SectionCard(
          title: text.settingsSpeed,
          leading: const Icon(Icons.speed_outlined),
          children: [
            ChoiceField<String>(
              label: text.whichEncoder,
              value: settings.encoderPreference,
              values: encoderPreferenceChoices,
              labelFor: text.encoderPreferenceLabel,
              helpFor: text.encoderPreferenceHelp,
              enabled: !running,
              onChanged: (value) =>
                  update(settings.copyWith(encoderPreference: value)),
            ),
            ChoiceField<String>(
              label: text.qualityStrategy,
              value: settings.qualityMode,
              values: qualityModeChoices,
              labelFor: text.qualityModeLabel,
              helpFor: text.qualityModeHelp,
              enabled: !running,
              onChanged: (value) =>
                  update(settings.copyWith(qualityMode: value)),
            ),
            ChoiceField<String>(
              label: text.qualityCheckLabel,
              value: settings.qualityCheck,
              values: qualityCheckChoices,
              labelFor: text.qualityCheckLabelFor,
              helpFor: text.qualityCheckHelp,
              enabled: !running,
              onChanged: (value) =>
                  update(settings.copyWith(qualityCheck: value)),
            ),
          ],
        ),
        SectionCard(
          title: text.appearance,
          leading: const Icon(Icons.palette_outlined),
          children: [
            ChoiceField<String>(
              label: text.appearance,
              value: settings.themeMode,
              values: const ['system', 'light', 'dark'],
              labelFor: text.themeLabel,
              onChanged: (value) => update(settings.copyWith(themeMode: value)),
            ),
            ChoiceField<String>(
              label: text.language,
              value: settings.language,
              values: languageCodes,
              labelFor: text.languageLabel,
              onChanged: (value) {
                update(settings.copyWith(language: value));
                widget.onLanguageChanged(value);
              },
            ),
          ],
        ),
        Card(
          margin: const EdgeInsets.only(bottom: 16),
          child: ExpansionTile(
            leading: const Icon(Icons.tune),
            title: Text(text.advanced),
            shape: const Border(),
            childrenPadding: const EdgeInsets.fromLTRB(20, 0, 20, 8),
            children: [
              LabelledField(
                label: text.limitLabel,
                controller: _limit,
                help: text.limitHelp,
                enabled: !running,
                keyboardType: TextInputType.number,
                onChanged: (value) => update(
                  settings.copyWith(limit: int.tryParse(value.trim()) ?? 0),
                ),
              ),
              _ReviewMargin(
                settings: settings,
                enabled: !running,
                onChanged: (value) =>
                    update(settings.copyWith(reviewMargin: value)),
              ),
              LabelledField(
                label: text.specificEncoder,
                controller: TextEditingController(text: settings.encoder),
                help: text.specificEncoderHelp,
                enabled: false,
              ),
              LabelledField(
                label: text.workingFolder,
                controller: _workingFolder,
                help: text.workingFolderHelp,
                enabled: !running,
                onChanged: (value) => update(settings.copyWith(tmpDir: value)),
              ),
              LabelledField(
                label: 'FFmpeg',
                controller: _ffmpeg,
                help: text.mediaToolsHelp,
                enabled: !running,
                onChanged: (value) => update(settings.copyWith(ffmpeg: value)),
              ),
              LabelledField(
                label: 'FFprobe',
                controller: _ffprobe,
                enabled: !running,
                onChanged: (value) => update(settings.copyWith(ffprobe: value)),
              ),
            ],
          ),
        ),
      ],
    );
  }
}

class _ReviewMargin extends StatelessWidget {
  const _ReviewMargin({
    required this.settings,
    required this.enabled,
    required this.onChanged,
  });

  final AppSettings settings;
  final bool enabled;
  final ValueChanged<int> onChanged;

  @override
  // AI-FUNC-SUMMARY: Builds the slider that sets how far below target still avoids a decision; returns the widget; side effects: none.
  Widget build(BuildContext context) {
    final text = AppText.of(context);
    final theme = Theme.of(context);
    final threshold = settings.qualityTarget - settings.reviewMargin;

    return Padding(
      padding: const EdgeInsets.only(bottom: 18),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(text.reviewMarginLabel, style: theme.textTheme.labelLarge),
          const SizedBox(height: 4),
          Text(formatQuality(threshold), style: theme.textTheme.headlineSmall),
          Slider(
            value: settings.reviewMargin.toDouble(),
            min: 0,
            max: 1000,
            divisions: 20,
            label: formatQuality(threshold),
            onChanged: enabled ? (value) => onChanged(value.round()) : null,
          ),
          Text(
            text.reviewMarginHelp,
            style: theme.textTheme.bodySmall?.copyWith(
              color: theme.colorScheme.onSurfaceVariant,
            ),
          ),
        ],
      ),
    );
  }
}
