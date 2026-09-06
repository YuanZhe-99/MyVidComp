# 流策略

决定映射哪些流，以及哪种容器能装下它们。面向读者的说明见
[../output-format.md](../output-format.md)。

| 声明 | 种类 | 用途 |
|---|---|---|
| `is_chapter_carrier_candidate` | function | 检查一条流是否具备可能的 QuickTime 章节载体在 ffprobe 一侧的全部特征，仅当是带数字轨道 ID 的 data/bin_data FourCC 文本流时返回真。 |
| `chapters_are_placeholder_or_empty` | function | 把没有章节、或只有一个无名且覆盖整个时长的章节判定为空占位，在实际封装器时间戳容差内返回真。 |
| `parse_bmff_chapter_targets` | function | 读取 ISO BMFF 结构，收集被 tref/chap 引用明确指向的轨道 ID。 |
| `matroska_incompatible_stream_reason` | function | 找出一条被映射的非主要流，其类型在保守规则下无法复制进 Matroska，返回实际的索引/类型/编码细节或无。 |
| `mp4_incompatible_stream_reason` | function | 找出第一条无法复制进 MP4 容器的非主要流，返回一条细节说明或无。 |
| `is_mp4_copy_compatible_stream` | function | 检查一条非主要流能否复制进 MP4 目标，流类型/编码受支持时返回真。 |
| `is_mp4_video_codec` | function | 检查被复制的非主要视频编码是否兼容 MP4，返回一个布尔值。 |
| `is_mp4_audio_codec` | function | 检查是否为 MP4 音频编码，返回一个布尔值。 |
| `is_mp4_subtitle_codec` | function | 检查被复制的字幕编码是否兼容 MP4，返回一个布尔值。 |
| `interlaced_source_reason` | function | 当源明确为隔行时返回一条跳过原因，逐行或场序未知时返回无。 |
| `estimate_av1_crf` | function | 构造或推导 AV1 CRF 估算数据，返回算得的值。 |
