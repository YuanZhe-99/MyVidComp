# 发现与探测

找出候选文件并读取它们的内容。面向读者的说明见
[../architecture.md](../architecture.md)。

| 声明 | 种类 | 用途 |
|---|---|---|
| `parse_video_probe` | function | 解析视频探测输入，返回解析出的值或错误。 |
| `known_color_value` | function | 提供已知色彩值的行为，返回声明的结果。 |
| `known_ratio_value` | function | 提供已知比例值的行为，返回声明的结果。 |
| `probe_streams` | function | 探测流布局与章节载体元数据，返回带索引的流；媒体不可读时返回空列表；工具或进程出错时返回错误。 |
| `ffprobe_failure_is_infrastructure` | function | 把 ffprobe 的工具/进程故障与普通的媒体不可读诊断区分开，仅在异常退出或已知运行时故障时返回真。 |
| `probe_failure_message` | function | 把一次失败的 ffprobe 退出连同路径和简明诊断一起格式化，返回一条终止性的进程错误。 |
| `parse_stream_probe` | function | 解析紧凑格式的 ffprobe 流记录，保留实际索引和章节载体元数据，按探测顺序返回不重复的带索引流。 |
| `known_probe_value` | function | 把非空的 ffprobe 字段转换为可选元数据，遇到空值、N/A 或 unknown 时返回无。 |
| `parse_track_id` | function | 解析十进制或 0x 前缀十六进制形式的 ffprobe 轨道 ID，返回一个正的 32 位 ID 或无。 |
| `probe_chapters` | function | 探测源的章节起止与标题元数据，返回解析出的章节或一条工具/进程错误。 |
| `parse_chapter_probe` | function | 解析紧凑格式的 ffprobe 章节记录，返回时间戳与标题完整有限的记录或一条结构错误。 |
| `parse_compact_fields` | function | 在未转义的分隔符处切分一条 FFprobe 紧凑记录并解码 C 风格字段值，返回键值字段且不丢失标题或 handler 中被转义的字符。 |
| `split_compact_unescaped` | function | 在未转义的分隔符处切分 FFprobe 紧凑文本，同时保留转义序列以便随后解码，返回仍带编码的片段。 |
| `decode_compact_escapes` | function | 在结构切分之后解码 FFprobe 紧凑格式常见的 C 转义序列，返回字段所表示的文本。 |
| `retryable` | function | 创建一次可重试的编码尝试失败，返回类型化的失败。 |
| `terminal` | function | 创建一次终止性的编码尝试失败，返回类型化的失败。 |
| `label` | function | 返回转码方案稳定的事件标签，取值为精确或适配。 |
| `output_pix_fmt` | function | 在需要适配时返回计划的输出像素格式，精确方案返回无。 |
| `description` | function | 返回面向用户的方案说明，内容为精确保真或适配细节。 |
| `transcode_plans` | function | 为一个文件构造有序的编码尝试列表。 |
