import 'dart:io';

import 'package:flutter/material.dart';

import '../app_controller.dart';
import '../app_localizations.dart';
import '../app_settings.dart';
import '../folder_picker.dart';
import '../formatting.dart';
import '../widgets.dart';
import 'folder_browser_page.dart';

/// The first screen: pick a folder, pick how the videos should come out, start.
class ConvertPage extends StatefulWidget {
  const ConvertPage({
    super.key,
    required this.controller,
    required this.onOpenSettings,
  });

  final AppController controller;
  final VoidCallback onOpenSettings;

  @override
  State<ConvertPage> createState() => _ConvertPageState();
}

class _ConvertPageState extends State<ConvertPage> {
  final TextEditingController _folder = TextEditingController();
  final TextEditingController _outputDir = TextEditingController();
  final FolderPicker _picker = const FolderPicker();
  bool _moreExpanded = false;

  @override
  // AI-FUNC-SUMMARY: Fills the folder boxes from stored settings; returns none; side effects: updates text controllers.
  void initState() {
    super.initState();
    _folder.text = widget.controller.settings.targetFolder;
    _outputDir.text = widget.controller.settings.outputDir;
  }

  @override
  // AI-FUNC-SUMMARY: Releases the folder text controllers; returns none; side effects: frees controllers.
  void dispose() {
    _folder.dispose();
    _outputDir.dispose();
    super.dispose();
  }

  @override
  // AI-FUNC-SUMMARY: Keeps folder boxes in step when settings change elsewhere; returns none; side effects: updates text controllers.
  void didUpdateWidget(covariant ConvertPage oldWidget) {
    super.didUpdateWidget(oldWidget);
    final stored = widget.controller.settings.targetFolder;
    if (_folder.text != stored && !_folder.selection.isValid) {
      _folder.text = stored;
    }
    final storedOut = widget.controller.settings.outputDir;
    if (_outputDir.text != storedOut && !_outputDir.selection.isValid) {
      _outputDir.text = storedOut;
    }
  }

  // AI-FUNC-SUMMARY: Opens a folder chooser for either source or separate output folder; returns when closed; side effects: may request file access and update settings.
  Future<void> _chooseFolder({required bool forOutputDir}) async {
    final current = forOutputDir ? _outputDir.text.trim() : _folder.text.trim();
    String? chosen;

    if (_picker.hasNativeDialog) {
      chosen = await _picker.pickFolder(startIn: current);
    } else {
      if (Platform.isAndroid && !await _picker.ensureAndroidFileAccess()) {
        return;
      }
      if (!mounted) {
        return;
      }
      chosen = await Navigator.of(context).push<String>(
        MaterialPageRoute(
          builder: (context) => FolderBrowserPage(startIn: current),
        ),
      );
    }

    if (chosen == null || !mounted) {
      return;
    }
    if (forOutputDir) {
      _outputDir.text = chosen;
      widget.controller.updateSettings(
        widget.controller.settings.copyWith(outputDir: chosen),
      );
    } else {
      _folder.text = chosen;
      widget.controller.updateSettings(
        widget.controller.settings.copyWith(targetFolder: chosen),
      );
    }
  }

  @override
  // AI-FUNC-SUMMARY: Builds the clean Convert screen with inline core options and collapsible run options; returns the widget; side effects: none.
  Widget build(BuildContext context) {
    final text = AppText.of(context);
    final theme = Theme.of(context);
    final controller = widget.controller;
    final settings = controller.settings;
    final running = controller.isRunning;

    void update(AppSettings next) => controller.updateSettings(next);

    final encoderOptions = <String>[
      'auto',
      ...controller.availableEncoders,
      if (settings.encoder != 'auto' &&
          !controller.availableEncoders.contains(settings.encoder))
        settings.encoder,
    ];

    final qualityValues = qualityPresets.contains(settings.qualityTarget)
        ? qualityPresets
        : [...qualityPresets, settings.qualityTarget];

    return PageBody(
      children: [
        (!controller.tools.areReady && !controller.tools.isChecking)
            ? _MissingToolsBanner(onFix: widget.onOpenSettings)
            : const SizedBox.shrink(),
        SectionCard(
          title: text.folderTitle,
          subtitle: text.folderHint,
          leading: const Icon(Icons.folder_outlined),
          children: [
            LabelledField(
              label: text.folderTitle,
              controller: _folder,
              hint: switch (true) {
                _ when Platform.isWindows => r'D:\Videos',
                _ when Platform.isAndroid => '/storage/emulated/0/DCIM',
                _ => '/home/me/Videos',
              },
              enabled: !running,
              onChanged: (value) =>
                  update(settings.copyWith(targetFolder: value)),
              trailing: FilledButton.tonalIcon(
                onPressed: running
                    ? null
                    : () => _chooseFolder(forOutputDir: false),
                icon: const Icon(Icons.folder_open),
                label: Text(
                  _picker.hasNativeDialog ? text.chooseFolder : text.browse,
                ),
              ),
            ),
            SwitchListTile(
              contentPadding: EdgeInsets.zero,
              value: settings.recursive,
              onChanged: running
                  ? null
                  : (value) => update(settings.copyWith(recursive: value)),
              title: Text(text.recursiveLabel),
              subtitle: Text(text.recursiveHelp),
            ),
          ],
        ),
        SectionCard(
          title: text.outputCodec,
          leading: const Icon(Icons.movie_outlined),
          children: [
            ChoiceField<String>(
              label: text.outputCodec,
              value: settings.codec,
              values: codecChoices,
              labelFor: text.codecLabel,
              helpFor: text.codecHelp,
              enabled: !running,
              onChanged: (value) =>
                  update(settings.copyWith(codec: value, encoder: 'auto')),
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
            ChoiceField<int>(
              label: text.quality,
              value: settings.qualityTarget,
              values: qualityValues,
              labelFor: text.qualityPresetLabel,
              helpFor: text.qualityPresetHelp,
              enabled: !running && settings.qualityMode != 'fixed',
              onChanged: (value) =>
                  update(settings.copyWith(qualityTarget: value)),
            ),
            Padding(
              padding: const EdgeInsets.only(bottom: 14),
              child: Row(
                children: [
                  Expanded(
                    child: Text(
                      '${text.customQualityLabel}: ${formatScore(settings.qualityTarget)}',
                      style: theme.textTheme.bodySmall?.copyWith(
                        color: theme.colorScheme.onSurfaceVariant,
                      ),
                    ),
                  ),
                  SizedBox(
                    width: 180,
                    child: Slider(
                      value: settings.qualityTarget.toDouble().clamp(
                        5000,
                        10000,
                      ),
                      min: 5000,
                      max: 10000,
                      divisions: 50,
                      label: formatScore(settings.qualityTarget),
                      onChanged: running || settings.qualityMode == 'fixed'
                          ? null
                          : (value) => update(
                              settings.copyWith(qualityTarget: value.round()),
                            ),
                    ),
                  ),
                ],
              ),
            ),
            ChoiceField<String>(
              label: text.howCareful,
              value: settings.preservation,
              values: preservationChoices,
              labelFor: text.preservationLabel,
              helpFor: text.preservationHelp,
              enabled: !running,
              onChanged: (value) =>
                  update(settings.copyWith(preservation: value)),
            ),
          ],
        ),
        Card(
          margin: const EdgeInsets.only(bottom: 16),
          child: ExpansionTile(
            initiallyExpanded: _moreExpanded,
            onExpansionChanged: (expanded) =>
                setState(() => _moreExpanded = expanded),
            leading: const Icon(Icons.tune),
            title: Text(text.moreRunOptions),
            subtitle: Text(
              '${text.encoderPreferenceLabel(settings.encoderPreference)} · '
              '${text.encoderPresetLabel(settings.encoderPreset)} · '
              '${text.qualityCheckLabelFor(settings.qualityCheck)} · '
              '${text.qualityMetricName(settings.qualityMetric)}',
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: theme.textTheme.bodySmall?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
              ),
            ),
            shape: const Border(),
            childrenPadding: const EdgeInsets.fromLTRB(16, 4, 16, 16),
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
                label: text.specificEncoder,
                value: settings.encoder,
                values: encoderOptions,
                labelFor: text.encoderLabel,
                enabled: !running,
                onChanged: (value) => update(settings.copyWith(encoder: value)),
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
                label: text.encoderPresetTitle,
                value: settings.encoderPreset,
                values: encoderPresetChoices,
                labelFor: text.encoderPresetLabel,
                helpFor: text.encoderPresetHelp,
                enabled: !running,
                onChanged: (value) =>
                    update(settings.copyWith(encoderPreset: value)),
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
              if (settings.qualityMode == 'fixed')
                Padding(
                  padding: const EdgeInsets.only(bottom: 14),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        '${text.fixedQualityLabel}: ${formatScore(settings.fixedQuality)}',
                        style: theme.textTheme.labelLarge,
                      ),
                      Slider(
                        value: settings.fixedQuality.toDouble().clamp(
                          1000,
                          5500,
                        ),
                        min: 1000,
                        max: 5500,
                        divisions: 90,
                        label: formatScore(settings.fixedQuality),
                        onChanged: running
                            ? null
                            : (value) => update(
                                settings.copyWith(fixedQuality: value.round()),
                              ),
                      ),
                      Text(
                        text.fixedQualityHelp,
                        style: theme.textTheme.bodySmall?.copyWith(
                          color: theme.colorScheme.onSurfaceVariant,
                        ),
                      ),
                    ],
                  ),
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
              ChoiceField<String>(
                label: text.qualityMetricLabel,
                value: settings.qualityMetric,
                values: qualityMetricChoices,
                labelFor: text.qualityMetricName,
                helpFor: text.qualityMetricHelp,
                enabled: !running,
                onChanged: (value) =>
                    update(settings.copyWith(qualityMetric: value)),
              ),
              ChoiceField<String>(
                label: text.sizeGuardTitle,
                value: settings.sizeGuard,
                values: sizeGuardChoices,
                labelFor: text.sizeGuardLabel,
                helpFor: text.sizeGuardHelp,
                enabled: !running,
                onChanged: (value) =>
                    update(settings.copyWith(sizeGuard: value)),
              ),
              ChoiceField<String>(
                label: text.audioPolicyTitle,
                value: settings.audioPolicy,
                values: audioPolicyChoices,
                labelFor: text.audioPolicyLabel,
                helpFor: text.audioPolicyHelp,
                enabled: !running,
                onChanged: (value) =>
                    update(settings.copyWith(audioPolicy: value)),
              ),
              ChoiceField<String>(
                label: text.deinterlaceTitle,
                value: settings.deinterlace,
                values: deinterlaceChoices,
                labelFor: text.deinterlaceLabel,
                helpFor: text.deinterlaceHelp,
                enabled: !running,
                onChanged: (value) =>
                    update(settings.copyWith(deinterlace: value)),
              ),
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
              LabelledField(
                label: text.outputDirLabel,
                controller: _outputDir,
                help: text.outputDirHelp,
                enabled: !running,
                onChanged: (value) =>
                    update(settings.copyWith(outputDir: value)),
                trailing: FilledButton.tonalIcon(
                  onPressed: running
                      ? null
                      : () => _chooseFolder(forOutputDir: true),
                  icon: const Icon(Icons.folder_open),
                  label: Text(
                    _picker.hasNativeDialog ? text.chooseFolder : text.browse,
                  ),
                ),
              ),
            ],
          ),
        ),
        Card(
          margin: const EdgeInsets.only(bottom: 16),
          child: Padding(
            padding: const EdgeInsets.all(16),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                SwitchListTile(
                  contentPadding: EdgeInsets.zero,
                  value: settings.previewOnly,
                  onChanged: running
                      ? null
                      : (value) =>
                            update(settings.copyWith(previewOnly: value)),
                  title: Text(text.previewOnly),
                  subtitle: Text(text.previewOnlyHelp),
                ),
                const SizedBox(height: 12),
                _StartStop(controller: controller),
              ],
            ),
          ),
        ),
      ],
    );
  }
}

class _MissingToolsBanner extends StatelessWidget {
  const _MissingToolsBanner({required this.onFix});

  final VoidCallback onFix;

  @override
  // AI-FUNC-SUMMARY: Renders an error card when FFmpeg or FFprobe is missing; returns the widget; side effects: none.
  Widget build(BuildContext context) {
    final text = AppText.of(context);
    final theme = Theme.of(context);

    return Card(
      margin: const EdgeInsets.only(bottom: 16),
      color: theme.colorScheme.errorContainer,
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Icon(
              Icons.error_outline,
              color: theme.colorScheme.onErrorContainer,
            ),
            const SizedBox(width: 12),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    text.readyToolsMissing,
                    style: theme.textTheme.titleSmall?.copyWith(
                      color: theme.colorScheme.onErrorContainer,
                    ),
                  ),
                  const SizedBox(height: 4),
                  Text(
                    text.toolsMissingHelp,
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: theme.colorScheme.onErrorContainer,
                    ),
                  ),
                  const SizedBox(height: 8),
                  TextButton(onPressed: onFix, child: Text(text.navSettings)),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _StartStop extends StatelessWidget {
  const _StartStop({required this.controller});

  final AppController controller;

  @override
  // AI-FUNC-SUMMARY: Builds the start and stop buttons; returns the widget; side effects: none.
  Widget build(BuildContext context) {
    final text = AppText.of(context);

    if (controller.isRunning) {
      return OutlinedButton.icon(
        onPressed: controller.isStopping ? null : controller.stop,
        icon: const Icon(Icons.stop_rounded),
        label: Text(controller.isStopping ? text.stopping : text.stop),
        style: OutlinedButton.styleFrom(minimumSize: const Size.fromHeight(52)),
      );
    }

    return FilledButton.icon(
      onPressed: controller.canStart ? controller.start : null,
      icon: const Icon(Icons.play_arrow_rounded),
      label: Text(text.start),
      style: FilledButton.styleFrom(minimumSize: const Size.fromHeight(52)),
    );
  }
}
