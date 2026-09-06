# 章节载体解析

读取有界的 ISO BMFF 元数据，以区分章节载体和普通文本轨道。面向读者的说明见
[../chapter-carrier.md](../chapter-carrier.md)。

声明名称与用途说明直接取自源码中的 `AI-FUNC-SUMMARY` 注释，按本仓库的编写规则，这些注释以英文书写，
因此不作翻译。

| 声明 | 种类 | 用途 |
|---|---|---|
| `bmff_children` | function | Enumerates direct child boxes inside a bounded parent range, returning validated headers or a structural/read error. |
| `read_tkhd_track_id` | function | Reads a version 0 or 1 tkhd track ID within its validated box bounds, returning a positive track ID or a structural/read error. |
| `read_be_u32` | function | Reads one big-endian u32 from a bounded parser position, returning the value or a concise read error. |
| `read_be_u64` | function | Reads one big-endian u64 from a bounded parser position, returning the value or a concise read error. |
| `select_output_container` | function | Selects MP4 or conservative Matroska fallback after carrier filtering, returning a container or detailed unsupported-stream reason. |
