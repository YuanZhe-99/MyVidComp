# Options

Every user-facing option: its wire value, its label, and the parsing that turns text into a
setting. Described for readers in [../options.md](../options.md).

| Declaration | Kind | Purpose |
|---|---|---|
| `as_str` | function | Maps a target codec to its stable wire value, returning static label. |
| `label` | function | Maps a target codec to its human-readable log label, returning static label. |
| `ffprobe_name` | function | Reports the ffprobe codec name the output must carry, returning static codec name. |
| `mp4_tag` | function | Reports the MP4 codec tag this codec needs for broad player support, returning a tag or none. |
| `parse_target_codec` | function | Parses a target-codec wire value, returning the codec or a user-facing error. |
| `normalize_target_codec_choice` | function | Normalizes an optional target-codec value, returning none for blank/auto/default or a parsed codec. |
| `as_str` | function | Maps a preservation policy to its stable wire value, returning static label. |
| `label` | function | Maps a preservation policy to its human-readable log label, returning static label. |
| `allows_deviations` | function | Reports whether recoverable differences may be accepted, returning true in flexible mode. |
| `parse_preservation` | function | Parses a preservation wire value, returning the policy or a user-facing error. |
| `normalize_preservation_choice` | function | Normalizes an optional preservation value, returning none for blank/auto/default or a parsed policy. |
| `as_str` | function | Maps a quality mode to its stable wire value, returning static label. |
| `label` | function | Maps a quality mode to its human-readable log label, returning static label. |
| `parse_quality_mode` | function | Parses a quality-mode wire value, returning the mode or a user-facing error. |
| `normalize_quality_mode_choice` | function | Normalizes an optional quality-mode value, returning none for blank/auto/default or a parsed mode. |
| `as_str` | function | Maps a quality check to its stable wire value, returning static label. |
| `label` | function | Maps a quality check to its human-readable log label, returning static label. |
| `subsample` | function | Reports how many frames to skip between measurements, returning the libvmaf n_subsample value. |
| `parse_quality_check` | function | Parses a quality-check wire value, returning the setting or a user-facing error. |
| `normalize_quality_check_choice` | function | Normalizes an optional quality-check value, returning none for blank/auto/default or a parsed setting. |
| `as_str` | function | Maps an encoder preference to its stable wire value, returning static label. |
| `label` | function | Maps an encoder preference to its human-readable log label, returning static label. |
| `parse_encoder_preference` | function | Parses an encoder-preference wire value, including the retired conversion-mode names, returning the preference or a user-facing error. |
| `normalize_encoder_preference_choice` | function | Normalizes an optional encoder-preference value, returning none for blank/default or a parsed preference. |
| `validate_quality_target` | function | Validates a quality target in hundredths of a VMAF point, returning the target or a user-facing error. |
| `parse_quality_target` | function | Parses a decimal quality target such as "95" or "94.5" into hundredths, returning the value or a user-facing error. |
| `format_quality` | function | Formats a hundredths quality value for display, returning a short decimal string. |
| `is_unset_choice` | function | Checks whether an option string means "not set", returning true for blank, auto, or default. |
| `parse_quality_points` | function | Parses a decimal VMAF point value such as "2" or "1.5" into hundredths, returning the value or a user-facing error. |
| `as_str` | function | Maps a quality metric to its stable wire value, returning static label. |
| `label` | function | Maps a quality metric to its human-readable log label, returning static label. |
| `parse_quality_metric` | function | Parses a quality-metric wire value, returning the metric or a user-facing error. |
| `normalize_quality_metric_choice` | function | Normalizes an optional quality-metric value, returning none for blank/auto/default or a parsed metric. |
| `as_str` | function | Maps an encoder preset to its stable wire value, returning static label. |
| `label` | function | Maps an encoder preset to its human-readable log label, returning static label. |
| `parse_encoder_preset` | function | Parses an encoder-preset wire value, returning the preset or a user-facing error. |
| `normalize_encoder_preset_choice` | function | Normalizes an optional encoder-preset value, returning none for blank/auto/default or a parsed preset. |
| `as_str` | function | Maps an audio policy to its stable wire value, returning static label. |
| `parse_audio_policy` | function | Parses an audio-policy wire value, returning the policy or a user-facing error. |
| `normalize_audio_policy_choice` | function | Normalizes an optional audio-policy value, returning none for blank/default or a parsed policy. |
| `as_str` | function | Maps a deinterlace policy to its stable wire value, returning static label. |
| `parse_deinterlace_policy` | function | Parses a deinterlace-policy wire value, returning the policy or a user-facing error. |
| `normalize_deinterlace_policy_choice` | function | Normalizes an optional deinterlace-policy value, returning none for blank/default or a parsed policy. |
| `as_str` | function | Maps a size-guard policy to its stable wire value, returning static label. |
| `parse_size_guard` | function | Parses a size-guard wire value, returning the policy or a user-facing error. |
| `normalize_size_guard_choice` | function | Normalizes an optional size-guard value, returning none for blank/default or a parsed policy. |
| `as_str` | function | Maps a hardware acceleration backend to its stable FFmpeg method name, returning static label. |
| `as_str` | function | Maps a decoder preference to its stable wire value, returning static label. |
| `label` | function | Formats a decoder preference for human-readable status logs, returning a string label. |
| `allows_hardware` | function | Reports whether a decoder preference permits hardware decoding, returning false for CPU-only mode. |
| `parse_decoder_preference` | function | Parses a decoder-preference wire value (`auto`, `gpu`, `cpu`, or named method), returning the preference or a user-facing error. |
| `normalize_decoder_preference_choice` | function | Normalizes an optional decoder-preference value, returning none for blank/default or a parsed preference. |
