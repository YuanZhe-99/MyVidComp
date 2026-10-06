import 'package:flutter/material.dart';
import 'package:myapps_ui/myapps_ui.dart';
import 'package:flutter_localizations/flutter_localizations.dart';

import 'app_controller.dart';
import 'app_localizations.dart';
import 'app_theme.dart';
import 'pages/convert_page.dart';
import 'pages/progress_page.dart';
import 'pages/review_page.dart';
import 'pages/settings_page.dart';

// AI-FUNC-SUMMARY: Starts the application; returns none; side effects: builds the widget tree.
void main() {
  runApp(const MyVidCompApp());
}

/// The application, its theme and its language.
class MyVidCompApp extends StatefulWidget {
  const MyVidCompApp({super.key, this.controller});

  /// Supplied by tests so they can run without touching the real filesystem.
  final AppController? controller;

  @override
  State<MyVidCompApp> createState() => _MyVidCompAppState();
}

class _MyVidCompAppState extends State<MyVidCompApp> {
  late final AppController _controller = widget.controller ?? AppController();
  late final bool _ownsController = widget.controller == null;

  @override
  // AI-FUNC-SUMMARY: Loads settings and finds the video tools when the app starts; returns none; side effects: reads settings and the filesystem.
  void initState() {
    super.initState();
    _controller.addListener(_onControllerChanged);
    _controller.initialise();
  }

  // AI-FUNC-SUMMARY: Rebuilds the app when the shared state changes; returns none; side effects: schedules a rebuild.
  void _onControllerChanged() {
    if (mounted) {
      setState(() {});
    }
  }

  @override
  // AI-FUNC-SUMMARY: Releases the shared state when the app closes; returns none; side effects: stops any running conversion.
  void dispose() {
    _controller.removeListener(_onControllerChanged);
    if (_ownsController) {
      _controller.dispose();
    }
    super.dispose();
  }

  @override
  // AI-FUNC-SUMMARY: Builds the application with its theme, language and home screen; returns the widget; side effects: none.
  Widget build(BuildContext context) {
    final settings = _controller.settings;
    final style = uiStyleFromName(settings.uiStyle);

    return MaterialApp(
      onGenerateTitle: (context) => AppText.of(context).appName,
      debugShowCheckedModeBanner: false,
      locale: localeForLanguageCode(settings.language),
      supportedLocales: AppText.supportedLocales,
      localizationsDelegates: const [
        AppText.delegate,
        GlobalMaterialLocalizations.delegate,
        GlobalCupertinoLocalizations.delegate,
        GlobalWidgetsLocalizations.delegate,
      ],
      themeMode: switch (settings.themeMode) {
        'light' => ThemeMode.light,
        'dark' => ThemeMode.dark,
        _ => ThemeMode.system,
      },
      theme: AppTheme.light(style),
      darkTheme: AppTheme.dark(style),
      home: HomeShell(controller: _controller),
    );
  }
}

/// The four screens, arranged to suit the size of the window.
class HomeShell extends StatefulWidget {
  const HomeShell({super.key, required this.controller});

  final AppController controller;

  /// Below this width the screen is a phone: navigation goes along the bottom.
  static const double compactWidth = 600;

  /// At this width and above there is room for a permanently open rail.
  static const double expandedWidth = 1100;

  @override
  State<HomeShell> createState() => _HomeShellState();
}

class _HomeShellState extends State<HomeShell> {
  int _index = 0;

  @override
  // AI-FUNC-SUMMARY: Purpose: Render style-aware shared navigation around stable pages; Inputs: context; Returns: Widget; Side effects: None; Notes: compact rail and floating Expressive bottom bar use shared defaults.
  Widget build(BuildContext context) {
    final text = AppText.of(context);
    final controller = widget.controller;

    final destinations = [
      (icon: Icons.compress, label: text.navConvert),
      (icon: Icons.timelapse_outlined, label: text.navActivity),
      (icon: Icons.rule, label: text.navReview),
      (icon: Icons.settings_outlined, label: text.navSettings),
    ];

    final pages = [
      ConvertPage(
        controller: controller,
        onOpenSettings: () => setState(() => _index = 3),
      ),
      ProgressPage(controller: controller),
      ReviewPage(controller: controller),
      SettingsPage(
        controller: controller,
        onLanguageChanged: (_) => setState(() {}),
      ),
    ];

    return LayoutBuilder(
      builder: (context, constraints) {
        final width = constraints.maxWidth;
        final compact = width < HomeShell.compactWidth;

        final body = SafeArea(
          child: IndexedStack(index: _index, children: pages),
        );

        return MyAppsNavigationShell(
          appBar: _appBar(context, text, controller),
          destinations: [
            for (final destination in destinations)
              MyAppsDestination(
                icon: _maybeBadge(
                  destination.label == text.navReview,
                  Icon(destination.icon),
                  controller.reviewCount,
                ),
                label: destination.label,
              ),
          ],
          selectedIndex: _index,
          onSelected: _select,
          placement: compact ? NavPlacement.bottom : NavPlacement.side,
          style: uiStyleFromName(controller.settings.uiStyle),
          child: body,
        );
      },
    );
  }

  // AI-FUNC-SUMMARY: Builds the title bar, showing progress while a run is going; returns the app bar; side effects: none.
  PreferredSizeWidget _appBar(
    BuildContext context,
    AppText text,
    AppController controller,
  ) => AppBar(
    title: Text(text.appName),
    centerTitle: false,
    bottom: controller.isRunning
        ? const PreferredSize(
            preferredSize: Size.fromHeight(3),
            child: LinearProgressIndicator(minHeight: 3),
          )
        : null,
  );

  // AI-FUNC-SUMMARY: Adds a count badge to the review icon when something is waiting; returns the icon, badged or plain; side effects: none.
  Widget _maybeBadge(bool isReview, Widget icon, int count) {
    if (!isReview || count == 0) {
      return icon;
    }
    return Badge(label: Text('$count'), child: icon);
  }

  // AI-FUNC-SUMMARY: Switches to another screen, refreshing the review list on the way in; returns none; side effects: may read the chosen folder.
  void _select(int index) {
    setState(() => _index = index);
    if (index == 2) {
      widget.controller.refreshReviews();
    }
  }
}
