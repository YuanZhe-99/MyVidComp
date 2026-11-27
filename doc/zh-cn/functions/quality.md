# 画质

把完成的转换与源做比较，以及选择编码时使用的设置。面向读者的说明见
[../quality.md](../quality.md)。

| 声明 | 种类 | 用途 |
|---|---|---|
| `points` | function | 把分数换算为 VMAF 分值，以浮点数返回该分数。 |
| `default` | function | 构造默认的测量选项，返回在所有核心上做抽样测量的设置。 |
| `detect_vmaf_hdr_v1_support` | function | 通过单帧合成探针检查当前 FFmpeg 构建是否支持 `hdr_vmaf_v1.1.8_3d0h`，返回布尔值。 |
| `detect_vmaf_support` | function | 检查当前 FFmpeg 构建能否测量画质及是否支持 HDR VMAF v1，返回支持情况，不支持时附带原因。 |
| `model_for` | function | 选择与源分辨率相称的 VMAF 模型，返回内置模型名称。 |
| `model_for_with_support` | function | 按分辨率与 HDR 传输特性选择 VMAF 模型，支持时把 PQ（`smpte2084`）路由至 `hdr_vmaf_v1.1.8`。 |
| `accuracy_caveat_with_support` | function | 根据 HDR 模型能力报告当前源是否由原生模型准确评估，或需要附带不准提示句。 |
| `comparison_pixel_format` | function | 选择两路比较输入都能转换到的像素格式，返回一种 libvmaf 接受的平面 YUV 格式。 |
| `build_vmaf_args` | function | 构造把转换结果与其源做比较的 ffmpeg 参数（可选启用 HDR v1 与 JSON 逐帧日志），返回参数列表。 |
| `parse_vmaf_score` | function | 从 ffmpeg 输出中读取汇总分数，以百分之一单位返回该分数，未打印分数时返回无。 |
| `measure_vmaf` | function | 把一个转换结果与其源做比较测量，返回该分数或一条面向用户的错误。 |
| `accuracy_caveat` | function | 报告某个源是否属于 VMAF 无法可靠判断的类型，返回一句提醒或无。 |
| `sample_plan` | function | 决定从源中取多少个样本、每个多长，返回该方案，文件太短以致取样没有意义时返回无。 |
| `sample_offsets` | function | 算出每个样本从哪里开始，均匀分布并避开最开头与最末尾，为每个样本返回一个起始时间。 |
| `next_quality` | function | 根据目前的结果挑选下一个要试的画质值，返回该值，搜索已收敛时返回无。 |
| `best_quality` | function | 从所有试验中选定最终设置，返回达到目标的最大值，若都未达到则返回得分最好的值。 |
| `snap` | function | 把画质值取整到其编码器接受的步长上，返回范围内取整后的值。 |
| `midpoint` | function | 报告一个画质范围的中点并取整到其步长，返回搜索的起始值。 |
| `build_sample_args` | function | 构造从源中复制出一个样本的 ffmpeg 参数，返回参数列表。 |
| `build_trial_args` | function | 构造以某个试验画质编码一个样本的 ffmpeg 参数，返回参数列表。 |
| `paths` | function | 报告已提取出来的样本文件，返回它们的路径。 |
| `drop` | function | 删除提取出来的样本，无返回值。 |
| `extract_samples` | function | 把若干短样本从源复制到工作目录，返回这些样本，一个也取不到时返回错误；失败的样本立即删除，出错中止时删除已写入的全部样本。 |
| `measure_trial` | function | 以同一个画质设置编码每个样本并测量结果，返回各样本分数的平均值。 |
| `trial_quality_args` | function | 构造一次试验编码在快速预设下的画质参数，返回参数列表。 |
| `quality_from_value` | function | 把搜索得到的值换算回编码实际使用的画质设置，返回该设置。 |
| `hardware_bitrate_ladder` | function | 列出硬件搜索应当尝试的码率，从小到大，以每秒千比特为单位返回。 |
| `detect_xpsnr_support` | function | 检查当前 FFmpeg 构建是否包含内置的 `xpsnr` 滤镜，返回布尔值。 |
| `detect_vmaf_cuda_support` | function | 检查当前 FFmpeg 构建是否包含 `libvmaf_cuda`，返回布尔值。 |
| `can_score_on_gpu` | function | 检查视频是否采用适合 CUDA VMAF 评分的 8 位 4:2:0 像素格式，返回布尔值。 |
| `build_vmaf_cuda_args` | function | 构造完全留在显存中的 `libvmaf_cuda` 比较所需 FFmpeg 参数，返回参数列表。 |
| `segmented_plan` | function | 为 60 秒及以上的视频计算最多 6 个 6 秒比对窗口，返回分段窗口或短片返回无。 |
| `parse_keyframe_timestamps` | function | 解析 `ffprobe` 输出的关键帧 CSV 时间戳，返回排序去重后的秒数列表。 |
| `probe_keyframes` | function | 通过 `ffprobe` 仅扫包头探测视频关键帧时间戳，返回排序后的秒数列表。 |
| `align_windows_to_keyframes` | function | 将计划的分段窗口吸附到转换成品的真实关键帧并剔除过近重复窗口，返回对齐后的窗口。 |
| `parse_packet_samples` | function | 把 `ffprobe` 输出的包头 CSV 行（`pts_time,size,flags`）解析为不解码视频帧的包样本列表。 |
| `probe_packets` | function | 通过 `ffprobe` 扫描视频包头（零解码），返回解析后的包样本列表。 |
| `dual_pool_plan` | function | 基于零解码包扫描构建彼此隔离的双池分段抽检计划（A 池复杂度分层 + B 池风险探针）并对齐关键帧，返回窗口或无。 |
| `snap_to_keyframe` | function | 在时长范围内把单个分段窗口吸附到最近的成品关键帧，返回起始秒数。 |
| `escape_filter_path` | function | 在 Windows 与 POSIX 上为 FFmpeg `-lavfi` 滤镜选项转义文件路径，返回转义后字符串。 |
| `vmaf_json_log_suffix` | function | 为 `libvmaf` 格式化可选的 `:log_fmt=json:log_path=...` 后缀，返回后缀或空串。 |
| `parse_vmaf_json_frames` | function | 使用标准库扫描 `libvmaf` JSON 日志中的 `(frameNum, vmaf)` 逐帧分数，返回列表。 |
| `summarize_frame_scores` | function | 从逐帧 VMAF 分数计算算术平均、调和平均、P05、最低分及最差帧索引。 |
| `find_json_key_value_start` | function | 定位 JSON 对象成员在 `:` 后的值起始字节偏移，返回偏移或无。 |
| `find_matching_brace` | function | 查找与左花括号 `{` 配对的右花括号 `}` 索引，返回位置或无。 |
| `parse_frame_object` | function | 从单个 JSON 帧对象切片提取 `(frameNum, vmaf)`，返回元组或无。 |
| `parse_json_number_for_key` | function | 在对象切片中解析指定键对应的有限 JSON 数值，返回浮点数或无。 |
| `build_segmented_vmaf_args` | function | 构造对转换流直接关键帧跳转、对源流采用两段混合跳转与 `trim` 对齐的分段 `libvmaf` 参数（可选启用 HDR v1 与 JSON 日志），返回参数列表。 |
| `build_xpsnr_args` | function | 构造使用快速 CPU `xpsnr` 滤镜（含 10 位竖屏 `transpose=1` 防护与分段混合对齐）的比对参数，返回参数列表。 |
| `parse_xpsnr_score` | function | 从 FFmpeg 输出解析加权 XPSNR 分贝值并换算到 0–10000 标度，返回百分之一分或无。 |
| `xpsnr_score_from_output` | function | 读取已完成的 XPSNR 运行输出，返回 `VmafScore` 包装或失败消息。 |
| `extract_xpsnr_channel` | function | 从输出汇总行中解析单个指定 XPSNR 通道（`y`、`u` 或 `v`），返回分贝值或无。 |
| `xpsnr_db_to_vmaf_hundredths` | function | 把加权 XPSNR 分贝值映射到 VMAF 百分之一标度，返回受限整数分数。 |
| `score_from_output` | function | 把 `libvmaf` 输出文本解析为 `VmafScore`，返回分数或错误消息。 |
| `vmaf_logit` | function | 把以百分之一为单位的 VMAF 分数映射到 logit 空间使 CRF/QP S 形曲线近似线性，返回对数几率。 |
| `logit_secant_extrapolate` | function | 在单边区间外基于 logit 空间割线斜率外推下一候选值，返回外推值或无。 |
| `next_quality_with_guess` | function | 从估算设置热启动并通过 logit 空间割线插值向目标步进，返回下一设定值，已收敛时返回无。 |
