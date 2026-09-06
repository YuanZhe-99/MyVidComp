# 编码器与画质估算

找出真正可用的编码器、为一个文件排定尝试顺序，以及在不做试编码的情况下估算画质设置。面向读者的说明见
[../encoders.md](../encoders.md)。

| 声明 | 种类 | 用途 |
|---|---|---|
| `detect_with_events` | function | 检测全部可用的 AV1 编码器并报告运行时探测诊断，返回排好序的选择或一条错误。 |
| `preferred` | function | 返回首选的精确编码器候选，即 GPU 优先排序中的第一个候选。 |
| `detect_requested_encoder` | function | 提供检测用户指定编码器的行为，返回声明的结果。 |
| `encoder_from_name` | function | 提供按名称取得编码器的行为，返回声明的结果。 |
| `supported_encoder_names` | function | 提供受支持编码器名称的行为，返回声明的结果。 |
| `detect_encoder_from_listing` | function | 提供从编码器清单中检测编码器的行为，返回声明的结果。 |
| `detect_encoder_candidates_from_listing` | function | 把对外公布的编码器清单筛选为 GPU 优先的候选，按确定的探测顺序返回候选。 |
| `check_encoder_runtime` | function | 校验编码器的运行时条件，返回成功、失败或测试断言结果。 |
| `encoder_runtime_check_args` | function | 提供编码器运行时检查参数的行为，返回声明的结果。 |
| `encoder_runtime_quality_args` | function | 提供编码器运行时画质参数的行为，返回声明的结果。 |
| `first_error_line` | function | 挑出 ffmpeg 输出中真正解释了失败原因的那一行，返回该行；没有任何一行像错误时返回最后一行。 |
| `is_ffmpeg_noise` | function | 检查 ffmpeg 输出的某一行是否只是例行絮语而非值得展示的消息，横幅、进度和流描述返回真。 |
| `line_looks_like_an_error` | function | 检查 ffmpeg 输出的某一行读起来是否像一次失败，指明了错误时返回真。 |
| `encoder_list_contains` | function | 提供编码器清单包含判断的行为，返回声明的结果。 |
| `ensure_binary` | function | 校验可执行文件是否就绪，返回成功、失败或测试断言结果。 |
| `ensure_tmp_dir` | function | 校验临时目录是否就绪，返回成功、失败或测试断言结果。 |
| `default_runtime_binary` | function | 提供默认运行时可执行文件的行为，返回声明的结果。 |
| `bundled_binary_candidates` | function | 提供随包可执行文件候选的行为，返回声明的结果。 |
| `push_binary_candidates` | function | 提供追加可执行文件候选的行为，返回声明的结果。 |
| `bundled_binary_names` | function | 提供随包可执行文件名称的行为，返回声明的结果。 |
| `platform_binary_name` | function | 提供各平台可执行文件名称的行为，返回声明的结果。 |
| `discover_files` | function | 发现或探测待处理文件，返回收集到的元数据。 |
| `is_candidate_video_path` | function | 检查是否为候选视频路径，返回一个布尔值。 |
| `is_candidate_video_extension` | function | 检查是否为候选视频扩展名，返回一个布尔值。 |
| `visit_dir` | function | 发现或探测一层目录，返回收集到的元数据。 |
| `probe_video` | function | 探测主视频流，返回元数据；媒体不可读时返回无；工具或进程出错时返回错误。 |
| `estimate_av1_crf_from_bitrate` | function | 根据码率构造或推导 AV1 CRF 估算值，返回算得的值。 |
| `apply_quality_preservation_adjustment` | function | 提供按保真程度调整画质的行为，返回声明的结果。 |
| `contains_any_ignore_case` | function | 提供忽略大小写的任一包含判断的行为，返回声明的结果。 |
| `source_codec_family` | function | 提供源编码族判定的行为，返回声明的结果。 |
| `map_source_quantizer_to_av1` | function | 构造或推导把源量化参数映射到 AV1 的结果，返回算得的值。 |
| `codec_efficiency_factor` | function | 提供编码效率系数的行为，返回声明的结果。 |
| `estimate_target_bitrate` | function | 构造或推导目标码率估算值，返回算得的值。 |
| `hardware_bitrate_ratio` | function | 提供硬件码率比例的行为，返回声明的结果。 |
| `fmt` | function | 提供格式化的行为，返回声明的结果。 |
| `extract_named_number` | function | 提供按名称提取数值的行为，返回声明的结果。 |
| `parse_ratio` | function | 解析比例输入，返回解析出的值或错误。 |
| `parse_frame_rate` | function | 解析帧率输入，返回解析出的值或错误。 |
| `progress_bar` | function | 提供进度条的行为，返回声明的结果。 |
| `estimate_eta` | function | 构造或推导剩余时间估算值，返回算得的值。 |
| `format_duration` | function | 构造或推导时长的显示文本，返回算得的值。 |
| `format_bytes` | function | 构造或推导字节数的显示文本，返回算得的值。 |
| `size_change_label` | function | 提供体积变化标签的行为，返回声明的结果。 |
| `copy_speed_label` | function | 执行复制速度标签的操作，返回操作状态或结果。 |
| `nonzero` | function | 提供非零判断的行为，返回声明的结果。 |
| `with_added_suffix` | function | 提供追加后缀的行为，返回声明的结果。 |
| `has_added_suffix` | function | 检查是否带有追加的后缀，返回一个布尔值。 |
| `temp_output_path` | function | 提供临时输出路径的行为，返回一个带时间戳、且容器扩展名正确的 MyVidComp 临时路径。 |
| `cached_temp_output_paths` | function | 提供缓存临时输出路径的行为，返回两种容器中相匹配的 MyVidComp 临时文件，最新的在前。 |
