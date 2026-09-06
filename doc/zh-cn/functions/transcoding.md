# 计划与转码

为一个文件构建有序的编码尝试列表并逐个运行。面向读者的说明见
[../options.md](../options.md) 和 [../encoders.md](../encoders.md)。

| 声明 | 种类 | 用途 |
|---|---|---|
| `codec_forced_pixel_format` | function | 报告某一种编码对它无法直接接受的源强制施加的转换，返回一种适配方案或无。 |
| `metadata_exemptions_for` | function | 算出所选编码对这个源无法记录的色彩细节，返回宽松运行下可以接受的豁免项。 |
| `exemption_descriptions` | function | 向用户描述每一项被接受的色彩豁免，每处变化返回一句话。 |
| `closest_hardware_pixel_format` | function | 选择最接近且安全的初始硬件像素格式目标，返回目标及变化说明；适配不安全或没有必要时返回无。 |
| `pixel_format_bit_depth` | function | 从 FFmpeg 像素格式名称中提取显式的 YUV 平面位深，如 9、10、12、14 或 16，隐含 8 位的名称返回无。 |
| `pixel_format_has_alpha` | function | 检出不得自动适配的带 alpha 像素格式，已知的 YUV/RGB alpha 格式返回真。 |
| `transcode_item` | function | 产出并提交一个通过校验的输出，同时允许在精确与适配编码器之间安全回退。 |
| `search_quality_for` | function | 通过编码并测量若干短样本，为一个文件调定画质设置。 |
| `plan_deviations` | function | 列出一个编码方案引入的可挽回差异，为每一项返回类别和一句说明。 |
| `assess_quality` | function | 测量一次完成的转换，并判定它是否需要人来决定，返回该结果。 |
| `finish_conversion` | function | 提交一次完成的转换，或替换原文件，或两份都留待复核，返回转换前后的体积或一条终止性错误。 |
| `commit_for_review` | function | 把完成的转换放到未被触动的原文件旁边，并记录它为何需要决定，返回转换前后的体积或一条终止性错误。 |
| `execute_transcode_attempt` | function | 执行一次精确或明确适配的编码器尝试但不提交，返回成功或分类过的失败。 |
| `remove_failed_attempt_output` | function | 在重用之前删除一次失败尝试留下的临时输出，返回成功或终止性的磁盘错误。 |
| `commit_validated_output` | function | 读取体积并提交一个通过校验的输出，返回转换前后的体积或终止性错误。 |
| `ffmpeg_failure_is_encoder_retryable` | function | 把一次 ffmpeg 失败判定为可安全回退编码器的情形，仅对已知的设备/编码器/格式能力诊断返回真。 |
| `validation_failure_is_retryable` | function | 为编码器回退对严格输出校验失败分类，工具/进程/修复等基础设施故障返回假。 |
| `validate_or_repair_output` | function | 校验或修复输出，返回成功、失败或测试断言结果。 |
| `prepare_cached_temp_output` | function | 找出此前生成、且在当前精确或硬件适配策略下仍能通过校验的 MyVidComp 临时文件。 |
| `validate_cached_output` | function | 按精确保真或当前硬件模式的适配校验一个缓存输出，两种允许的方案任一通过即返回成功。 |
| `validation_error_may_need_remux_repair` | function | 检查一次校验失败能否通过不重新编码的重封装或 AV1 元数据修复解决，流数量和色彩/色度元数据失败时返回真。 |
| `validation_error_indicates_unusable_temp` | function | 检查一次校验失败是否意味着临时输出无法安全重用，输出不可读或结构错误时返回真。 |
| `remove_unusable_temp_output` | function | 在不可挽回的校验失败后删除临时输出，无返回值。 |
| `is_temp_output_path` | function | 检查一个路径是否像本工具自己的转换临时输出，当前的 .myvidcomp-*.tmp.mp4/.mkv 名称和旧的 .pvac-* 名称都返回真。 |
| `file_size` | function | 提供文件体积的行为，返回声明的结果。 |
| `build_ffmpeg_args` | function | 为一次普通的精确 AV1 尝试构造 ffmpeg 参数，供测试使用，返回命令参数。 |
| `build_ffmpeg_args_for_plan` | function | 为一次编码尝试构造 ffmpeg 参数，返回命令参数。 |
| `ensure_metadata_bsf` | function | 检查当前 ffmpeg 构建是否带有某一种编码记录色彩元数据所需的比特流过滤器，返回成功或一条面向用户的错误。 |
| `repair_av1_metadata` | function | 执行 AV1 元数据修复操作，返回操作状态或结果。 |
| `build_av1_metadata_repair_args` | function | 构造或推导 AV1 元数据修复参数，返回算得的值。 |
| `stream_map_args` | function | 依据探测到的实际流索引构造显式的 FFmpeg 映射，每个索引返回一对 -map，绝不使用宽泛映射。 |
| `replace_with_repaired_output` | function | 执行以修复后的输出替换原输出的操作，返回操作状态或结果。 |
| `ffmpeg_failure_detail` | function | 提供 ffmpeg 失败细节的行为，返回声明的结果。 |
| `output_frame_rate_arg` | function | 提供输出帧率参数的行为，返回声明的结果。 |
| `output_pix_fmt_arg` | function | 提供输出像素格式参数的行为，返回声明的结果。 |
| `normalized_pix_fmt` | function | 构造或推导归一化的像素格式，返回算得的值。 |
| `color_metadata_args` | function | 提供色彩元数据参数的行为，返回声明的结果。 |
| `metadata_bsf_arg` | function | 构造把源色彩元数据带入编码流的比特流过滤器参数，返回该参数；没有可带入的内容时返回无。 |
| `unsupported_metadata_reason` | function | 报告所选编码无法记录的第一项色彩细节，返回一条跳过原因；全部都能保留时返回无。 |
| `av1_color_primaries_value` | function | 提供 AV1 色彩原色取值的行为，返回声明的结果。 |
| `av1_transfer_characteristics_value` | function | 提供 AV1 传输特性取值的行为，返回声明的结果。 |
| `av1_matrix_coefficients_value` | function | 提供 AV1 矩阵系数取值的行为，返回声明的结果。 |
| `metadata_key` | function | 提供元数据键名的行为，返回声明的结果。 |
| `aspect_metadata_args` | function | 提供宽高比元数据参数的行为，返回声明的结果。 |
| `push_optional_arg` | function | 提供追加可选参数的行为，返回声明的结果。 |
| `label` | function | 为日志和事件描述一项画质设置，返回一个简短标签。 |
| `default_encoder_quality` | function | 为一个文件和一个编码器选择起始画质设置，按该编码器自身的刻度返回该设置。 |
| `quality_value_arg` | function | 按编码器期望的写法格式化一个恒定画质值，返回参数文本。 |
| `mediacodec_args` | function | 构造每一次 Android 硬件编码都需要的参数，返回参数列表。 |
| `encoder_quality_args` | function | 为一个编码器在一个设置下构造画质参数，返回参数列表。 |
| `run_ffmpeg_with_progress` | function | 运行 ffmpeg 并同时报告进度，返回退出状态与捕获的 stderr，或进程管理错误。 |
| `read_to_string` | function | 提供读取为字符串的行为，返回声明的结果。 |
| `validate_output` | function | 校验输出条件，返回成功、失败或测试断言结果。 |
| `validate_output_for_plan` | function | 对照精确或适配方案校验输出，返回成功或校验错误。 |
| `validate_output_against` | function | 对照预期的主视频属性和源的流布局校验探测到的输出，返回成功或校验错误。 |
