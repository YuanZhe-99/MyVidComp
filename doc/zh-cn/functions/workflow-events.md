# 工作流与事件

运行过程本身、它报告的事件，以及渲染这些事件的进度界面。面向读者的说明见
[../architecture.md](../architecture.md)。

声明名称与用途说明直接取自源码中的 `AI-FUNC-SUMMARY` 注释，按本仓库的编写规则，这些注释以英文书写，
因此不作翻译。

| 声明 | 种类 | 用途 |
|---|---|---|
| `process_command` | function | Creates a child-process command without a console window on Windows, returning a configurable command. |
| `as_str` | function | Maps an output format to its stable wire value, returning static label. |
| `label` | function | Maps an output format to its human-readable log label, returning static label. |
| `parse_output_format` | function | Parses an output-format wire value, returning the format or a user-facing error. |
| `normalize_output_format_choice` | function | Normalizes an optional output-format value, returning none for blank/auto/default or a parsed format. |
| `default` | function | Builds default embedded run options, returning options matching CLI defaults. |
| `review_threshold` | function | Reports the score below which a result must be reviewed, returning the threshold in hundredths. |
| `conversion_attempts` | function | Counts conversion attempts represented by the summary, returning converted plus failed files. |
| `on_event` | function | Receives a structured MyVidComp workflow event, returning none. |
| `on_event` | function | Forwards a MyVidComp workflow event into a closure sink, returning none. |
| `default` | function | Constructs a non-cancelled token, returning the token. |
| `new` | function | Constructs a non-cancelled token, returning the token. |
| `request_stop` | function | Requests graceful cancellation after the active file completes, returning none. |
| `is_stop_requested` | function | Checks whether graceful cancellation has been requested, returning a boolean. |
| `on_event` | function | Discards a structured MyVidComp workflow event, returning none. |
| `event_json` | function | Serializes a MyVidComp event to JSON for FFI callbacks, returning compact JSON text. |
| `candidate_json` | function | Serializes a dry-run candidate preview to JSON, returning compact JSON text. |
| `summary_json` | function | Serializes a run summary to JSON, returning compact JSON text. |
| `json_path` | function | Serializes a path value to a JSON string, returning escaped JSON text. |
| `json_string` | function | Serializes a string value to JSON, returning escaped JSON string text. |
| `json_optional_string` | function | Serializes an optional string to JSON, returning string or null JSON text. |
| `json_optional_u32` | function | Serializes an optional 32-bit count for JSON output, returning the number or null. |
| `json_optional_usize` | function | Serializes an optional usize to JSON, returning number or null JSON text. |
| `json_optional_u64` | function | Serializes an optional u64 to JSON, returning number or null JSON text. |
| `json_f64` | function | Serializes a finite float to JSON, returning a number string with fallback for non-finite values. |
| `log_level_label` | function | Maps a log level to its JSON label, returning static label. |
| `file_status_label` | function | Maps a file status to its JSON label, returning static label. |
| `main_entry` | function | Runs the MyVidComp command-line workflow, returning process result through normal CLI control flow. |
| `run` | function | Runs the MyVidComp command-line workflow, returning process result through normal CLI control flow. |
| `print_reviews` | function | Prints every conversion in a folder that is waiting for a decision, returning success. |
| `resolve_all_reviews` | function | Applies one decision to every pending review in a folder, returning success or the first failure. |
| `run_with_events` | function | Runs MyVidComp from an embedded caller using structured events instead of terminal UI.. |
| `run_workflow` | function | Executes the shared MyVidComp workflow for both terminal CLI and embedded callers.. |
| `validate_run_options` | function | Validates embedded run options before workflow side effects, returning success or a user-facing error. |
| `move_validated_output` | function | Performs move validated output operation, returning operation status or result. |
| `move_validated_output_with_ui` | function | Performs move validated output with ui operation, returning operation status or result. |
| `move_validated_output_with_progress` | function | Performs move validated output with progress operation, returning operation status or result. |
| `copy_file_to_new_path` | function | Performs copy file to new path operation, returning operation status or result. |
| `copy_file_to_new_path_with_progress` | function | Performs copy file to new path with progress operation, returning operation status or result. |
| `final_commit_temp_path` | function | Provides final commit temp path behavior, returning the declared result. |
| `metadata_repair_temp_path` | function | Provides metadata repair temp path behavior, returning the declared result. |
| `metadata_backup_temp_path` | function | Provides metadata backup temp path behavior, returning the declared result. |
| `apply` | function | Provides apply behavior, returning the declared result. |
| `parse_progress_line` | function | Parses progress line input, returning parsed values or errors. |
| `parse_hms` | function | Parses hms input, returning parsed values or errors. |
| `start` | function | Starts the optional stdin listener for graceful shutdown requests.. |
| `prompt_active_flag` | function | Clones the prompt-active flag for progress rendering coordination, returning shared flag handle. |
| `wait_until_prompt_inactive` | function | Waits for an active quit confirmation prompt to finish, returning when UI output is safe. |
| `graceful_exit_input_loop` | function | Reads stdin commands and records a confirmed graceful-exit request.. |
| `new` | function | Constructs the progress UI state, returning a new instance. |
| `set_codec` | function | Records the codec this run produces, returning none. |
| `set_quality_available` | function | Records whether quality can be measured in this run, returning none. |
| `emit_quality_measured` | function | Reports a finished quality measurement, returning none. |
| `emit_quality_search` | function | Reports one step of a quality search, returning none. |
| `emit_review_pending` | function | Reports that a conversion is waiting for a keep-or-discard decision, returning none. |
| `emit_capability_missing` | function | Reports a capability this ffmpeg build lacks, returning none. |
| `prompt_active` | function | Checks whether quit confirmation currently owns the terminal, returning true while progress rendering should pause. |
| `wait_until_prompt_inactive` | function | Waits until quit confirmation no longer owns the terminal, returning once structured UI output can safely print. |
| `start_file` | function | Provides start file behavior, returning the declared result. |
| `start_attempt` | function | Starts one encoder attempt for the active file, returning none. |
| `render_file_progress` | function | Provides render file progress behavior, returning the declared result. |
| `render_copy_progress` | function | Provides render copy progress behavior, returning the declared result. |
| `render_stage` | function | Provides render stage behavior, returning the declared result. |
| `finish_file` | function | Provides finish file behavior, returning the declared result. |
| `emit_file_skipped` | function | Emits a file-skipped event, returning none. |
| `emit_file_finished` | function | Emits a file-finished event, returning none. |
| `emit_stop_requested` | function | Emits a stop-requested event, returning none. |
| `emit_summary` | function | Emits the final summary event, returning none. |
| `emit_log` | function | Emits a log event without terminal rendering, returning none. |
| `log_info` | function | Emits an informational log and optional terminal line, returning none. |
| `log_warning` | function | Emits a warning log and optional terminal line, returning none. |
| `log_error` | function | Emits an error log and optional terminal line, returning none. |
| `log` | function | Emits a log message to terminal mode and the structured event sink, returning none. |
| `total_label` | function | Provides total label behavior, returning the declared result. |
| `quality_label` | function | Provides quality label behavior, returning the declared result. |
| `conversion_size_label` | function | Formats one conversion's before/after byte counts, returning a concise size change label. |
| `print_summary` | function | Provides print summary behavior, returning the declared result. |
| `print_dry_run` | function | Provides print dry run behavior, returning the declared result. |
| `source_stream_policy` | function | Determines mapped streams and chapter behavior without guessing that arbitrary text/data tracks are disposable.. |
