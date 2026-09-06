# 校验与提交

对照源检查完成的文件，并把它放到位。面向读者的说明见
[../validation.md](../validation.md) 和 [../safety.md](../safety.md)。

声明名称与用途说明直接取自源码中的 `AI-FUNC-SUMMARY` 注释，按本仓库的编写规则，这些注释以英文书写，
因此不作翻译。

| 声明 | 种类 | 用途 |
|---|---|---|
| `validate_chapter_policy` | function | Validates output chapters only for confirmed chapter-carrier sources, returning semantic timestamp/title mismatch detail or success. |
| `chapter_validation_error` | function | Compares expected and output chapter count, timestamps, and exact titles, returning detailed mismatch with practical timestamp tolerance or none. |
| `pixel_format_validation_error` | function | Provides pixel format validation error behavior, returning the declared result. |
| `validate_stream_signature` | function | Validates validate stream signature conditions, returning success, failure, or test assertion result. |
| `stream_signature_validation_error` | function | Provides stream signature validation error behavior, returning the declared result. |
| `display_metadata_validation_error` | function | Provides display metadata validation error behavior, returning the declared result. |
| `display_metadata_matches` | function | Compares display metadata with rational tolerance for aspect ratios, returning true when values are equivalent enough for validation. |
| `rational_metadata_matches` | function | Compares rational metadata values with a small tolerance for container rewrites, returning true when ratios are visually equivalent. |
| `duration_validation_error` | function | Compares the converted duration against the source, returning an error when the output is noticeably shorter or longer. |
| `color_metadata_validation_error` | function | Provides color metadata validation error behavior, returning the declared result. |
| `color_metadata_matches` | function | Checks color metadata matches predicate, returning a boolean. |
| `missing_chroma_location_report_is_acceptable` | function | Checks missing chroma location report is acceptable predicate, returning a boolean. |
| `frame_rate_validation_error` | function | Provides frame rate validation error behavior, returning the declared result. |
| `frame_rates_match` | function | Provides frame rates match behavior, returning the declared result. |
| `frame_rate_label` | function | Provides frame rate label behavior, returning the declared result. |
| `fps_matches` | function | Checks fps matches predicate, returning a boolean. |
| `commit_output` | function | Performs commit output operation, returning operation status or result. |
