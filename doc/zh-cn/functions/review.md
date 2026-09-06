# 复核

保留在原文件旁边、等待用户决定的转换。面向读者的说明见
[../review.md](../review.md)。

| 声明 | 种类 | 用途 |
|---|---|---|
| `new` | function | 构造一条偏差记录，返回该值。 |
| `size_percent` | function | 以相对原文件的百分比报告体积变化，返回该百分比，原文件为空时返回无。 |
| `parse_review_decision` | function | 解析复核决定的传输值，返回该决定或一条面向用户的错误。 |
| `review_output_path` | function | 为一个转换结果构造待复核的输出路径，返回带有复核后缀的路径。 |
| `review_sidecar_path` | function | 为一个待复核文件构造伴随文件的路径，返回该伴随文件路径。 |
| `is_review_artifact` | function | 检查一个文件名是否标记着待复核的输出或它的伴随文件，两者之一即返回真。 |
| `has_pending_review` | function | 检查一个源文件是否已经有一次转换在等待决定，旁边存在复核文件时返回真。 |
| `write_sidecar` | function | 写出描述一项待复核内容的伴随文件，返回成功或磁盘错误。 |
| `sidecar_json` | function | 把一项复核条目序列化为伴随文件的 JSON 文档，返回该文本。 |
| `review_list_json` | function | 为 FFI 序列化一组复核条目，返回一个 JSON 数组文档。 |
| `list_reviews` | function | 找出某个文件夹树下的全部待复核内容，返回按原文件路径排序的条目。 |
| `collect_reviews` | function | 遍历一层目录收集复核伴随文件，无返回值。 |
| `read_sidecar` | function | 把一个伴随文件读成一项复核条目，返回该条目，记录不可读或其文件已不存在时返回无。 |
| `resolve_review` | function | 对一项待复核内容执行决定，返回成功或一条面向用户的错误。 |
| `rename_into_place` | function | 把复核文件改名到目标位置且不覆盖任何东西，返回成功或一条面向用户的错误。 |
| `kept_both_path` | function | 构造用户选择两份都留时转换结果所用的名称，返回一个不与原文件冲突的路径。 |
| `review_summary_line` | function | 为终端输出描述一项复核条目，返回一行摘要。 |
| `field` | function | 从伴随文件文档中读取一个字符串字段，返回反转义后的值或无。 |
| `number` | function | 从伴随文件文档中读取一个数值字段，返回该值，字段缺失或为 null 时返回无。 |
| `deviations` | function | 从伴随文件文档中读取偏差列表，返回记录下来的全部变化。 |
