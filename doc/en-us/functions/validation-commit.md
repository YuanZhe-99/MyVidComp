# Validation and commit

Checking a finished file against its source and putting it in place. Described for readers in
[../validation.md](../validation.md) and [../safety.md](../safety.md).

| Declaration | Kind | Purpose |
|---|---|---|
| `validate_chapter_policy` | function | Validates output chapters only for confirmed chapter-carrier sources, returning semantic timestamp/title mismatch detail or success. |
| `chapter_validation_error` | function | Compares expected and output chapter count, timestamps, and exact titles, returning detailed mismatch with practical timestamp tolerance or none. |
| `pixel_format_validation_error` | function | Provides pixel format validation error behavior, returning the declared result. |
| `validate_stream_signature` | function | Validates validate stream signature conditions, returning success, failure, or test assertion result. |
| `stream_signature_validation_error` | function | Provides stream signature validation error behavior, returning the declared result. |
| `display_metadata_validation_error` | function | Provides display metadata validation error behavior, returning the declared result. |
| `display_metadata_matches` | function | Compares display metadata with rational tolerance for aspect ratios, returning true when values are equivalent enough for validation. |
| `rational_metadata_matches` | function | Compares rational metadata values with a small tolerance for container rewrites, returning true when ratios are visually equivalent. |
| `duration_validation_error` | function | Compares the converted duration against the source under a fresh-encode or reused-temp tolerance, returning an error when the output is noticeably shorter or longer. |
| `tolerance_seconds` | function | Gives the allowed duration difference for a source of the given length, returning the larger of 0.5 s and 1% for a fresh encode, or of 0.5 s and 0.1% for a reused temp. |
| `codec_mismatch_error` | function | Words the error for an output in a codec other than the one the run asked for, returning the message the unusable-temp check recognizes. |
| `color_metadata_validation_error` | function | Provides color metadata validation error behavior, returning the declared result. |
| `color_metadata_matches` | function | Checks color metadata matches predicate, returning a boolean. |
| `missing_chroma_location_report_is_acceptable` | function | Checks missing chroma location report is acceptable predicate, returning a boolean. |
| `frame_rate_validation_error` | function | Provides frame rate validation error behavior, returning the declared result. |
| `frame_rates_match` | function | Provides frame rates match behavior, returning the declared result. |
| `frame_rate_label` | function | Provides frame rate label behavior, returning the declared result. |
| `fps_matches` | function | Checks fps matches predicate, returning a boolean. |
| `commit_output` | function | Moves one validated temp output into place and retires the original under the keep-original policy, returning success or a user-facing error; an output name that is the source itself, including a differently-cased name for the same file, goes through the recovery rename. |
| `is_same_existing_file` | function | Decides whether two paths name the very same existing file, returning true only when both exist, neither is a symbolic link, and they share device and inode with one link (Unix) or resolve to one full path (elsewhere). |
