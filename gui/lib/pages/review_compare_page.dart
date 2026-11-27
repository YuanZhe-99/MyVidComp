import 'dart:async';
import 'dart:io';
import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../app_controller.dart';
import '../app_localizations.dart';
import '../core_ffi.dart';
import '../formatting.dart';
import '../widgets.dart';

/// View modes shared by both still-frame inspection and live clip playback.
enum _CompareViewMode { wipe, toggle, sideBySide, diff }

/// Dual-mode comparison workspace for inspecting a pending review before deciding.
class ReviewComparePage extends StatefulWidget {
  const ReviewComparePage({
    super.key,
    required this.controller,
    required this.review,
  });

  final AppController controller;
  final PendingReview review;

  @override
  State<ReviewComparePage> createState() => _ReviewComparePageState();
}

class _ReviewComparePageState extends State<ReviewComparePage>
    with SingleTickerProviderStateMixin {
  late final TabController _tabs = TabController(length: 2, vsync: this);
  final TransformationController _zoom = TransformationController();
  final FocusNode _focusNode = FocusNode();

  Directory? _tempDir;
  double _durationSeconds = 0;
  double _timestampSeconds = 0;
  double _wipePosition = 0.5;
  _CompareViewMode _viewMode = _CompareViewMode.wipe;
  bool _holdingOriginal = false;

  PreviewFramePair? _frames;
  bool _loadingFrames = true;
  String? _frameError;

  PreviewClipSequence? _sequence;
  bool _loadingSequence = false;
  String? _sequenceError;
  int _frameIndex = 0;
  bool _playing = false;
  double _playbackSpeed = 1.0;
  Timer? _playbackTimer;
  bool _exportingClip = false;

  @override
  // AI-FUNC-SUMMARY: Initializes comparison state, temp storage, and initial frame extraction; returns none; side effects: creates temp dir and probes/extracts frames.
  void initState() {
    super.initState();
    _durationSeconds = widget.review.durationSeconds ?? 0;
    _timestampSeconds = widget.review.worstFrameSeconds ?? 0;
    _tabs.addListener(_onTabChanged);
    unawaited(_initialisePreview());
  }

  @override
  // AI-FUNC-SUMMARY: Releases timers, controllers, and temporary preview files; returns none; side effects: deletes temporary preview directory.
  void dispose() {
    _playbackTimer?.cancel();
    _tabs.removeListener(_onTabChanged);
    _tabs.dispose();
    _zoom.dispose();
    _focusNode.dispose();
    final dir = _tempDir;
    if (dir != null) {
      unawaited(dir.delete(recursive: true).catchError((_) => dir));
    }
    super.dispose();
  }

  // AI-FUNC-SUMMARY: Prepares temp dir, probes duration if absent, and loads the first frame pair; returns when ready; side effects: runs ffprobe/ffmpeg via PreviewService.
  Future<void> _initialisePreview() async {
    _tempDir = await Directory.systemTemp.createTemp('myvidcomp-preview-');
    if (_durationSeconds <= 0 && widget.controller.tools.areReady) {
      try {
        final info = await widget.controller.previewService.probe(
          widget.controller.tools.ffprobe.path,
          widget.review.originalPath,
        );
        _durationSeconds = info.durationSeconds;
      } catch (_) {
        // Duration stays 0 and scrubbing degrades cleanly.
      }
    }
    if (widget.review.worstFrameSeconds == null && _durationSeconds > 0) {
      _timestampSeconds = _durationSeconds * 0.5;
    }
    if (mounted) {
      setState(() {});
    }
    await _loadFramesAt(_timestampSeconds);
  }

  // AI-FUNC-SUMMARY: Handles tab switches by pausing playback or lazily loading synchronized clip frames; returns none; side effects: may extract clip sequence.
  void _onTabChanged() {
    if (_tabs.index == 0) {
      _pausePlayback();
    } else if (_sequence == null && !_loadingSequence) {
      unawaited(_loadClipSequence());
    }
  }

  // AI-FUNC-SUMMARY: Extracts aligned original, converted, and difference PNG frames at a timestamp; returns when complete; side effects: writes PNGs in temp dir.
  Future<void> _loadFramesAt(double seconds) async {
    final dir = _tempDir;
    if (dir == null || !widget.controller.tools.areReady) {
      if (mounted) {
        setState(() {
          _loadingFrames = false;
          _frameError = AppText.of(context).readyToolsMissing;
        });
      }
      return;
    }

    setState(() {
      _loadingFrames = true;
      _frameError = null;
    });

    final stampDir = Directory(
      '${dir.path}${Platform.pathSeparator}f-${DateTime.now().microsecondsSinceEpoch}',
    );
    try {
      final pair = await widget.controller.previewService.extractFramePair(
        ffmpeg: widget.controller.tools.ffmpeg.path,
        original: widget.review.originalPath,
        review: widget.review.reviewPath,
        timestampMs: (seconds * 1000).round().clamp(0, 1 << 50),
        outDir: stampDir.path,
        includeDiff: true,
      );
      if (!mounted) {
        return;
      }
      PaintingBinding.instance.imageCache.clear();
      setState(() {
        _frames = pair;
        _loadingFrames = false;
      });
    } catch (error) {
      if (!mounted) {
        return;
      }
      setState(() {
        _loadingFrames = false;
        _frameError = error.toString();
      });
    }
  }

  // AI-FUNC-SUMMARY: Extracts a 5-second synchronized JPEG sequence centered near the active timestamp; returns when complete; side effects: writes JPEGs in temp dir and starts playback.
  Future<void> _loadClipSequence() async {
    final dir = _tempDir;
    if (dir == null || !widget.controller.tools.areReady) {
      return;
    }
    _pausePlayback();
    setState(() {
      _loadingSequence = true;
      _sequenceError = null;
    });

    final seqDir = Directory(
      '${dir.path}${Platform.pathSeparator}seq-${DateTime.now().microsecondsSinceEpoch}',
    );
    final maxStart = math.max(0.0, _durationSeconds - 5.0);
    final startSeconds = (_timestampSeconds - 2.5).clamp(0.0, maxStart);

    try {
      final sequence = await widget.controller.previewService
          .extractClipSequence(
            ffmpeg: widget.controller.tools.ffmpeg.path,
            original: widget.review.originalPath,
            review: widget.review.reviewPath,
            startMs: (startSeconds * 1000).round(),
            durationMs: 5000,
            fps: 15,
            maxHeight: 720,
            outDir: seqDir.path,
          );
      if (!mounted) {
        return;
      }
      if (_viewMode == _CompareViewMode.diff) {
        _viewMode = _CompareViewMode.wipe;
      }
      setState(() {
        _sequence = sequence;
        _frameIndex = 0;
        _loadingSequence = false;
      });
      _startPlayback();
    } catch (error) {
      if (!mounted) {
        return;
      }
      setState(() {
        _loadingSequence = false;
        _sequenceError = error.toString();
      });
    }
  }

  // AI-FUNC-SUMMARY: Starts or restarts the synchronized frame-sequence playback timer; returns none; side effects: schedules periodic timer.
  void _startPlayback() {
    final seq = _sequence;
    if (seq == null || seq.originalFrames.isEmpty) {
      return;
    }
    _playbackTimer?.cancel();
    final intervalMs = (1000 / (seq.fps * _playbackSpeed)).round().clamp(
      16,
      500,
    );
    _playing = true;
    _playbackTimer = Timer.periodic(Duration(milliseconds: intervalMs), (_) {
      if (!mounted || _sequence == null) {
        return;
      }
      setState(() {
        _frameIndex = (_frameIndex + 1) % _sequence!.originalFrames.length;
      });
    });
    if (mounted) {
      setState(() {});
    }
  }

  // AI-FUNC-SUMMARY: Pauses synchronized playback; returns none; side effects: cancels timer.
  void _pausePlayback() {
    _playbackTimer?.cancel();
    _playbackTimer = null;
    if (_playing && mounted) {
      setState(() => _playing = false);
    } else {
      _playing = false;
    }
  }

  // AI-FUNC-SUMMARY: Exports a 6-second side-by-side MP4 comparison clip beside the review file; returns when finished; side effects: runs ffmpeg and shows snackbar.
  Future<void> _exportComparisonClip() async {
    if (!widget.controller.tools.areReady || _exportingClip) {
      return;
    }
    setState(() => _exportingClip = true);
    final reviewFile = File(widget.review.reviewPath);
    final outPath =
        '${reviewFile.parent.path}${Platform.pathSeparator}${widget.review.fileName}.compare.mp4';
    final maxStart = math.max(0.0, _durationSeconds - 6.0);
    final startSeconds = (_timestampSeconds - 3.0).clamp(0.0, maxStart);

    final error = await widget.controller.previewService.exportClip(
      ffmpeg: widget.controller.tools.ffmpeg.path,
      original: widget.review.originalPath,
      review: widget.review.reviewPath,
      startMs: (startSeconds * 1000).round(),
      durationMs: 6000,
      outPath: outPath,
    );

    if (!mounted) {
      return;
    }
    setState(() => _exportingClip = false);
    final text = AppText.of(context);
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(content: Text(error ?? '${text.clipExported}: $outPath')),
    );
  }

  // AI-FUNC-SUMMARY: Opens a video file in the operating system's default media player; returns when launched; side effects: starts OS opener process.
  Future<void> _openExternal(String path) async {
    try {
      if (Platform.isWindows) {
        await Process.start('cmd', ['/c', 'start', '', path]);
      } else if (Platform.isMacOS) {
        await Process.start('open', [path]);
      } else if (Platform.isLinux) {
        await Process.start('xdg-open', [path]);
      }
    } catch (error) {
      if (mounted) {
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(SnackBar(content: Text(error.toString())));
      }
    }
  }

  // AI-FUNC-SUMMARY: Applies a review decision and closes the comparison workspace on success; returns when finished; side effects: resolves review on disk.
  Future<void> _resolve(String decision) async {
    _pausePlayback();
    final error = await widget.controller.resolveReview(
      widget.review,
      decision,
    );
    if (!mounted) {
      return;
    }
    if (error != null) {
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text(error)));
      return;
    }
    Navigator.of(context).pop();
  }

  // AI-FUNC-SUMMARY: Jumps to a specific timestamp and updates the active comparison tab; returns none; side effects: extracts frames or clip sequence.
  void _jumpTo(double seconds) {
    final clamped = _durationSeconds > 0
        ? seconds.clamp(0.0, _durationSeconds)
        : math.max(0.0, seconds);
    setState(() => _timestampSeconds = clamped);
    if (_tabs.index == 0) {
      unawaited(_loadFramesAt(clamped));
    } else {
      unawaited(_loadClipSequence());
    }
  }

  @override
  // AI-FUNC-SUMMARY: Builds the dual-mode comparison workspace; returns the scaffold widget; side effects: none.
  Widget build(BuildContext context) {
    final text = AppText.of(context);
    final theme = Theme.of(context);
    final review = widget.review;
    final percent = review.sizePercent;

    return KeyboardListener(
      focusNode: _focusNode,
      autofocus: true,
      onKeyEvent: (event) {
        if (event.logicalKey == LogicalKeyboardKey.space) {
          if (event is KeyDownEvent && !_holdingOriginal) {
            setState(() => _holdingOriginal = true);
          } else if (event is KeyUpEvent && _holdingOriginal) {
            setState(() => _holdingOriginal = false);
          }
        }
      },
      child: Scaffold(
        appBar: AppBar(
          title: Text(review.fileName, overflow: TextOverflow.ellipsis),
          actions: [
            Padding(
              padding: const EdgeInsets.only(right: 12),
              child: Center(
                child: QualityBadge(
                  score: review.score,
                  notMeasuredLabel: text.qualityNotMeasured,
                ),
              ),
            ),
          ],
          bottom: TabBar(
            controller: _tabs,
            tabs: [
              Tab(icon: const Icon(Icons.compare), text: text.compareFrameTab),
              Tab(
                icon: const Icon(Icons.play_circle_outline),
                text: text.comparePlayTab,
              ),
            ],
          ),
        ),
        body: Column(
          children: [
            Container(
              width: double.infinity,
              padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
              color: theme.colorScheme.surfaceContainerLow,
              child: Wrap(
                spacing: 16,
                runSpacing: 4,
                crossAxisAlignment: WrapCrossAlignment.center,
                children: [
                  Text(
                    '${formatBytes(review.originalBytes)} → ${formatBytes(review.reviewBytes)}'
                    '${percent != null ? ' (${percent.toStringAsFixed(0)}%)' : ''}',
                    style: theme.textTheme.bodySmall,
                  ),
                  for (final dev in review.deviations)
                    Chip(
                      label: Text(dev, style: theme.textTheme.labelSmall),
                      visualDensity: VisualDensity.compact,
                    ),
                ],
              ),
            ),
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
              child: Wrap(
                spacing: 8,
                runSpacing: 8,
                crossAxisAlignment: WrapCrossAlignment.center,
                children: [
                  SegmentedButton<_CompareViewMode>(
                    selected: {_viewMode},
                    showSelectedIcon: false,
                    segments: [
                      ButtonSegment(
                        value: _CompareViewMode.wipe,
                        label: Text(text.modeWipe),
                      ),
                      ButtonSegment(
                        value: _CompareViewMode.toggle,
                        label: Text(text.modeToggle),
                      ),
                      ButtonSegment(
                        value: _CompareViewMode.sideBySide,
                        label: Text(text.modeSideBySide),
                      ),
                      if (_tabs.index == 0)
                        ButtonSegment(
                          value: _CompareViewMode.diff,
                          label: Text(text.modeDiff),
                        ),
                    ],
                    onSelectionChanged: (selection) =>
                        setState(() => _viewMode = selection.first),
                  ),
                  if (_tabs.index == 0)
                    TextButton.icon(
                      onPressed: () => _zoom.value = Matrix4.identity(),
                      icon: const Icon(Icons.zoom_out_map, size: 18),
                      label: Text(text.resetZoom),
                    ),
                ],
              ),
            ),
            Expanded(
              child: TabBarView(
                controller: _tabs,
                physics: const NeverScrollableScrollPhysics(),
                children: [
                  _buildFrameTab(context, text),
                  _buildPlaybackTab(context, text),
                ],
              ),
            ),
            _buildTimelineBar(context, text),
            _buildDecisionFooter(context, text),
          ],
        ),
      ),
    );
  }

  // AI-FUNC-SUMMARY: Builds the still-frame inspection canvas with zoom, wipe, A/B toggle, side-by-side, and diff map; returns the widget; side effects: none.
  Widget _buildFrameTab(BuildContext context, AppText text) {
    if (_loadingFrames) {
      return Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            const CircularProgressIndicator(),
            const SizedBox(height: 12),
            Text(text.loadingPreview),
          ],
        ),
      );
    }
    if (_frameError != null || _frames == null) {
      return Center(
        child: Padding(
          padding: const EdgeInsets.all(24),
          child: Text(_frameError ?? text.readyToolsMissing),
        ),
      );
    }

    final pair = _frames!;
    if (_viewMode == _CompareViewMode.diff && pair.diffImage != null) {
      return InteractiveViewer(
        transformationController: _zoom,
        minScale: 1,
        maxScale: 6,
        child: Center(child: Image.file(File(pair.diffImage!))),
      );
    }

    return _ComparisonCanvas(
      originalPath: pair.originalImage,
      reviewPath: pair.reviewImage,
      viewMode: _viewMode == _CompareViewMode.diff
          ? _CompareViewMode.wipe
          : _viewMode,
      wipePosition: _wipePosition,
      holdingOriginal: _holdingOriginal,
      zoomController: _zoom,
      onWipeChanged: (value) => setState(() => _wipePosition = value),
      onHoldChanged: (holding) => setState(() => _holdingOriginal = holding),
      originalLabel: text.showingOriginal,
      convertedLabel: text.showingConverted,
      holdHint: text.holdToCompareOrig,
    );
  }

  // AI-FUNC-SUMMARY: Builds the live synchronized playback tab with transport controls and external clip export; returns the widget; side effects: none.
  Widget _buildPlaybackTab(BuildContext context, AppText text) {
    if (_loadingSequence) {
      return Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            const CircularProgressIndicator(),
            const SizedBox(height: 12),
            Text(text.playbackPreparing),
          ],
        ),
      );
    }
    if (_sequenceError != null ||
        _sequence == null ||
        _sequence!.originalFrames.isEmpty) {
      return Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Text(_sequenceError ?? text.playbackPreparing),
            const SizedBox(height: 12),
            FilledButton.tonal(
              onPressed: _loadClipSequence,
              child: Text(text.refresh),
            ),
          ],
        ),
      );
    }

    final seq = _sequence!;
    final idx = _frameIndex.clamp(0, seq.originalFrames.length - 1);

    return Column(
      children: [
        Expanded(
          child: _ComparisonCanvas(
            originalPath: seq.originalFrames[idx],
            reviewPath: seq.reviewFrames[idx],
            viewMode: _viewMode == _CompareViewMode.diff
                ? _CompareViewMode.wipe
                : _viewMode,
            wipePosition: _wipePosition,
            holdingOriginal: _holdingOriginal,
            zoomController: null,
            onWipeChanged: (value) => setState(() => _wipePosition = value),
            onHoldChanged: (holding) =>
                setState(() => _holdingOriginal = holding),
            originalLabel: text.showingOriginal,
            convertedLabel: text.showingConverted,
            holdHint: text.holdToCompareOrig,
          ),
        ),
        Padding(
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 6),
          child: Wrap(
            spacing: 8,
            runSpacing: 6,
            crossAxisAlignment: WrapCrossAlignment.center,
            children: [
              FilledButton.tonalIcon(
                onPressed: _playing ? _pausePlayback : _startPlayback,
                icon: Icon(_playing ? Icons.pause : Icons.play_arrow),
                label: Text(_playing ? text.pauseLabel : text.playLabel),
              ),
              SegmentedButton<double>(
                selected: {_playbackSpeed},
                showSelectedIcon: false,
                segments: const [
                  ButtonSegment(value: 0.5, label: Text('0.5×')),
                  ButtonSegment(value: 1.0, label: Text('1×')),
                ],
                onSelectionChanged: (sel) {
                  setState(() => _playbackSpeed = sel.first);
                  if (_playing) {
                    _startPlayback();
                  }
                },
              ),
              OutlinedButton.icon(
                onPressed: _exportingClip ? null : _exportComparisonClip,
                icon: const Icon(Icons.movie_creation_outlined, size: 18),
                label: Text(text.exportComparisonClip),
              ),
              if (Platform.isWindows ||
                  Platform.isMacOS ||
                  Platform.isLinux) ...[
                TextButton(
                  onPressed: () => _openExternal(widget.review.originalPath),
                  child: Text(text.openOriginalExternal),
                ),
                TextButton(
                  onPressed: () => _openExternal(widget.review.reviewPath),
                  child: Text(text.openConvertedExternal),
                ),
              ],
            ],
          ),
        ),
      ],
    );
  }

  // AI-FUNC-SUMMARY: Builds the timeline scrubber and quick-jump chips including Worst Frame; returns the widget; side effects: none.
  Widget _buildTimelineBar(BuildContext context, AppText text) {
    final maxSeconds = math.max(1.0, _durationSeconds);
    final worst = widget.review.worstFrameSeconds;

    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 4, 16, 4),
      child: Column(
        children: [
          Row(
            children: [
              Text(
                formatDuration(Duration(seconds: _timestampSeconds.round())),
                style: Theme.of(context).textTheme.labelMedium,
              ),
              Expanded(
                child: Slider(
                  value: _timestampSeconds.clamp(0.0, maxSeconds),
                  min: 0,
                  max: maxSeconds,
                  onChanged: (value) =>
                      setState(() => _timestampSeconds = value),
                  onChangeEnd: _jumpTo,
                ),
              ),
              Text(
                formatDuration(Duration(seconds: maxSeconds.round())),
                style: Theme.of(context).textTheme.labelMedium,
              ),
            ],
          ),
          Wrap(
            spacing: 8,
            runSpacing: 4,
            children: [
              if (worst != null)
                ActionChip(
                  avatar: const Icon(Icons.warning_amber_outlined, size: 16),
                  label: Text(
                    '${text.worstFrame} (${formatDuration(Duration(seconds: worst.round()))})',
                  ),
                  onPressed: () => _jumpTo(worst),
                ),
              for (final fraction in const [0.15, 0.50, 0.85])
                ActionChip(
                  label: Text('${(fraction * 100).round()}%'),
                  onPressed: () => _jumpTo(maxSeconds * fraction),
                ),
            ],
          ),
        ],
      ),
    );
  }

  // AI-FUNC-SUMMARY: Builds the sticky bottom decision bar so the user can decide immediately after comparing; returns the widget; side effects: none.
  Widget _buildDecisionFooter(BuildContext context, AppText text) {
    return SafeArea(
      top: false,
      child: Container(
        width: double.infinity,
        padding: const EdgeInsets.fromLTRB(16, 8, 16, 12),
        decoration: BoxDecoration(
          border: Border(
            top: BorderSide(color: Theme.of(context).dividerColor),
          ),
        ),
        child: Wrap(
          spacing: 8,
          runSpacing: 8,
          alignment: WrapAlignment.end,
          children: [
            FilledButton(
              onPressed: () => _resolve('keep-new'),
              child: Text(text.keepNew),
            ),
            OutlinedButton(
              onPressed: () => _resolve('keep-original'),
              child: Text(text.keepOriginal),
            ),
            TextButton(
              onPressed: () => _resolve('keep-both'),
              child: Text(text.keepBoth),
            ),
          ],
        ),
      ),
    );
  }
}

class _ComparisonCanvas extends StatelessWidget {
  const _ComparisonCanvas({
    required this.originalPath,
    required this.reviewPath,
    required this.viewMode,
    required this.wipePosition,
    required this.holdingOriginal,
    required this.zoomController,
    required this.onWipeChanged,
    required this.onHoldChanged,
    required this.originalLabel,
    required this.convertedLabel,
    required this.holdHint,
  });

  final String originalPath;
  final String reviewPath;
  final _CompareViewMode viewMode;
  final double wipePosition;
  final bool holdingOriginal;
  final TransformationController? zoomController;
  final ValueChanged<double> onWipeChanged;
  final ValueChanged<bool> onHoldChanged;
  final String originalLabel;
  final String convertedLabel;
  final String holdHint;

  @override
  // AI-FUNC-SUMMARY: Renders the active comparison view mode over synchronized original and review images; returns the widget; side effects: none.
  Widget build(BuildContext context) {
    final origImage = Image.file(
      File(originalPath),
      gaplessPlayback: true,
      fit: BoxFit.contain,
    );
    final revImage = Image.file(
      File(reviewPath),
      gaplessPlayback: true,
      fit: BoxFit.contain,
    );

    if (viewMode == _CompareViewMode.sideBySide) {
      return Row(
        children: [
          Expanded(
            child: Stack(
              fit: StackFit.expand,
              children: [
                _maybeZoom(origImage),
                Positioned(
                  left: 12,
                  top: 12,
                  child: _LabelPill(text: originalLabel),
                ),
              ],
            ),
          ),
          const VerticalDivider(width: 2),
          Expanded(
            child: Stack(
              fit: StackFit.expand,
              children: [
                _maybeZoom(revImage),
                Positioned(
                  right: 12,
                  top: 12,
                  child: _LabelPill(text: convertedLabel),
                ),
              ],
            ),
          ),
        ],
      );
    }

    if (viewMode == _CompareViewMode.toggle) {
      return GestureDetector(
        onTapDown: (_) => onHoldChanged(true),
        onTapUp: (_) => onHoldChanged(false),
        onTapCancel: () => onHoldChanged(false),
        child: Stack(
          fit: StackFit.expand,
          children: [
            _maybeZoom(holdingOriginal ? origImage : revImage),
            Positioned(
              left: 12,
              top: 12,
              child: _LabelPill(
                text: holdingOriginal ? originalLabel : convertedLabel,
              ),
            ),
            Positioned(
              bottom: 12,
              left: 0,
              right: 0,
              child: Center(child: _LabelPill(text: holdHint)),
            ),
          ],
        ),
      );
    }

    return LayoutBuilder(
      builder: (context, constraints) {
        final dividerX = (constraints.maxWidth * wipePosition).clamp(
          0.0,
          constraints.maxWidth,
        );
        return GestureDetector(
          behavior: HitTestBehavior.opaque,
          onHorizontalDragUpdate: (details) {
            if (constraints.maxWidth > 0) {
              onWipeChanged(
                (details.localPosition.dx / constraints.maxWidth).clamp(
                  0.02,
                  0.98,
                ),
              );
            }
          },
          child: Stack(
            fit: StackFit.expand,
            children: [
              _maybeZoom(revImage),
              ClipRect(
                clipper: _WipeClipper(wipePosition),
                child: _maybeZoom(origImage),
              ),
              Positioned(
                left: dividerX - 1,
                top: 0,
                bottom: 0,
                width: 2,
                child: Container(color: Colors.white),
              ),
              Positioned(
                left: 12,
                top: 12,
                child: _LabelPill(text: originalLabel),
              ),
              Positioned(
                right: 12,
                top: 12,
                child: _LabelPill(text: convertedLabel),
              ),
            ],
          ),
        );
      },
    );
  }

  // AI-FUNC-SUMMARY: Wraps an image in a shared InteractiveViewer when zoom is enabled; returns the widget; side effects: none.
  Widget _maybeZoom(Widget child) {
    if (zoomController == null) {
      return Center(child: child);
    }
    return InteractiveViewer(
      transformationController: zoomController,
      minScale: 1,
      maxScale: 6,
      child: Center(child: child),
    );
  }
}

class _WipeClipper extends CustomClipper<Rect> {
  const _WipeClipper(this.fraction);

  final double fraction;

  @override
  // AI-FUNC-SUMMARY: Clips the original overlay to the left fraction of the comparison canvas; returns clip rect; side effects: none.
  Rect getClip(Size size) =>
      Rect.fromLTWH(0, 0, size.width * fraction.clamp(0.0, 1.0), size.height);

  @override
  // AI-FUNC-SUMMARY: Reclips whenever the wipe slider moves; returns true on fraction changes; side effects: none.
  bool shouldReclip(_WipeClipper oldClipper) => oldClipper.fraction != fraction;
}

class _LabelPill extends StatelessWidget {
  const _LabelPill({required this.text});

  final String text;

  @override
  // AI-FUNC-SUMMARY: Renders a high-contrast overlay badge for original/converted indicators; returns the widget; side effects: none.
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 4),
      decoration: BoxDecoration(
        color: Colors.black.withValues(alpha: 0.68),
        borderRadius: BorderRadius.circular(999),
      ),
      child: Text(
        text,
        style: Theme.of(
          context,
        ).textTheme.labelSmall?.copyWith(color: Colors.white),
      ),
    );
  }
}
