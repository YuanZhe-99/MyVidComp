import 'package:flutter/material.dart';
import 'package:flutter_localizations/flutter_localizations.dart';

import 'app_controller.dart';
import 'app_localizations.dart';
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
    // One seed colour produces a matching light and dark palette, so the
    // interface follows the device instead of forcing one appearance.
    const seed = Color(0xFF1D6FD0);
    final settings = _controller.settings;

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
      theme: _theme(seed, Brightness.light),
      darkTheme: _theme(seed, Brightness.dark),
      home: HomeShell(controller: _controller),
    );
  }

  // AI-FUNC-SUMMARY: Builds the theme for one brightness; returns the theme; side effects: none.
  ThemeData _theme(Color seed, Brightness brightness) {
    final scheme = ColorScheme.fromSeed(
      seedColor: seed,
      brightness: brightness,
    );
    return ThemeData(
      colorScheme: scheme,
      useMaterial3: true,
      cardTheme: CardThemeData(
        elevation: 0,
        color: scheme.surfaceContainerLow,
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(20)),
      ),
      inputDecorationTheme: const InputDecorationTheme(
        border: OutlineInputBorder(),
        isDense: true,
      ),
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
  // AI-FUNC-SUMMARY: Builds the navigation shell around the selected screen; returns the widget; side effects: none.
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
        final expanded = width >= HomeShell.expandedWidth;

        final body = SafeArea(
          child: IndexedStack(index: _index, children: pages),
        );

        if (compact) {
          return Scaffold(
            appBar: _appBar(context, text, controller),
            body: body,
            bottomNavigationBar: NavigationBar(
              selectedIndex: _index,
              onDestinationSelected: _select,
              destinations: [
                for (final destination in destinations)
                  NavigationDestination(
                    icon: _maybeBadge(
                      destination.label == text.navReview,
                      Icon(destination.icon),
                      controller.reviewCount,
                    ),
                    label: destination.label,
                  ),
              ],
            ),
          );
        }

        return Scaffold(
          appBar: _appBar(context, text, controller),
          body: Row(
            children: [
              NavigationRail(
                selectedIndex: _index,
                onDestinationSelected: _select,
                extended: expanded,
                minExtendedWidth: 200,
                labelType: expanded
                    ? NavigationRailLabelType.none
                    : NavigationRailLabelType.all,
                destinations: [
                  for (final destination in destinations)
                    NavigationRailDestination(
                      icon: _maybeBadge(
                        destination.label == text.navReview,
                        Icon(destination.icon),
                        controller.reviewCount,
                      ),
                      label: Text(destination.label),
                    ),
                ],
              ),
              const VerticalDivider(width: 1),
              Expanded(child: body),
            ],
          ),
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
