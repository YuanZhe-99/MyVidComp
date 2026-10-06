import 'package:flutter/material.dart';
import 'package:myapps_ui/myapps_ui.dart';

import '../app_controller.dart';
import '../app_localizations.dart';
import '../formatting.dart';
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
  final GlobalKey _advancedKey = GlobalKey();
  bool _advancedExpanded = false;

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
  // AI-FUNC-SUMMARY: Purpose: Render shared settings sections and responsive panes; Inputs: context; Returns: Widget; Side effects: selections persist settings; Notes: advanced controls keep a stable key across split changes.
  Widget build(BuildContext context) {
    final text = AppText.of(context);
    final controller = widget.controller;
    final settings = controller.settings;
    final running = controller.isRunning;

    void update(AppSettings next) => controller.updateSettings(next);

    final sections = <Widget>[
      MyAppsSettingsSection(
        title: text.settingsOriginals,
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
      MyAppsSettingsSection(
        title: text.settingsSpeed,
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
            label: text.whichDecoder,
            value: settings.decoderPreference,
            values: decoderPreferenceChoices,
            labelFor: text.decoderPreferenceLabel,
            helpFor: text.decoderPreferenceHelp,
            enabled: !running,
            onChanged: (value) =>
                update(settings.copyWith(decoderPreference: value)),
          ),
          ChoiceField<String>(
            label: text.qualityStrategy,
            value: settings.qualityMode,
            values: qualityModeChoices,
            labelFor: text.qualityModeLabel,
            helpFor: text.qualityModeHelp,
            enabled: !running,
            onChanged: (value) => update(settings.copyWith(qualityMode: value)),
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
      MyAppsSettingsSection(
        title: text.appearance,
        children: [
          MyAppsSettingsSegmentRow<String>(
            leading: const Icon(Icons.brightness_6_outlined),
            title: text.appearance,
            selected: {settings.themeMode},
            segments: [
              for (final value in const ['system', 'light', 'dark'])
                ButtonSegment(
                  value: value,
                  label: Text(text.themeLabel(value)),
                ),
            ],
            onSelectionChanged: (values) =>
                update(settings.copyWith(themeMode: values.first)),
          ),
          MyAppsSettingsSegmentRow<String>(
            leading: const Icon(Icons.palette_outlined),
            title: text.uiStyleTitle,
            description: text.uiStyleHelp(settings.uiStyle),
            selected: {settings.uiStyle},
            segments: [
              for (final value in const ['material3', 'expressive'])
                ButtonSegment(
                  value: value,
                  label: Text(text.uiStyleLabel(value)),
                ),
            ],
            onSelectionChanged: (values) =>
                update(settings.copyWith(uiStyle: values.first)),
          ),
          MyAppsSettingsSegmentRow<String>(
            leading: const Icon(Icons.language),
            title: text.language,
            selected: {settings.language},
            segments: [
              for (final value in languageCodes)
                ButtonSegment(
                  value: value,
                  label: Text(text.languageLabel(value)),
                ),
            ],
            onSelectionChanged: (values) {
              final value = values.first;
              update(settings.copyWith(language: value));
              widget.onLanguageChanged(value);
            },
          ),
        ],
      ),
      const MyAppsSettingsSection(
        title: 'MyApps-UI',
        children: [
          SelectableText(
            'myapps_ui — GNU GPL version 3\n'
            'Copyright (C) 2026 yuanzhe and contributors\n'
            'https://github.com/YuanZhe-99/MyApps-UI\n'
            'https://www.gnu.org/licenses/gpl-3.0.html',
          ),
        ],
      ),
      Card(
        key: _advancedKey,
        margin: const EdgeInsets.only(bottom: 16),
        child: ExpansionTile(
          initiallyExpanded: _advancedExpanded,
          onExpansionChanged: (expanded) => _advancedExpanded = expanded,
          leading: const Icon(Icons.tune),
          title: Text(text.advanced),
          shape: const Border(),
          childrenPadding: const EdgeInsets.fromLTRB(16, 0, 16, 8),
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
            ListTile(
              title: Text(text.specificEncoder),
              subtitle: Text(
                '${text.encoderLabel(settings.encoder)}\n${text.specificEncoderHelp}',
              ),
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
    ];
    return LayoutBuilder(
      // AI-FUNC-SUMMARY: Purpose: Partition settings using actual available width; Inputs: context and constraints; Returns: shared pane body; Side effects: None; Notes: narrow primary includes advanced controls.
      builder: (context, constraints) {
        final split = constraints.maxWidth >= 900;
        var effectiveSplit = split;
        return MyAppsPaneBody(
          primary: Builder(
            builder: (context) => PageBody(
              children: effectiveSplit
                  ? sections.sublist(0, sections.length - 1)
                  : sections,
            ),
          ),
          secondary: Builder(
            builder: (context) => effectiveSplit
                ? PageBody(children: [sections.last])
                : const SizedBox.shrink(),
          ),
          allowSplit: split,
          primaryWidthFor: (width) => width / 2,
          primaryMinWidth: 400,
          secondaryMinWidth: 400,
          topInset: MediaQuery.paddingOf(context).top,
          onSplitChanged: (value) => effectiveSplit = value,
        );
      },
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
          Text(formatScore(threshold), style: theme.textTheme.headlineSmall),
          Slider(
            value: settings.reviewMargin.toDouble(),
            min: 0,
            max: 1000,
            divisions: 20,
            label: formatScore(threshold),
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
