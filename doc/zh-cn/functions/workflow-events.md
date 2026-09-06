# 工作流与事件

运行过程本身、它报告的事件，以及渲染这些事件的进度界面。面向读者的说明见
[../architecture.md](../architecture.md)。

| 声明 | 种类 | 用途 |
|---|---|---|
| `process_command` | function | 创建一个在 Windows 上不弹出控制台窗口的子进程命令，返回一个可继续配置的命令。 |
| `as_str` | function | 把输出格式映射到它稳定的传输值，返回静态标签。 |
| `label` | function | 把输出格式映射到便于人读的日志标签，返回静态标签。 |
| `parse_output_format` | function | 解析输出格式的传输值，返回该格式或一条面向用户的错误。 |
| `normalize_output_format_choice` | function | 归一化可选的输出格式值，空白、auto 或 default 时返回无，否则返回解析出的格式。 |
| `default` | function | 构造默认的嵌入运行选项，返回与命令行默认值一致的选项。 |
| `review_threshold` | function | 报告低于哪个分数就必须复核，以百分之一单位返回该阈值。 |
| `conversion_attempts` | function | 统计摘要所代表的转换尝试次数，返回已转换与失败文件数之和。 |
| `on_event` | function | 接收一条结构化的 MyVidComp 工作流事件，无返回值。 |
| `on_event` | function | 把一条 MyVidComp 工作流事件转发到闭包接收端，无返回值。 |
| `default` | function | 构造一个未取消的令牌，返回该令牌。 |
| `new` | function | 构造一个未取消的令牌，返回该令牌。 |
| `request_stop` | function | 请求在当前文件完成后优雅取消，无返回值。 |
| `is_stop_requested` | function | 检查是否已请求优雅取消，返回一个布尔值。 |
| `on_event` | function | 丢弃一条结构化的 MyVidComp 工作流事件，无返回值。 |
| `event_json` | function | 为 FFI 回调把一条 MyVidComp 事件序列化为 JSON，返回紧凑的 JSON 文本。 |
| `candidate_json` | function | 把一条预览模式的候选项序列化为 JSON，返回紧凑的 JSON 文本。 |
| `summary_json` | function | 把一次运行的摘要序列化为 JSON，返回紧凑的 JSON 文本。 |
| `json_path` | function | 把一个路径值序列化为 JSON 字符串，返回转义后的 JSON 文本。 |
| `json_string` | function | 把一个字符串值序列化为 JSON，返回转义后的 JSON 字符串文本。 |
| `json_optional_string` | function | 把一个可选字符串序列化为 JSON，返回字符串或 null 的 JSON 文本。 |
| `json_optional_u32` | function | 为 JSON 输出序列化一个可选的 32 位计数，返回该数字或 null。 |
| `json_optional_usize` | function | 把一个可选的 usize 序列化为 JSON，返回数字或 null 的 JSON 文本。 |
| `json_optional_u64` | function | 把一个可选的 u64 序列化为 JSON，返回数字或 null 的 JSON 文本。 |
| `json_f64` | function | 把一个有限浮点数序列化为 JSON，返回数字字符串，非有限值有回退写法。 |
| `log_level_label` | function | 把日志级别映射到它的 JSON 标签，返回静态标签。 |
| `file_status_label` | function | 把文件状态映射到它的 JSON 标签，返回静态标签。 |
| `main_entry` | function | 运行 MyVidComp 的命令行工作流程，按常规命令行控制流返回进程结果。 |
| `run` | function | 运行 MyVidComp 的命令行工作流程，按常规命令行控制流返回进程结果。 |
| `print_reviews` | function | 打印某个文件夹中每一项等待决定的转换，返回成功。 |
| `resolve_all_reviews` | function | 对某个文件夹中全部待复核内容执行同一个决定，返回成功或第一个失败。 |
| `run_with_events` | function | 从嵌入调用方运行 MyVidComp，使用结构化事件而非终端界面。 |
| `run_workflow` | function | 为终端命令行和嵌入调用方执行共用的 MyVidComp 工作流程。 |
| `validate_run_options` | function | 在工作流程产生副作用之前校验嵌入运行选项，返回成功或一条面向用户的错误。 |
| `move_validated_output` | function | 执行移动已校验输出的操作，返回操作状态或结果。 |
| `move_validated_output_with_ui` | function | 执行带界面反馈的移动已校验输出操作，返回操作状态或结果。 |
| `move_validated_output_with_progress` | function | 执行带进度的移动已校验输出操作，返回操作状态或结果。 |
| `copy_file_to_new_path` | function | 执行把文件复制到新路径的操作，返回操作状态或结果。 |
| `copy_file_to_new_path_with_progress` | function | 执行带进度的把文件复制到新路径操作，返回操作状态或结果。 |
| `final_commit_temp_path` | function | 提供最终提交临时路径的行为，返回声明的结果。 |
| `metadata_repair_temp_path` | function | 提供元数据修复临时路径的行为，返回声明的结果。 |
| `metadata_backup_temp_path` | function | 提供元数据备份临时路径的行为，返回声明的结果。 |
| `apply` | function | 提供应用的行为，返回声明的结果。 |
| `parse_progress_line` | function | 解析进度行输入，返回解析出的值或错误。 |
| `parse_hms` | function | 解析时分秒输入，返回解析出的值或错误。 |
| `start` | function | 启动可选的标准输入监听，以接收优雅停止请求。 |
| `prompt_active_flag` | function | 克隆“提示进行中”标志以协调进度渲染，返回共享的标志句柄。 |
| `wait_until_prompt_inactive` | function | 等待正在进行的退出确认提示结束，界面输出安全时返回。 |
| `graceful_exit_input_loop` | function | 读取标准输入命令并记录一次确认过的优雅退出请求。 |
| `new` | function | 构造进度界面状态，返回一个新实例。 |
| `set_codec` | function | 记录本次运行产出的编码，无返回值。 |
| `set_quality_available` | function | 记录本次运行能否测量画质，无返回值。 |
| `emit_quality_measured` | function | 报告一次完成的画质测量，无返回值。 |
| `emit_quality_search` | function | 报告画质搜索的一步，无返回值。 |
| `emit_review_pending` | function | 报告一次转换正在等待“留哪一份”的决定，无返回值。 |
| `emit_capability_missing` | function | 报告当前 ffmpeg 构建缺少的一项能力，无返回值。 |
| `prompt_active` | function | 检查退出确认是否正占用终端，进度渲染应当暂停时返回真。 |
| `wait_until_prompt_inactive` | function | 等到退出确认不再占用终端，结构化界面输出可以安全打印时返回。 |
| `start_file` | function | 提供开始处理一个文件的行为，返回声明的结果。 |
| `start_attempt` | function | 为当前文件开始一次编码器尝试，无返回值。 |
| `render_file_progress` | function | 提供渲染文件进度的行为，返回声明的结果。 |
| `render_copy_progress` | function | 提供渲染复制进度的行为，返回声明的结果。 |
| `render_stage` | function | 提供渲染阶段的行为，返回声明的结果。 |
| `finish_file` | function | 提供结束一个文件的行为，返回声明的结果。 |
| `emit_file_skipped` | function | 发出一条文件被跳过的事件，无返回值。 |
| `emit_file_finished` | function | 发出一条文件已完成的事件，无返回值。 |
| `emit_stop_requested` | function | 发出一条已请求停止的事件，无返回值。 |
| `emit_summary` | function | 发出最终的摘要事件，无返回值。 |
| `emit_log` | function | 发出一条不做终端渲染的日志事件，无返回值。 |
| `log_info` | function | 发出一条信息级日志以及可选的终端行，无返回值。 |
| `log_warning` | function | 发出一条警告级日志以及可选的终端行，无返回值。 |
| `log_error` | function | 发出一条错误级日志以及可选的终端行，无返回值。 |
| `log` | function | 把一条日志消息发往终端模式和结构化事件接收端，无返回值。 |
| `total_label` | function | 提供总计标签的行为，返回声明的结果。 |
| `quality_label` | function | 提供画质标签的行为，返回声明的结果。 |
| `conversion_size_label` | function | 格式化一次转换前后的字节数，返回一个简明的体积变化标签。 |
| `print_summary` | function | 提供打印摘要的行为，返回声明的结果。 |
| `print_dry_run` | function | 提供打印预览结果的行为，返回声明的结果。 |
| `source_stream_policy` | function | 决定映射哪些流以及章节如何处理，不臆断任意文本/数据轨道可以丢弃。 |
