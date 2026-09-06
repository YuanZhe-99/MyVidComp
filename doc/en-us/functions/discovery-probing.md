# Discovery and probing

Finding candidate files and reading what is inside them. Described for readers in
[../architecture.md](../architecture.md).

| Declaration | Kind | Purpose |
|---|---|---|
| `parse_video_probe` | function | Parses video probe input, returning parsed values or errors. |
| `known_color_value` | function | Provides known color value behavior, returning the declared result. |
| `known_ratio_value` | function | Provides known ratio value behavior, returning the declared result. |
| `probe_streams` | function | Probes stream layout and chapter-carrier metadata, returning indexed streams, an empty list for unreadable media, or a tool/process error. |
| `ffprobe_failure_is_infrastructure` | function | Distinguishes ffprobe tool/process failures from ordinary unreadable media diagnostics, returning true only for abnormal exits or known runtime failures. |
| `probe_failure_message` | function | Formats a failed ffprobe exit with path and concise diagnostics, returning a terminal process error. |
| `parse_stream_probe` | function | Parses compact ffprobe stream records, retaining actual indexes and chapter-carrier metadata, returning unique indexed streams in probe order. |
| `known_probe_value` | function | Converts a non-empty ffprobe field into optional metadata, returning none for empty, N/A, or unknown values. |
| `parse_track_id` | function | Parses an ffprobe track ID in decimal or 0x-prefixed hexadecimal form, returning a positive 32-bit ID or none. |
| `probe_chapters` | function | Probes source chapter start/end/title metadata, returning parsed chapters or a tool/process error. |
| `parse_chapter_probe` | function | Parses compact ffprobe chapter records, returning complete finite timestamp/title records or a structural error. |
| `parse_compact_fields` | function | Splits one FFprobe compact record at unescaped separators and decodes C-style field values, returning key/value fields without losing escaped title or handler characters. |
| `split_compact_unescaped` | function | Splits FFprobe compact text on unescaped delimiters while retaining escape sequences for later decoding, returning encoded segments. |
| `decode_compact_escapes` | function | Decodes common FFprobe compact C escape sequences after structural splitting, returning the represented field text. |
| `retryable` | function | Creates a retryable encoder-attempt failure, returning typed failure. |
| `terminal` | function | Creates a terminal encoder-attempt failure, returning typed failure. |
| `label` | function | Returns the stable event label for a transcode plan, returning exact or adapted. |
| `output_pix_fmt` | function | Returns the planned output pixel format when adaptation is requested, returning none for exact plans. |
| `description` | function | Returns a user-facing plan description, returning exact-preservation or adaptation detail. |
| `transcode_plans` | function | Builds the ordered list of encode attempts for one file.. |
