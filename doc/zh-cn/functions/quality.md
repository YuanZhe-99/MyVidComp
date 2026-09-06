# 画质

把完成的转换与源做比较，以及选择编码时使用的设置。面向读者的说明见
[../quality.md](../quality.md)。

| 声明 | 种类 | 用途 |
|---|---|---|
| `points` | function | 把分数换算为 VMAF 分值，以浮点数返回该分数。 |
| `default` | function | 构造默认的测量选项，返回在所有核心上做抽样测量的设置。 |
| `detect_vmaf_support` | function | 检查当前 FFmpeg 构建能否测量画质，返回支持情况，不支持时附带原因。 |
| `model_for` | function | 选择与源分辨率相称的 VMAF 模型，返回内置模型名称。 |
| `comparison_pixel_format` | function | 选择两路比较输入都能转换到的像素格式，返回一种 libvmaf 接受的平面 YUV 格式。 |
| `build_vmaf_args` | function | 构造把转换结果与其源做比较的 ffmpeg 参数，返回参数列表。 |
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
| `extract_samples` | function | 把若干短样本从源复制到工作目录，返回这些样本，一个也取不到时返回错误。 |
| `measure_trial` | function | 以同一个画质设置编码每个样本并测量结果，返回各样本分数的平均值。 |
| `trial_quality_args` | function | 构造一次试验编码在快速预设下的画质参数，返回参数列表。 |
| `quality_from_value` | function | 把搜索得到的值换算回编码实际使用的画质设置，返回该设置。 |
| `hardware_bitrate_ladder` | function | 列出硬件搜索应当尝试的码率，从小到大，以每秒千比特为单位返回。 |
