# Quality

Measuring a finished conversion against its source, and choosing the setting to encode it at.
Described for readers in [../quality.md](../quality.md).

| Declaration | Kind | Purpose |
|---|---|---|
| `points` | function | Converts a score to a VMAF point value, returning the score as a float. |
| `default` | function | Builds default measurement options, returning sampled measurement on all cores. |
| `detect_vmaf_hdr_v1_support` | function | Checks whether the FFmpeg build supports `hdr_vmaf_v1.1.8_3d0h` via a 1-frame synthetic probe, returning a boolean. |
| `detect_vmaf_support` | function | Checks whether the FFmpeg build can measure quality (and whether HDR VMAF v1 is available), returning support with a reason when unavailable. |
| `model_for` | function | Chooses the VMAF model that matches the source resolution, returning the built-in model name. |
| `model_for_with_support` | function | Chooses the VMAF model matching resolution and HDR transfer capability, routing PQ (`smpte2084`) to `hdr_vmaf_v1.1.8` when available. |
| `accuracy_caveat_with_support` | function | Reports whether HDR accuracy is handled natively (`hdr_vmaf_v1.1.8` for PQ) or warrants an inaccuracy caveat sentence. |
| `comparison_pixel_format` | function | Chooses a pixel format both comparison inputs can be converted to, returning a planar YUV format libvmaf accepts. |
| `build_vmaf_args` | function | Builds the ffmpeg arguments that compare a converted file against its source (with optional HDR v1 routing and JSON frame logging), returning the argument list. |
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
| `extract_samples` | function | Copies short samples out of a source into the working directory, returning the samples, or an error when none could be taken; removes a failed sample at once and every written sample when an error ends extraction. |
| `measure_trial` | function | Encodes every sample at one quality setting and measures the result, returning the average score across the samples. |
| `trial_quality_args` | function | Builds the quality arguments for one trial encode at a fast preset, returning the argument list. |
| `quality_from_value` | function | Converts a searched value back into the quality setting the encode will use, returning the setting. |
| `hardware_bitrate_ladder` | function | Lists the bitrates a hardware search should try, from smallest to largest, returning bitrates in kilobits per second. |
| `detect_xpsnr_support` | function | Checks whether the FFmpeg build includes the built-in `xpsnr` filter, returning a boolean. |
| `detect_vmaf_cuda_support` | function | Checks whether the FFmpeg build includes `libvmaf_cuda`, returning a boolean. |
| `can_score_on_gpu` | function | Checks whether a video uses 8-bit 4:2:0 pixel format suitable for CUDA VMAF scoring, returning a boolean. |
| `build_vmaf_cuda_args` | function | Builds FFmpeg arguments for CUDA-resident `libvmaf_cuda` comparison, returning the argument list. |
| `segmented_plan` | function | Computes up to six 6-second comparison windows across files 60 s or longer, returning segment windows or none for short files. |
| `parse_keyframe_timestamps` | function | Parses `ffprobe` CSV keyframe timestamps, returning sorted deduplicated seconds. |
| `probe_keyframes` | function | Probes decoded keyframe timestamps from a video without decoding full frames, returning sorted seconds. |
| `align_windows_to_keyframes` | function | Snaps planned segment windows onto exact keyframes of the distorted output and drops near duplicates, returning aligned windows. |
| `parse_packet_samples` | function | Parses `ffprobe` packet CSV lines (`pts_time,size,flags`) into packet header samples without decoding frames. |
| `probe_packets` | function | Scans video packet headers via `ffprobe` without decoding frames, returning packet samples. |
| `dual_pool_plan` | function | Builds an isolated Dual-Pool segmented plan (Pool A stratified + Pool B risk probes) snapped to distorted keyframes, returning windows or none. |
| `snap_to_keyframe` | function | Snaps one segment window onto the nearest distorted keyframe within duration bounds, returning start seconds. |
| `escape_filter_path` | function | Escapes a path for FFmpeg `-lavfi` option values on Windows and POSIX, returning the escaped string. |
| `vmaf_json_log_suffix` | function | Formats optional `:log_fmt=json:log_path=...` suffix for `libvmaf`, returning suffix or empty string. |
| `parse_vmaf_json_frames` | function | Extracts `(frameNum, vmaf)` scores from a `libvmaf` JSON log using stdlib scanning, returning parsed frame scores. |
| `summarize_frame_scores` | function | Computes arithmetic mean, harmonic mean, P05, min, and worst frame index from per-frame VMAF scores. |
| `find_json_key_value_start` | function | Locates the value byte offset for a JSON object key after `:`, returning offset or none. |
| `find_matching_brace` | function | Finds the closing `}` matching an opening `{` in JSON text, returning index or none. |
| `parse_frame_object` | function | Extracts `(frameNum, vmaf)` from one JSON frame object slice, returning pair or none. |
| `parse_json_number_for_key` | function | Parses a finite JSON numeric value for a given key inside an object slice, returning float or none. |
| `build_segmented_vmaf_args` | function | Builds FFmpeg arguments using distorted keyframe seek plus two-stage hybrid seek+trim alignment on the reference for fast `libvmaf` scoring (with optional HDR v1 and JSON logging), returning the argument list. |
| `build_xpsnr_args` | function | Builds FFmpeg arguments comparing two streams using the fast CPU `xpsnr` filter with 10-bit portrait `transpose=1` protection and hybrid windowed alignment, returning the argument list. |
| `parse_xpsnr_score` | function | Parses the weighted XPSNR decibel value from FFmpeg output and converts it onto the 0–10000 scale, returning hundredths or none. |
| `xpsnr_score_from_output` | function | Reads a completed XPSNR run result, returning a `VmafScore` wrapper or a failure message. |
| `extract_xpsnr_channel` | function | Parses a single named XPSNR channel (`y`, `u`, or `v`) from an output summary line, returning decibels or none. |
| `xpsnr_db_to_vmaf_hundredths` | function | Maps a weighted XPSNR decibel score onto the VMAF hundredths scale, returning a bounded integer score. |
| `score_from_output` | function | Parses `libvmaf` output lines into a `VmafScore`, returning the score or an error message. |
| `vmaf_logit` | function | Maps a VMAF score in hundredths into logit space to linearize the CRF/QP S-curve, returning log-odds. |
| `logit_secant_extrapolate` | function | Extrapolates the next candidate outside a one-sided bracket using logit-space secant slope, returning value or none. |
| `next_quality_with_guess` | function | Warm-starts quality search from an estimated setting and steps towards the target using logit-space secant interpolation, returning the next setting or none when converged. |
