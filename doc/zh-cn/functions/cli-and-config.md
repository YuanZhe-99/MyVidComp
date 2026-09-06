# 命令行与配置

解析命令行和设置文件，并把两者变成运行选项。面向读者的说明见
[../options.md](../options.md)。

| 声明 | 种类 | 用途 |
|---|---|---|
| `parse` | function | 解析输入，返回解析出的值或错误。 |
| `parse_with_default_config` | function | 结合默认配置文件解析输入，返回解析出的值或错误。 |
| `from` | function | 把解析出的命令行选项转换为嵌入用的运行选项，返回公开的运行选项。 |
| `usage` | function | 构造或推导用法文本，返回算得的值。 |
| `version_label` | function | 构造或推导版本标签，返回算得的值。 |
| `missing_target_message` | function | 检查是否缺少目标文件夹提示，返回一个布尔值。 |
| `count_limit_info` | function | 提供数量上限提示的行为，返回声明的结果。 |
| `count_candidate_info` | function | 提供候选数量提示的行为，返回声明的结果。 |
| `conversion_limit` | function | 提供转换数量上限的行为，返回声明的结果。 |
| `conversion_limit_reached` | function | 提供是否已达转换数量上限的行为，返回声明的结果。 |
| `read` | function | 提供读取的行为，返回声明的结果。 |
| `parse_config_yaml` | function | 解析 YAML 配置输入，返回解析出的值或错误。 |
| `strip_yaml_comment` | function | 提供去除 YAML 注释的行为，返回声明的结果。 |
| `optional_yaml_value` | function | 读取一个允许留空的配置值，返回该标量或一个空字符串。 |
| `parse_yaml_scalar` | function | 解析 YAML 标量输入，返回解析出的值或错误。 |
| `unescape_double_quoted_yaml_scalar` | function | 提供双引号 YAML 标量反转义的行为，返回声明的结果。 |
| `parse_yaml_bool` | function | 解析 YAML 布尔输入，返回解析出的值或错误。 |
| `normalize_encoder_choice` | function | 构造或推导归一化后的编码器选择，返回算得的值。 |
| `from` | function | 转换为对应的值，返回一个新实例。 |
| `map_value` | function | 返回本源章节策略对应的 FFmpeg 章节输入选择器，只有确认存在有意义的章节时返回 0，否则返回 -1。 |
| `new` | function | 构造对应的值，返回一个新实例。 |
| `output_path` | function | 提供输出路径的行为，返回声明的结果。 |
| `extension` | function | 返回本容器所用的文件扩展名，返回不带点的静态扩展名。 |
| `ffmpeg_format` | function | 返回本容器对应的 FFmpeg 格式标志，返回静态格式名称。 |
| `from_options` | function | 从整套运行选项中提取逐文件的设置，返回该策略。 |
| `new` | function | 构造对应的值，返回一个新实例或一条跳过原因。 |
| `skip_reason_label` | function | 把内部的跳过原因格式化以便向用户和界面报告，返回一条便于人读的消息。 |
| `is_gpu` | function | 检查一个编码器是否由 GPU 支撑，硬件和 Vulkan 后端返回真。 |
