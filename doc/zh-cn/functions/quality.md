# 画质

把完成的转换与源做比较，以及选择编码时使用的设置。面向读者的说明见
[../quality.md](../quality.md)。

声明名称与用途说明直接取自源码中的 `AI-FUNC-SUMMARY` 注释，按本仓库的编写规则，这些注释以英文书写，
因此不作翻译。

| 声明 | 种类 | 用途 |
|---|---|---|
| `points` | function | Converts a score to a VMAF point value, returning the score as a float. |
| `default` | function | Builds default measurement options, returning sampled measurement on all cores. |
| `detect_vmaf_support` | function | Checks whether the FFmpeg build can measure quality, returning support with a reason when unavailable. |
| `model_for` | function | Chooses the VMAF model that matches the source resolution, returning the built-in model name. |
| `comparison_pixel_format` | function | Chooses a pixel format both comparison inputs can be converted to, returning a planar YUV format libvmaf accepts. |
| `build_vmaf_args` | function | Builds the ffmpeg arguments that compare a converted file against its source, returning the argument list. |
| `parse_vmaf_score` | function | Reads the pooled score out of ffmpeg output, returning the score in hundredths or none when no score was printed. |
| `measure_vmaf` | function | Measures one converted file against its source, returning the score or a user-facing error. |
| `accuracy_caveat` | function | Reports whether a source is one VMAF cannot judge reliably, returning a caveat sentence or none. |
| `sample_plan` | function | Decides how many samples to take from a source and how long each should be, returning the plan, or none when the file is too short to sample usefully. |
| `sample_offsets` | function | Works out where each sample starts, spread evenly and away from the very beginning and end, returning one start time per sample. |
| `next_quality` | function | Picks the next quality value to try from the results so far, returning the value, or none when the search has converged. |
| `best_quality` | function | Chooses the final setting from every trial, returning the largest value that reached the target, or the best-scoring value when none did. |
| `snap` | function | Rounds a quality value onto the steps its encoder accepts, returning the snapped value inside the range. |
| `midpoint` | function | Reports the middle of a quality range, snapped to its steps, returning the starting value for a search. |
| `build_sample_args` | function | Builds the ffmpeg arguments that copy one sample out of a source, returning the argument list. |
| `build_trial_args` | function | Builds the ffmpeg arguments that encode one sample at a trial quality, returning the argument list. |
| `paths` | function | Reports the sample files that were extracted, returning their paths. |
| `drop` | function | Removes the extracted samples, returning none. |
| `extract_samples` | function | Copies short samples out of a source into the working directory, returning the samples, or an error when none could be taken. |
| `measure_trial` | function | Encodes every sample at one quality setting and measures the result, returning the average score across the samples. |
| `trial_quality_args` | function | Builds the quality arguments for one trial encode at a fast preset, returning the argument list. |
| `quality_from_value` | function | Converts a searched value back into the quality setting the encode will use, returning the setting. |
| `hardware_bitrate_ladder` | function | Lists the bitrates a hardware search should try, from smallest to largest, returning bitrates in kilobits per second. |
