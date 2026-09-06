# 章节载体解析

读取有界的 ISO BMFF 元数据，以区分章节载体和普通文本轨道。面向读者的说明见
[../chapter-carrier.md](../chapter-carrier.md)。

| 声明 | 种类 | 用途 |
|---|---|---|
| `bmff_children` | function | 在有界的父级范围内枚举直接子 box，返回校验过的头部或一条结构/读取错误。 |
| `read_tkhd_track_id` | function | 在校验过的 box 边界内读取版本 0 或 1 的 tkhd 轨道 ID，返回一个正的轨道 ID 或一条结构/读取错误。 |
| `read_be_u32` | function | 从有界的解析位置读取一个大端 u32，返回该值或一条简明的读取错误。 |
| `read_be_u64` | function | 从有界的解析位置读取一个大端 u64，返回该值或一条简明的读取错误。 |
| `select_output_container` | function | 在过滤掉载体轨道后选择 MP4 或保守的 Matroska 回退，返回一种容器或详细的不支持流原因。 |
