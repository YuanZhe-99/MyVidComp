# 编码器与画质估算

找出真正可用的编码器、为一个文件排定尝试顺序，以及在不做试编码的情况下估算画质设置。面向读者的说明见
[../encoders.md](../encoders.md)。

声明名称与用途说明直接取自源码中的 `AI-FUNC-SUMMARY` 注释，按本仓库的编写规则，这些注释以英文书写，
因此不作翻译。

| 声明 | 种类 | 用途 |
|---|---|---|
| `detect_with_events` | function | Detects all usable AV1 encoders and reports runtime probe diagnostics, returning an ordered selection or an error. |
| `preferred` | function | Returns the preferred exact encoder candidate, returning the first GPU-first candidate. |
| `detect_requested_encoder` | function | Provides detect requested encoder behavior, returning the declared result. |
| `encoder_from_name` | function | Provides encoder from name behavior, returning the declared result. |
| `supported_encoder_names` | function | Provides supported encoder names behavior, returning the declared result. |
| `detect_encoder_from_listing` | function | Provides detect encoder from listing behavior, returning the declared result. |
| `detect_encoder_candidates_from_listing` | function | Filters the advertised encoder listing into GPU-first candidates, returning candidates in deterministic probe order. |
| `check_encoder_runtime` | function | Validates check encoder runtime conditions, returning success, failure, or test assertion result. |
| `encoder_runtime_check_args` | function | Provides encoder runtime check args behavior, returning the declared result. |
| `encoder_runtime_quality_args` | function | Provides encoder runtime quality args behavior, returning the declared result. |
| `first_error_line` | function | Picks the line of ffmpeg output that actually explains a failure, returning that line, or the last line when nothing looks like an error. |
| `is_ffmpeg_noise` | function | Checks whether a line of ffmpeg output is routine chatter rather than a message worth showing, returning true for banners, progress and stream descriptions. |
| `line_looks_like_an_error` | function | Checks whether a line of ffmpeg output reads like a failure, returning true when it names an error. |
| `encoder_list_contains` | function | Provides encoder list contains behavior, returning the declared result. |
| `ensure_binary` | function | Validates ensure binary conditions, returning success, failure, or test assertion result. |
| `ensure_tmp_dir` | function | Validates ensure tmp dir conditions, returning success, failure, or test assertion result. |
| `default_runtime_binary` | function | Provides default runtime binary behavior, returning the declared result. |
| `bundled_binary_candidates` | function | Provides bundled binary candidates behavior, returning the declared result. |
| `push_binary_candidates` | function | Provides push binary candidates behavior, returning the declared result. |
| `bundled_binary_names` | function | Provides bundled binary names behavior, returning the declared result. |
| `platform_binary_name` | function | Provides platform binary name behavior, returning the declared result. |
| `discover_files` | function | Discovers or probes discover files data, returning collected metadata. |
| `is_candidate_video_path` | function | Checks is candidate video path predicate, returning a boolean. |
| `is_candidate_video_extension` | function | Checks is candidate video extension predicate, returning a boolean. |
| `visit_dir` | function | Discovers or probes visit dir data, returning collected metadata. |
| `probe_video` | function | Probes the primary video stream, returning metadata, none for unreadable media, or a tool/process error. |
| `estimate_av1_crf_from_bitrate` | function | Builds or derives estimate av1 crf from bitrate data, returning the computed value. |
| `apply_quality_preservation_adjustment` | function | Provides apply quality preservation adjustment behavior, returning the declared result. |
| `contains_any_ignore_case` | function | Provides contains any ignore case behavior, returning the declared result. |
| `source_codec_family` | function | Provides source codec family behavior, returning the declared result. |
| `map_source_quantizer_to_av1` | function | Builds or derives map source quantizer to av1 data, returning the computed value. |
| `codec_efficiency_factor` | function | Provides codec efficiency factor behavior, returning the declared result. |
| `estimate_target_bitrate` | function | Builds or derives estimate target bitrate data, returning the computed value. |
| `hardware_bitrate_ratio` | function | Provides hardware bitrate ratio behavior, returning the declared result. |
| `fmt` | function | Provides fmt behavior, returning the declared result. |
| `extract_named_number` | function | Provides extract named number behavior, returning the declared result. |
| `parse_ratio` | function | Parses ratio input, returning parsed values or errors. |
| `parse_frame_rate` | function | Parses frame rate input, returning parsed values or errors. |
| `progress_bar` | function | Provides progress bar behavior, returning the declared result. |
| `estimate_eta` | function | Builds or derives estimate eta data, returning the computed value. |
| `format_duration` | function | Builds or derives format duration data, returning the computed value. |
| `format_bytes` | function | Builds or derives format bytes data, returning the computed value. |
| `size_change_label` | function | Provides size change label behavior, returning the declared result. |
| `copy_speed_label` | function | Performs copy speed label operation, returning operation status or result. |
| `nonzero` | function | Provides nonzero behavior, returning the declared result. |
| `with_added_suffix` | function | Provides with added suffix behavior, returning the declared result. |
| `has_added_suffix` | function | Checks has added suffix predicate, returning a boolean. |
| `temp_output_path` | function | Provides temp output path behavior, returning a timestamped MyVidComp temp path with the correct container extension. |
| `cached_temp_output_paths` | function | Provides cached temp output paths behavior, returning matching MyVidComp temp files for both containers sorted newest-first. |
