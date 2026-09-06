# Planning and transcoding

Building the ordered list of encode attempts for one file and running each of them. Described for
readers in [../options.md](../options.md) and [../encoders.md](../encoders.md).

| Declaration | Kind | Purpose |
|---|---|---|
| `codec_forced_pixel_format` | function | Reports the conversion a codec forces on every source it cannot take directly, returning an adapted plan kind or none. |
| `metadata_exemptions_for` | function | Works out which colour details the chosen codec cannot record for this source, returning the exemptions a flexible run may accept. |
| `exemption_descriptions` | function | Describes each accepted colour exemption for the user, returning one sentence per change. |
| `closest_hardware_pixel_format` | function | Chooses the closest safe initial hardware pixel-format target, returning target plus change description or none when adaptation is unsafe/unnecessary. |
| `pixel_format_bit_depth` | function | Extracts an explicit YUV planar bit depth such as 9, 10, 12, 14, or 16 from an FFmpeg pixel-format name, returning none for implicit 8-bit names. |
| `pixel_format_has_alpha` | function | Detects alpha-bearing pixel formats that must not be adapted automatically, returning true for known YUV/RGB alpha formats. |
| `transcode_item` | function | Produces and commits one validated output while allowing safe exact/adapted encoder fallback.. |
| `search_quality_for` | function | Tunes the quality setting for one file by encoding and measuring short samples.. |
| `plan_deviations` | function | Lists the recoverable differences one encode plan introduces, returning a kind and sentence for each. |
| `assess_quality` | function | Measures a finished conversion and decides whether it needs a human decision, returning the outcome. |
| `finish_conversion` | function | Commits a finished conversion, either replacing the original or keeping both for review, returning the conversion sizes or a terminal error. |
| `commit_for_review` | function | Moves a finished conversion beside its untouched original and records why it needs a decision, returning the conversion sizes or a terminal error. |
| `execute_transcode_attempt` | function | Executes one exact or explicitly adapted encoder attempt without committing it, returning success or classified failure. |
| `remove_failed_attempt_output` | function | Removes a failed-attempt temp output before reuse, returning success or terminal disk error. |
| `commit_validated_output` | function | Reads sizes and commits one validated output, returning conversion sizes or terminal error. |
| `ffmpeg_failure_is_encoder_retryable` | function | Classifies an ffmpeg failure as a safe encoder fallback case, returning true only for known device/encoder/format capability diagnostics. |
| `validation_failure_is_retryable` | function | Classifies strict output validation failures for encoder fallback, returning false for tool/process/repair infrastructure failures. |
| `validate_or_repair_output` | function | Validates validate or repair output conditions, returning success, failure, or test assertion result. |
| `prepare_cached_temp_output` | function | Finds a previously generated MyVidComp temp that validates under the current exact or hardware-adapted policy.. |
| `validate_cached_output` | function | Validates a cached output against exact preservation or the current hardware-mode adaptation, returning success for either allowed plan. |
| `validation_error_may_need_remux_repair` | function | Checks whether a validation failure may be fixed by a no-reencode remux or AV1 metadata repair, returning true for stream-count and color/chroma metadata failures. |
| `validation_error_indicates_unusable_temp` | function | Checks whether a validation failure means a temp output cannot be safely reused, returning true for unreadable or structurally wrong outputs. |
| `remove_unusable_temp_output` | function | Deletes a temp output after an unrecoverable validation failure, returning nothing. |
| `is_temp_output_path` | function | Checks whether a path looks like a temporary conversion output owned by this tool, returning true for current .myvidcomp-*.tmp.mp4/.mkv names and legacy .pvac-* names. |
| `file_size` | function | Provides file size behavior, returning the declared result. |
| `build_ffmpeg_args` | function | Builds the ffmpeg arguments for a plain exact AV1 attempt, for tests, returning command arguments. |
| `build_ffmpeg_args_for_plan` | function | Builds ffmpeg arguments for one encode attempt, returning command arguments. |
| `ensure_metadata_bsf` | function | Checks that this ffmpeg build carries the bitstream filter one codec needs for colour metadata, returning success or a user-facing error. |
| `repair_av1_metadata` | function | Performs repair av1 metadata operation, returning operation status or result. |
| `build_av1_metadata_repair_args` | function | Builds or derives build av1 metadata repair args data, returning the computed value. |
| `stream_map_args` | function | Builds explicit FFmpeg mappings from actual probed stream indexes, returning one -map pair per index and never emits a broad map. |
| `replace_with_repaired_output` | function | Performs replace with repaired output operation, returning operation status or result. |
| `ffmpeg_failure_detail` | function | Provides ffmpeg failure detail behavior, returning the declared result. |
| `output_frame_rate_arg` | function | Provides output frame rate arg behavior, returning the declared result. |
| `output_pix_fmt_arg` | function | Provides output pix fmt arg behavior, returning the declared result. |
| `normalized_pix_fmt` | function | Builds or derives normalized pix fmt data, returning the computed value. |
| `color_metadata_args` | function | Provides color metadata args behavior, returning the declared result. |
| `metadata_bsf_arg` | function | Builds the bitstream-filter argument that carries the source colour metadata into the encoded stream, returning the argument or none when there is nothing to carry. |
| `unsupported_metadata_reason` | function | Reports the first colour detail the chosen codec cannot record, returning a skip reason or none when everything can be preserved. |
| `av1_color_primaries_value` | function | Provides av1 color primaries value behavior, returning the declared result. |
| `av1_transfer_characteristics_value` | function | Provides av1 transfer characteristics value behavior, returning the declared result. |
| `av1_matrix_coefficients_value` | function | Provides av1 matrix coefficients value behavior, returning the declared result. |
| `metadata_key` | function | Provides metadata key behavior, returning the declared result. |
| `aspect_metadata_args` | function | Provides aspect metadata args behavior, returning the declared result. |
| `push_optional_arg` | function | Provides push optional arg behavior, returning the declared result. |
| `label` | function | Describes a quality setting for logs and events, returning a short label. |
| `default_encoder_quality` | function | Chooses the starting quality setting for one file and encoder, returning the setting on that encoder's own scale. |
| `quality_value_arg` | function | Formats a constant-quality value the way its encoder expects it, returning the argument text. |
| `encoder_quality_args` | function | Builds the quality arguments for one encoder at one setting, returning the argument list. |
| `run_ffmpeg_with_progress` | function | Runs ffmpeg while reporting progress, returning exit status plus captured stderr or process-management error. |
| `read_to_string` | function | Provides read to string behavior, returning the declared result. |
| `validate_output` | function | Validates validate output conditions, returning success, failure, or test assertion result. |
| `validate_output_for_plan` | function | Validates output against an exact or adapted plan, returning success or validation error. |
| `validate_output_against` | function | Validates probed output against expected primary-video properties and source stream layout, returning success or validation error. |
