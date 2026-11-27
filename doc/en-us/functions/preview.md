# Preview and comparison

Backend frame extraction, difference heatmap generation, synchronized JPEG burst playback, and side-by-side clip export used by the interactive comparison inspector. Described for readers in [../review.md](../review.md) and [../ffi-abi.md](../ffi-abi.md).

| Declaration | Kind | Purpose |
|---|---|---|
| `probe_preview_info` | function | Probes a video file for preview duration, frame count, pixel dimensions, and frame rate, returning metadata or an error. |
| `preview_info_json` | function | Serializes preview stream metadata into compact JSON for FFI callers, returning the JSON text. |
| `build_single_frame_args` | function | Constructs FFmpeg arguments to extract one exact high-resolution PNG still frame at a given timestamp, returning the argument list. |
| `build_diff_heatmap_args` | function | Constructs FFmpeg arguments combining original and converted frames with `blend=all_mode=difference` contrast enhancement, returning the argument list. |
| `extract_frame_pair` | function | Extracts synchronized original and converted PNG still frames plus an optional difference heatmap, returning output paths or an error. |
| `frame_pair_json` | function | Serializes extracted still-frame comparison paths into JSON, returning the JSON text. |
| `build_clip_frames_args` | function | Constructs FFmpeg arguments to decode a short burst of synchronized 720p JPEG frames, returning the argument list. |
| `extract_clip_sequence` | function | Extracts paired original and converted JPEG frame bursts into a cache folder for live in-app comparison playback, returning frame path lists or an error. |
| `clip_sequence_json` | function | Serializes extracted JPEG playback burst paths and timing into JSON, returning the JSON text. |
| `export_comparison_clip` | function | Encodes a 6-second horizontally stacked MP4 comparison clip combining original and converted video, returning success or an error. |
| `list_usable_encoders_json` | function | Probes FFmpeg encoders matching a target codec and validates their runtime usability, returning serialized candidate descriptors as JSON. |
| `run_ffmpeg_quiet` | function | Runs FFmpeg via `process_command` without flashing a console window, returning success or the primary stderr diagnostic line. |
| `clear_frame_files` | function | Removes stale PNG/JPEG preview artifacts inside a working preview directory, returning nothing. |
| `collect_prefixed_jpegs` | function | Collects and sorts extracted JPEG frames matching a sequence prefix inside a preview folder, returning ordered paths. |
