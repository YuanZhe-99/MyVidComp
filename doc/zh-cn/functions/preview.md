# 预览与对比试看

供交互式对比检查器使用的后端帧提取、差异热力图生成、同步 JPEG 突发帧序列播放以及并排对比片段导出。面向读者的说明见 [../review.md](../review.md) 与 [../ffi-abi.md](../ffi-abi.md)。

| 声明 | 种类 | 用途 |
|---|---|---|
| `probe_preview_info` | function | 探测视频的预览时长、帧数、像素分辨率和帧率，返回元数据或错误。 |
| `preview_info_json` | function | 为 FFI 调用方把预览流元数据序列化为紧凑 JSON，返回 JSON 文本。 |
| `build_single_frame_args` | function | 构造在指定时间戳提取一帧高分辨率 PNG 静帧的 FFmpeg 参数，返回参数列表。 |
| `build_diff_heatmap_args` | function | 构造结合原始帧与转换帧并通过 `blend=all_mode=difference` 增强对比度的 FFmpeg 参数，返回参数列表。 |
| `extract_frame_pair` | function | 提取同步的原图与转换后 PNG 静帧及可选差异热力图，返回输出路径或错误。 |
| `frame_pair_json` | function | 把提取出的静帧对比路径序列化为 JSON，返回 JSON 文本。 |
| `build_clip_frames_args` | function | 构造解码一段同步 720p JPEG 帧突发序列的 FFmpeg 参数，返回参数列表。 |
| `extract_clip_sequence` | function | 把原视频与转换视频的成对 JPEG 突发帧序列提取到缓存目录供应用内实时对比播放，返回帧路径列表或错误。 |
| `clip_sequence_json` | function | 把提取出的 JPEG 播放序列路径及计时信息序列化为 JSON，返回 JSON 文本。 |
| `export_comparison_clip` | function | 编码一段合并原始视频与转换视频的 6 秒水平并排 MP4 对比片段，返回成功或错误。 |
| `list_usable_encoders_json` | function | 探测匹配目标编解码器的 FFmpeg 编码器并验证运行时可用性，返回序列化后的候选描述符 JSON。 |
| `run_ffmpeg_quiet` | function | 通过 `process_command` 无黑框静默运行 FFmpeg，返回成功或主要 stderr 诊断行。 |
| `clear_frame_files` | function | 清理预览工作目录中残留的 PNG/JPEG 预览产物，无返回值。 |
| `collect_prefixed_jpegs` | function | 在预览目录中收集并排序匹配序列前缀的已提取 JPEG 帧，返回有序路径。 |
