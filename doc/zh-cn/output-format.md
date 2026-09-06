# 输出格式与容器选择

`OutputFormat` 策略是类型化的容器策略边界。MyVidComp 默认输出 AV1 MP4，除非用户选择保守的 MKV 回退，否则绝不离开 MP4 路径。

## 两种格式

| 格式 | 线值 | 行为 |
|---|---|---|
| 严格 MP4（默认） | `mp4` | 映射的非主流水流位于保守 MP4 复制白名单之外时，以具体原因跳过该文件。 |
| MP4 + MKV 回退 | `mkv-fallback` | 对这类文件，仅当每个映射的非主流水流类型都保守地兼容 Matroska 复制时，将输出容器切换为 MKV。 |

选择发生在**已确认的 QuickTime 章节承载轨被排除出映射流集合之后**，因此章节承载轨绝不触发 MKV 回退——此类文件保持 MP4。

## 保守 MP4 复制白名单

主视频流被重新编码为 AV1，因此不受此白名单约束。其他每个映射流必须可复制兼容：

| 流类型 | 白名单编解码器 |
|---|---|
| 视频（非主） | `av1`、`h264`、`hevc`、`mpeg4`、`mjpeg`、`jpeg2000` |
| 音频 | `aac`、`mp3`、`alac`、`ac3`、`eac3`、`flac`、`opus` |
| 字幕 | `mov_text` |

这是刻意保守的静态基线。白名单之外的流——包括 ASS/SSA 和 subrip 字幕、DTS/TrueHD/PCM 音频、`timed_id3` 数据、附件和未知流类型——产生带流索引、编解码器和（已知时）handler 名的具体跳过原因。

## MKV 回退规则

`mkv-fallback` 不意味着"把任何东西放进 MKV"。仅当**每个**映射的非主流水流类型都保守地兼容 Matroska 复制时才选择回退，目前即：

| 流类型 | Matroska 回退 |
|---|---|
| `video` / `audio` / `subtitle` | 允许（如 ASS、subrip、DTS、TrueHD 音频） |
| `data` / `attachment` / 未知 | 绝不允许 — 具体安全跳过 |

Matroska 能力并不证明任意流可被安全复制：任意数据、附件和未知流即使启用回退也仍是具体的安全跳过。当 MP4 与 MKV 都不安全时，跳过原因先报 MP4 原因，再附 Matroska 原因。

## 跳过原因

`SkipReason::UnsafeReplacement` 携带具体详情消息：

- 不兼容流：流索引、类型、编解码器，以及可用时的 handler。
- 隔行源：探测到的 field order；MyVidComp 不做去隔行。
- 无法映射的 AV1 元数据：具体的元数据字段。
- 章节承载轨分类失败：原子级或章节记录级的问题。

## 选择入口

- CLI：`--output-format mp4|mkv-fallback`
- 配置：`output_format`（包模板中为兼容旧构建而注释）
- 嵌入 API：`RunOptions.output_format`
- GUI：本地化的输出格式选择器，持久化于 GUI 设置（schema v3+）
- 嵌入 ABI：`FfiRunOptionsV1` 的 `output_format` 字段（空值映射为 `mp4`）

所选格式通过 `output_format_selected` 事件和终端"Output format"行上报。
