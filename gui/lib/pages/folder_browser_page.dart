import 'dart:io';

import 'package:flutter/material.dart';

import '../app_localizations.dart';
import '../folder_picker.dart';

/// An in-app folder chooser for platforms with no dialog that hands back a real
/// path. Android is the reason it exists.
class FolderBrowserPage extends StatefulWidget {
  const FolderBrowserPage({super.key, required this.startIn});

  /// Where to open. Ignored when it is not a folder that can be read.
  final String startIn;

  @override
  State<FolderBrowserPage> createState() => _FolderBrowserPageState();
}

class _FolderBrowserPageState extends State<FolderBrowserPage> {
  static const FolderBrowser _browser = FolderBrowser();
  static const FolderPicker _picker = FolderPicker();

  String? _path;
  List<Directory> _children = const [];
  List<String> _roots = const [];
  bool _loading = true;

  @override
  // AI-FUNC-SUMMARY: Opens the browser at the most useful folder available; returns none; side effects: reads storage volumes and one folder.
  void initState() {
    super.initState();
    _open();
  }

  // AI-FUNC-SUMMARY: Loads the storage volumes and the starting folder; returns when both are read; side effects: reads the filesystem.
  Future<void> _open() async {
    final roots = await _picker.androidStorageRoots();
    final start = widget.startIn.trim().isNotEmpty
        ? widget.startIn.trim()
        : (roots.isNotEmpty ? roots.first : Directory.current.path);

    if (!mounted) {
      return;
    }
    setState(() => _roots = roots);
    await _show(start);
  }

  // AI-FUNC-SUMMARY: Lists one folder's contents; returns when the list is loaded; side effects: reads the folder.
  Future<void> _show(String path) async {
    setState(() {
      _loading = true;
      _path = path;
    });
    final children = await _browser.childFolders(path);
    if (mounted) {
      setState(() {
        _children = children;
        _loading = false;
      });
    }
  }

  @override
  // AI-FUNC-SUMMARY: Builds the folder browser; returns the widget; side effects: none.
  Widget build(BuildContext context) {
    final text = AppText.of(context);
    final path = _path;
    final parent = path == null ? null : FolderBrowser.parentOf(path);

    return Scaffold(
      appBar: AppBar(
        title: Text(text.chooseFolder),
        actions: [
          if (path != null)
            TextButton(
              onPressed: () => Navigator.of(context).pop(path),
              child: Text(MaterialLocalizations.of(context).okButtonLabel),
            ),
        ],
      ),
      body: Column(
        children: [
          if (path != null)
            Padding(
              padding: const EdgeInsets.fromLTRB(16, 12, 16, 4),
              child: Align(
                alignment: Alignment.centerLeft,
                child: Text(path, style: Theme.of(context).textTheme.bodySmall),
              ),
            ),
          if (_roots.length > 1)
            SizedBox(
              height: 52,
              child: ListView(
                scrollDirection: Axis.horizontal,
                padding: const EdgeInsets.symmetric(horizontal: 12),
                children: [
                  for (final root in _roots)
                    Padding(
                      padding: const EdgeInsets.symmetric(horizontal: 4),
                      child: ActionChip(
                        label: Text(FolderBrowser.displayName(root)),
                        onPressed: () => _show(root),
                      ),
                    ),
                ],
              ),
            ),
          const Divider(height: 1),
          Expanded(
            child: _loading
                ? const Center(child: CircularProgressIndicator())
                : ListView(
                    children: [
                      if (parent != null)
                        ListTile(
                          leading: const Icon(Icons.arrow_upward),
                          title: const Text('..'),
                          onTap: () => _show(parent),
                        ),
                      for (final child in _children)
                        ListTile(
                          leading: const Icon(Icons.folder_outlined),
                          title: Text(FolderBrowser.displayName(child.path)),
                          onTap: () => _show(child.path),
                        ),
                    ],
                  ),
          ),
        ],
      ),
      floatingActionButton: path == null
          ? null
          : FloatingActionButton.extended(
              onPressed: () => Navigator.of(context).pop(path),
              icon: const Icon(Icons.check),
              label: Text(text.chooseFolder),
            ),
    );
  }
}
