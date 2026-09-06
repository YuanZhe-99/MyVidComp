# 编码

三种输出编码之间的差异：各由哪些编码器产出、它们的画质控制是什么形态，以及色彩元数据如何携带。面向读者的说明见
[../codecs.md](../codecs.md)。

声明名称与用途说明直接取自源码中的 `AI-FUNC-SUMMARY` 注释，按本仓库的编写规则，这些注释以英文书写，
因此不作翻译。

| 声明 | 种类 | 用途 |
|---|---|---|
| `encoder_candidates` | function | Lists the encoders that can produce one codec, best first, returning the ordered candidate table. |
| `canonical_encoder_name` | function | Expands a short encoder alias into the full name for one codec, returning the canonical encoder name. |
| `vvc_alias` | function | Expands a short encoder alias for the VVC codec, returning the canonical encoder name. |
| `quality_style` | function | Reports which quality knob an encoder family exposes, returning the style. |
| `quality_range` | function | Reports the searchable quality range for an encoder family, returning the range and step. |
| `convert_quality` | function | Converts a quality value chosen for AV1 into the equivalent for another codec, returning the converted value. |
| `codec_bitrate_multiplier` | function | Reports how much of the source bitrate a codec needs for comparable quality, returning a multiplier. |
| `required_pixel_format` | function | Reports the only pixel format an encoder accepts, when it accepts just one, returning the format or none. |
| `metadata_filter` | function | Reports the metadata bitstream filter for one codec, returning the filter description or none when the codec has no usable filter. |
| `chroma_location_value` | function | Maps an ffprobe chroma location onto the value one codec can record, returning the value or none when the codec cannot express it. |
| `color_range_value` | function | Maps a colour range onto the value one codec records, returning the value or none. |
| `codec_accepts_source` | function | Reports whether a source can be encoded to one codec without changing its pixel format, returning true when no conversion is needed. |
