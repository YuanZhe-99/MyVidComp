# 校验与提交

对照源检查完成的文件，并把它放到位。面向读者的说明见
[../validation.md](../validation.md) 和 [../safety.md](../safety.md)。

| 声明 | 种类 | 用途 |
|---|---|---|
| `validate_chapter_policy` | function | 仅对确认存在章节载体的源校验输出章节，返回语义上的时间戳/标题不符细节或成功。 |
| `chapter_validation_error` | function | 比较预期与输出的章节数量、时间戳和完全一致的标题，在实际时间戳容差内返回详细的不符说明或无。 |
| `pixel_format_validation_error` | function | 提供像素格式校验错误的行为，返回声明的结果。 |
| `validate_stream_signature` | function | 校验流签名条件，返回成功、失败或测试断言结果。 |
| `stream_signature_validation_error` | function | 提供流签名校验错误的行为，返回声明的结果。 |
| `display_metadata_validation_error` | function | 提供显示元数据校验错误的行为，返回声明的结果。 |
| `display_metadata_matches` | function | 以有理数容差比较显示元数据中的宽高比，两值足够等价可通过校验时返回真。 |
| `rational_metadata_matches` | function | 以适应容器改写的小容差比较有理数元数据值，比例在观感上等价时返回真。 |
| `duration_validation_error` | function | 把转换后的时长与源比较，输出明显偏短或偏长时返回一条错误。 |
| `color_metadata_validation_error` | function | 提供色彩元数据校验错误的行为，返回声明的结果。 |
| `color_metadata_matches` | function | 检查色彩元数据是否相符，返回一个布尔值。 |
| `missing_chroma_location_report_is_acceptable` | function | 检查缺失的色度位置报告是否可以接受，返回一个布尔值。 |
| `frame_rate_validation_error` | function | 提供帧率校验错误的行为，返回声明的结果。 |
| `frame_rates_match` | function | 提供帧率是否相符的行为，返回声明的结果。 |
| `frame_rate_label` | function | 提供帧率标签的行为，返回声明的结果。 |
| `fps_matches` | function | 检查帧率是否相符，返回一个布尔值。 |
| `commit_output` | function | 执行输出提交操作，返回操作状态或结果。 |
