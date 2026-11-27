# 选项

每一个面向用户的选项：它的传输值、它的标签，以及把文本变成设置的解析过程。面向读者的说明见
[../options.md](../options.md)。

| 声明 | 种类 | 用途 |
|---|---|---|
| `as_str` | function | 把目标编码映射到它稳定的传输值，返回静态标签。 |
| `label` | function | 把目标编码映射到便于人读的日志标签，返回静态标签。 |
| `ffprobe_name` | function | 报告输出必须带有的 ffprobe 编码名称，返回静态编码名称。 |
| `mp4_tag` | function | 报告这一种编码为获得广泛播放器支持所需的 MP4 编码标签，返回标签或无。 |
| `parse_target_codec` | function | 解析目标编码的传输值，返回该编码或一条面向用户的错误。 |
| `normalize_target_codec_choice` | function | 归一化可选的目标编码值，空白、auto 或 default 时返回无，否则返回解析出的编码。 |
| `as_str` | function | 把保真策略映射到它稳定的传输值，返回静态标签。 |
| `label` | function | 把保真策略映射到便于人读的日志标签，返回静态标签。 |
| `allows_deviations` | function | 报告是否允许接受可挽回的差异，宽松模式下返回真。 |
| `parse_preservation` | function | 解析保真策略的传输值，返回该策略或一条面向用户的错误。 |
| `normalize_preservation_choice` | function | 归一化可选的保真策略值，空白、auto 或 default 时返回无，否则返回解析出的策略。 |
| `as_str` | function | 把画质策略映射到它稳定的传输值，返回静态标签。 |
| `label` | function | 把画质策略映射到便于人读的日志标签，返回静态标签。 |
| `parse_quality_mode` | function | 解析画质策略的传输值，返回该策略或一条面向用户的错误。 |
| `normalize_quality_mode_choice` | function | 归一化可选的画质策略值，空白、auto 或 default 时返回无，否则返回解析出的策略。 |
| `as_str` | function | 把画质检查方式映射到它稳定的传输值，返回静态标签。 |
| `label` | function | 把画质检查方式映射到便于人读的日志标签，返回静态标签。 |
| `subsample` | function | 报告两次测量之间要跳过多少帧，返回 libvmaf 的 n_subsample 值。 |
| `parse_quality_check` | function | 解析画质检查方式的传输值，返回该设置或一条面向用户的错误。 |
| `normalize_quality_check_choice` | function | 归一化可选的画质检查值，空白、auto 或 default 时返回无，否则返回解析出的设置。 |
| `as_str` | function | 把编码器偏好映射到它稳定的传输值，返回静态标签。 |
| `label` | function | 把编码器偏好映射到便于人读的日志标签，返回静态标签。 |
| `parse_encoder_preference` | function | 解析编码器偏好的传输值，包括已停用的旧转换模式名称，返回该偏好或一条面向用户的错误。 |
| `normalize_encoder_preference_choice` | function | 归一化可选的编码器偏好值，空白或 default 时返回无，否则返回解析出的偏好。 |
| `validate_quality_target` | function | 校验以 VMAF 分数百分之一为单位的画质目标，返回该目标或一条面向用户的错误。 |
| `parse_quality_target` | function | 把 “95” 或 “94.5” 这样的十进制画质目标解析为百分之一单位，返回该值或一条面向用户的错误。 |
| `format_quality` | function | 把百分之一单位的画质值格式化以供显示，返回一个简短的十进制字符串。 |
| `is_unset_choice` | function | 检查一个选项字符串是否表示“未设置”，空白、auto 或 default 时返回真。 |
| `parse_quality_points` | function | 把 “2” 或 “1.5” 这样的十进制 VMAF 分值解析为百分之一单位，返回该值或一条面向用户的错误。 |
| `as_str` | function | 把画质指标映射到它稳定的传输值，返回静态标签。 |
| `label` | function | 把画质指标映射到便于人读的日志标签，返回静态标签。 |
| `parse_quality_metric` | function | 解析画质指标的传输值，返回该指标或一条面向用户的错误。 |
| `normalize_quality_metric_choice` | function | 归一化可选的画质指标值，空白、auto 或 default 时返回无，否则返回解析出的指标。 |
| `as_str` | function | 把编码预设映射到它稳定的传输值，返回静态标签。 |
| `label` | function | 把编码预设映射到便于人读的日志标签，返回静态标签。 |
| `parse_encoder_preset` | function | 解析编码预设的传输值，返回该预设或一条面向用户的错误。 |
| `normalize_encoder_preset_choice` | function | 归一化可选的编码预设值，空白、auto 或 default 时返回无，否则返回解析出的预设。 |
| `as_str` | function | 把音频策略映射到它稳定的传输值，返回静态标签。 |
| `parse_audio_policy` | function | 解析音频策略的传输值，返回该策略或一条面向用户的错误。 |
| `normalize_audio_policy_choice` | function | 归一化可选的音频策略值，空白或 default 时返回无，否则返回解析出的策略。 |
| `as_str` | function | 把反交错策略映射到它稳定的传输值，返回静态标签。 |
| `parse_deinterlace_policy` | function | 解析反交错策略的传输值，返回该策略或一条面向用户的错误。 |
| `normalize_deinterlace_policy_choice` | function | 归一化可选的反交错策略值，空白或 default 时返回无，否则返回解析出的策略。 |
| `as_str` | function | 把体积保护策略映射到它稳定的传输值，返回静态标签。 |
| `parse_size_guard` | function | 解析体积保护策略的传输值，返回该策略或一条面向用户的错误。 |
| `normalize_size_guard_choice` | function | 归一化可选的体积保护策略值，空白或 default 时返回无，否则返回解析出的策略。 |
| `as_str` | function | 把硬件加速后端映射到它稳定的 FFmpeg 方式名称，返回静态标签。 |
| `as_str` | function | 把解码器偏好映射到它稳定的传输值，返回静态标签。 |
| `label` | function | 把解码器偏好格式化为便于人读的状态日志标签，返回字符串标签。 |
| `allows_hardware` | function | 报告解码器偏好是否允许硬件解码，仅 CPU 模式下返回假。 |
| `parse_decoder_preference` | function | 解析解码器偏好的传输值（`auto`、`gpu`、`cpu` 或指定方式名），返回该偏好或一条面向用户的错误。 |
| `normalize_decoder_preference_choice` | function | 归一化可选的解码器偏好值，空白或 default 时返回无，否则返回解析出的偏好。 |
