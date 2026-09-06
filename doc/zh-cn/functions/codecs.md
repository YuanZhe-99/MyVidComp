# 编码

三种输出编码之间的差异：各由哪些编码器产出、它们的画质控制是什么形态，以及色彩元数据如何携带。面向读者的说明见
[../codecs.md](../codecs.md)。

| 声明 | 种类 | 用途 |
|---|---|---|
| `encoder_candidates` | function | 列出能产出某一种编码的全部编码器，最佳者在前，返回排好序的候选表。 |
| `canonical_encoder_name` | function | 针对某一种编码，把编码器简称展开为全名，返回规范的编码器名称。 |
| `vvc_alias` | function | 展开 VVC 编码的编码器简称，返回规范的编码器名称。 |
| `quality_style` | function | 报告某一族编码器提供的是哪一种画质旋钮，返回该形态。 |
| `quality_range` | function | 报告某一族编码器可供搜索的画质范围，返回范围与步长。 |
| `convert_quality` | function | 把为 AV1 选定的画质值换算成另一种编码的等效值，返回换算后的值。 |
| `codec_bitrate_multiplier` | function | 报告某一种编码要达到相当画质需要源码率的多大比例，返回一个倍数。 |
| `required_pixel_format` | function | 当某个编码器只接受一种像素格式时报告该格式，返回该格式，否则返回无。 |
| `metadata_filter` | function | 报告某一种编码所用的元数据比特流过滤器，返回该过滤器的描述，编码没有可用过滤器时返回无。 |
| `chroma_location_value` | function | 把 ffprobe 给出的色度位置映射到某一种编码能够记录的值，返回该值，编码无法表达时返回无。 |
| `color_range_value` | function | 把色彩范围映射到某一种编码所记录的值，返回该值，否则返回无。 |
| `codec_accepts_source` | function | 报告某个源能否在不改变像素格式的前提下编码为某一种编码，无需转换时返回真。 |
