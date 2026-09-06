import 'dart:io';

import 'package:flutter/material.dart';

import '../app_controller.dart';
import '../app_localizations.dart';
import '../app_settings.dart';
import '../folder_picker.dart';
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
  final FolderPicker _picker = const FolderPicker();

  @override
  // AI-FUNC-SUMMARY: Fills the folder box from the stored settings; returns none; side effects: updates the text box.
  void initState() {
    super.initState();
    _folder.text = widget.controller.settings.targetFolder;
  }

  @override
  // AI-FUNC-SUMMARY: Releases the folder text box; returns none; side effects: frees the controller.
  void dispose() {
    _folder.dispose();
    super.dispose();
  }

  @override
  // AI-FUNC-SUMMARY: Keeps the folder box in step when the folder changes elsewhere; returns none; side effects: updates the text box.
  void didUpdateWidget(covariant ConvertPage oldWidget) {
    super.didUpdateWidget(oldWidget);
    final stored = widget.controller.settings.targetFolder;
    if (_folder.text != stored && !_folder.selection.isValid) {
      _folder.text = stored;
    }
  }

  // AI-FUNC-SUMMARY: Opens the folder chooser and stores the result; returns when the chooser closes; side effects: may request file access and update the settings.
  Future<void> _chooseFolder() async {
    String? chosen;

    if (_picker.hasNativeDialog) {
      chosen = await _picker.pickFolder(startIn: _folder.text.trim());
    } else {
      // Android has no dialog that returns a path the engine can use, so the
      // app browses folders itself. It needs file access before it can.
      if (Platform.isAndroid && !await _picker.ensureAndroidFileAccess()) {
        return;
      }
      if (!mounted) {
        return;
      }
      chosen = await Navigator.of(context).push<String>(
        MaterialPageRoute(
          builder: (context) => FolderBrowserPage(startIn: _folder.text.trim()),
        ),
      );
    }

    if (chosen == null || !mounted) {
      return;
    }
    _folder.text = chosen;
    widget.controller.updateSettings(
      widget.controller.settings.copyWith(targetFolder: chosen),
    );
  }

  @override
  // AI-FUNC-SUMMARY: Builds the convert screen; returns the widget; side effects: none.
  Widget build(BuildContext context) {
    final text = AppText.of(context);
    final controller = widget.controller;
    final settings = controller.settings;
    final running = controller.isRunning;

    return PageBody(
      children: [
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
              onChanged: (value) => controller.updateSettings(
                settings.copyWith(targetFolder: value),
              ),
              trailing: FilledButton.tonalIcon(
                onPressed: running ? null : _chooseFolder,
                icon: const Icon(Icons.folder_open),
                label: Text(
                  _picker.hasNativeDialog ? text.chooseFolder : text.browse,
                ),
              ),
            ),
            _ToolStatus(controller: controller, onFix: widget.onOpenSettings),
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
                  controller.updateSettings(settings.copyWith(codec: value)),
            ),
            ChoiceField<int>(
              label: text.quality,
              value: settings.qualityTarget,
              values: qualityPresets,
              labelFor: text.qualityPresetLabel,
              helpFor: text.qualityPresetHelp,
              enabled: !running,
              onChanged: (value) => controller.updateSettings(
                settings.copyWith(qualityTarget: value),
              ),
            ),
            ChoiceField<String>(
              label: text.howCareful,
              value: settings.preservation,
              values: preservationChoices,
              labelFor: text.preservationLabel,
              helpFor: text.preservationHelp,
              enabled: !running,
              onChanged: (value) => controller.updateSettings(
                settings.copyWith(preservation: value),
              ),
            ),
            SwitchListTile(
              contentPadding: EdgeInsets.zero,
              value: settings.previewOnly,
              onChanged: running
                  ? null
                  : (value) => controller.updateSettings(
                      settings.copyWith(previewOnly: value),
                    ),
              title: Text(text.previewOnly),
              subtitle: Text(text.previewOnlyHelp),
            ),
          ],
        ),
        _StartStop(controller: controller),
      ],
    );
  }
}

class _ToolStatus extends StatelessWidget {
  const _ToolStatus({required this.controller, required this.onFix});

  final AppController controller;
  final VoidCallback onFix;

  @override
  // AI-FUNC-SUMMARY: Builds the banner saying whether the video tools were found; returns the widget; side effects: none.
  Widget build(BuildContext context) {
    final text = AppText.of(context);
    final theme = Theme.of(context);
    final ready = controller.tools.areReady;

    final background = ready
        ? theme.colorScheme.primaryContainer
        : theme.colorScheme.errorContainer;
    final foreground = ready
        ? theme.colorScheme.onPrimaryContainer
        : theme.colorScheme.onErrorContainer;

    return Container(
      padding: const EdgeInsets.all(14),
      decoration: BoxDecoration(
        color: background,
        borderRadius: BorderRadius.circular(14),
      ),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Icon(
            ready ? Icons.check_circle_outline : Icons.error_outline,
            color: foreground,
          ),
          const SizedBox(width: 12),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  ready ? text.readyToolsFound : text.readyToolsMissing,
                  style: theme.textTheme.titleSmall?.copyWith(
                    color: foreground,
                  ),
                ),
                if (!ready) ...[
                  const SizedBox(height: 4),
                  Text(
                    text.toolsMissingHelp,
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: foreground,
                    ),
                  ),
                  const SizedBox(height: 8),
                  TextButton(onPressed: onFix, child: Text(text.navSettings)),
                ],
              ],
            ),
          ),
        ],
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
      return Column(
        children: [
          OutlinedButton.icon(
            onPressed: controller.isStopping ? null : controller.stop,
            icon: const Icon(Icons.stop_rounded),
            label: Text(controller.isStopping ? text.stopping : text.stop),
            style: OutlinedButton.styleFrom(
              minimumSize: const Size.fromHeight(52),
            ),
          ),
        ],
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
