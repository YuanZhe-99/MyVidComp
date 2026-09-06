# Version history

Dated record of MyVidComp behaviour changes. Useful for understanding *why* a behavior exists before
changing it — several entries record deliberate safety fixes that look like quirks otherwise.
MyVidComp uses semantic versions. `--version` prints the version, and this page records what
changed in each one.

## Timeline

- `2026-05-22`: Initial Rust CLI, config support, bundled runtime lookup, AV1 encoder selection,
  quality estimation, validation, progress UI, and Windows packaging.
- `2026-05-22`: Added SMB-friendly discovery, `tmp_dir`, cross-device commit fallback, cached temp
  handling, explicit frame-rate preservation, and source color/pixel metadata handling.
- `2026-05-22`: Tightened output preservation so only the primary video codec may change; stream
  count/order/codecs and pixel/display metadata are validated.
- `2026-05-23`: Added runtime-checked hardware encoder priority, configurable encoder override,
  package checksums, build marker, and `--version` output.
- `2026-05-23`: Reworked stream mapping to explicit source stream indexes, disabled forced MP4
  chapter mapping, made AV1/MP4 missing chroma-location reports advisory, and added cached temp
  validation before re-encoding.
- `2026-05-23`: Removed `PLAN.md`, consolidated behavior policy into `README.md` and `AGENTS.md`,
  split the binary entry point from the core library, and added `AI-FUNC-SUMMARY` comments.
- `2026-05-23`: Added ffprobe stream-index deduplication and pre-encode skipping for
  MP4-incompatible data or unknown streams.
- `2026-05-23`: Made sample/display aspect-ratio validation tolerant to equivalent rational
  rewrites and limited metadata repair attempts to remux-repairable failures.
- `2026-05-25`: Added confirmed graceful stop handling with paused progress refresh during
  confirmation, per-file size-change output, `-nostdin` ffmpeg runs, and cleanup for unreadable
  MyVidComp temp outputs.
- `2026-06-04`: Added Windows ARM64 packaging notes for `aarch64-pc-windows-gnullvm`, copied
  llvm-mingw runtime DLLs into ARM64 packages, and documented Snapdragon/Qualcomm AV1 hardware
  detection via `av1_mf` then `av1_vulkan`.
- `2026-06-07`: Renamed user-facing project documentation to MyVidComp-only branding and removed the
  expanded product name from repo docs.
- `2026-06-21`: Added the embedded `RunOptions`/`run_with_events` core API, structured `PvacEvent`
  reporting, `CancellationToken` support, and a `cdylib` C ABI for future Flutter/Dart FFI GUI
  integration while preserving CLI behavior.
- `2026-06-21`: Added the first Flutter desktop GUI under `gui/`, Dart FFI bindings, worker-isolate
  execution, settings/progress/log UI, and Windows CMake integration that builds and installs
  `myvidcomp_core.dll` with the GUI.
- `2026-06-21`: Completed the Windows GUI workflow with settings persistence, picker buttons,
  typed Dart event handling, `scripts/package-gui-windows.ps1`, package checksums, and package
  output verification.
- `2026-06-22`: Fixed idle GUI progress bars, added automatic FFmpeg/FFprobe detection in the GUI,
  and added `-DownloadFfmpeg` Windows GUI packaging with package-local `bin/` runtime tools.
- `2026-06-22`: Added GUI localization for English, Simplified Chinese, Traditional Chinese, and
  Japanese, switched encoder choices to user-friendly display labels, and changed the Windows GUI
  title/product display name to `MyVidComp` while keeping `MyVidComp.exe`.
- `2026-07-18`: Added the `ConversionMode` policy (`consistency` default, `hardware` opt-in) across
  CLI (`--conversion-mode`), config (`conversion_mode`), `RunOptions`, GUI settings schema v2, and
  localized GUI strategy selector; automatic encoder ranking is now mode-dependent
  (software-first vs hardware-first) while output validation stays identical; added the versioned
  FFI V2 ABI (`PvacFfiRunOptionsV2`, `pvac_run_blocking_v2`, `pvac_ffi_abi_version`) and the
  `mode_selected` event.
- `2026-07-18`: Added deterministic ARM64+x64 Windows GUI release packaging through
  `package-gui-windows-all.ps1`, target-specific Rust MSVC core builds, explicit single-package
  platform selection, and PE architecture validation for every packaged EXE/DLL.
- `2026-07-19`: Changed both conversion modes to GPU-first per-file planning, added exact
  GPU-to-CPU fallback for consistency mode, minimum-difference YUV GPU adaptation for hardware
  mode, structured encoder/attempt diagnostics, conservative retry classification, and hidden
  Windows child-process creation.
- `2026-07-20`: Added the `OutputFormat` policy (`mp4` default, `mkv-fallback` opt-in) across CLI
  (`--output-format`), config (`output_format`), `RunOptions`, GUI settings schema v3, and
  localized GUI output-format selector; split `SkipReason::UnsafeReplacement` into specific detail
  messages; added FFI V3 ABI (`PvacFfiRunOptionsV3`, `pvac_run_blocking_v3`,
  `output_format_selected` event).
- `2026-07-21`: Added actual-index stream mapping and metadata-repair probing, bounded ISO BMFF
  chapter-reference detection, confirmed chapter-carrier filtering, placeholder/meaningful chapter
  mapping and validation policies, conservative Matroska stream-type fallback, and restored the
  conservative MP4 codec whitelist. Documentation moved from `AGENTS.md` into this `doc/` tree
  with a bilingual `en-us`/`zh-cn` structure.
- `2026-09-05`: Released as **MyVidComp 0.1.0**, renamed throughout from the previous project.
  Added H.265 and H.266 as output codecs beside AV1, with per-codec encoder tables, quality
  controls and colour-metadata handling. Added quality measurement with VMAF, and a tuning search
  that samples a file to find the smallest setting still reaching a quality target. Replaced the
  conversion-mode and output-format pair with codec, quality target, quality strategy, quality
  check, preservation and encoder preference; the retired `--conversion-mode` flag and its values
  are still accepted. Added flexible preservation, which converts files the strict policy refused
  and keeps both copies for the user to choose between, with a record of what changed. Replaced
  the three-version embedding ABI with a single version 1. Rebuilt the application around four
  screens and an adaptive layout for phone, tablet and desktop, with every setting explained in
  plain words in four languages. Added an Android build. Reset the version to 0.1.0 and removed
  the build marker.
- `2026-09-05`: Stopped asking FFmpeg for automatic hardware decoding. It crashed outright on
  roughly one run in six during testing, losing a whole file's work, and the encode dominates the
  time anyway. Hardware encoding is unaffected. An FFmpeg failure with no error message is now
  treated as retryable rather than fatal, and the message shown for a failure is chosen from the
  end of its output rather than the first line, which was usually a banner.
- `2026-09-05`: Added a duration check to output validation. A truncated encode previously passed
  every other check, because resolution, frame rate, stream layout and metadata all still matched.
