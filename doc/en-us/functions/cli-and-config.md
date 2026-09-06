# Command line and config

Parsing the command line and the settings file, and turning both into run options. Described for
readers in [../options.md](../options.md).

| Declaration | Kind | Purpose |
|---|---|---|
| `parse` | function | Parses parse input, returning parsed values or errors. |
| `parse_with_default_config` | function | Parses with default config input, returning parsed values or errors. |
| `from` | function | Converts parsed CLI options into embedded run options, returning public run options. |
| `usage` | function | Builds or derives usage data, returning the computed value. |
| `version_label` | function | Builds or derives version label data, returning the computed value. |
| `missing_target_message` | function | Checks missing target message predicate, returning a boolean. |
| `count_limit_info` | function | Provides count limit info behavior, returning the declared result. |
| `count_candidate_info` | function | Provides count candidate info behavior, returning the declared result. |
| `conversion_limit` | function | Provides conversion limit behavior, returning the declared result. |
| `conversion_limit_reached` | function | Provides conversion limit reached behavior, returning the declared result. |
| `read` | function | Provides read behavior, returning the declared result. |
| `parse_config_yaml` | function | Parses config yaml input, returning parsed values or errors. |
| `strip_yaml_comment` | function | Provides strip yaml comment behavior, returning the declared result. |
| `optional_yaml_value` | function | Reads a config value that may be left blank, returning the scalar or an empty string. |
| `parse_yaml_scalar` | function | Parses yaml scalar input, returning parsed values or errors. |
| `unescape_double_quoted_yaml_scalar` | function | Provides unescape double quoted yaml scalar behavior, returning the declared result. |
| `parse_yaml_bool` | function | Parses yaml bool input, returning parsed values or errors. |
| `normalize_encoder_choice` | function | Builds or derives normalize encoder choice data, returning the computed value. |
| `from` | function | Constructs the associated value, returning a new instance. |
| `map_value` | function | Returns FFmpeg's chapter input selector for this source policy, returning 0 only for meaningful confirmed chapters and -1 otherwise. |
| `new` | function | Constructs the associated value, returning a new instance. |
| `output_path` | function | Provides output path behavior, returning the declared result. |
| `extension` | function | Returns the file extension used for this container, returning static extension without dot. |
| `ffmpeg_format` | function | Returns the FFmpeg format flag for this container, returning static format name. |
| `from_options` | function | Extracts the per-file settings from a full set of run options, returning the policy. |
| `new` | function | Constructs the associated value, returning a new instance or a skip reason. |
| `skip_reason_label` | function | Formats an internal skip reason for user and GUI reporting, returning a human-readable message. |
| `is_gpu` | function | Checks whether an encoder is GPU-backed, returning true for hardware and Vulkan backends. |
