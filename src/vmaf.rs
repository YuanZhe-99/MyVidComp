//! Measuring how close a converted file looks to its source.
//!
//! Quality is measured with Netflix's VMAF through FFmpeg's `libvmaf` filter.
//! The filter is strict about its two inputs: the distorted video must be first,
//! the reference second, and both must arrive at the same size, pixel format and
//! timebase. Everything here exists to satisfy those rules and to read the one
//! number back out.

use std::path::Path;
use std::process::Stdio;

use super::{
    HwAccel, PROGRESS_ARGS, QualityMetric, VideoInfo, first_error_line, normalized_pix_fmt,
    pixel_format_bit_depth, process_command,
};

/// Which sampling pool a segmented quality window belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SegmentPool {
    /// Complexity-stratified sample representing global visual quality.
    Stratified,
    /// Targeted risk probe checking bitrate starvation or complexity spikes.
    Risk,
}

/// Result of a quality measurement, including optional per-frame pooling statistics
/// and a worst-quality timestamp.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QualityMeasurement {
    pub score: VmafScore,
    pub worst_frame_seconds: Option<f64>,
    pub metric: QualityMetric,
    pub harmonic_mean_hundredths: Option<u32>,
    pub p05_hundredths: Option<u32>,
    pub min_hundredths: Option<u32>,
}

/// One short window for segmented quality checking.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SegmentWindow {
    pub start_seconds: f64,
    pub duration_seconds: f64,
    pub pool: SegmentPool,
}

/// Packet header sample extracted via ffprobe without decoding video frames.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PacketSample {
    pub pts_seconds: f64,
    pub size_bytes: u64,
    pub is_keyframe: bool,
}

/// Per-frame VMAF summary metrics aggregated from libvmaf frame logs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameStats {
    pub mean_hundredths: u32,
    pub harmonic_mean_hundredths: u32,
    pub p05_hundredths: u32,
    pub min_hundredths: u32,
    pub worst_frame_index: usize,
    pub frame_count: usize,
}

/// A measured quality score, kept in hundredths of a VMAF point so it can cross
/// the FFI boundary as an integer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VmafScore {
    /// The pooled score for the whole comparison, in hundredths.
    pub hundredths: u32,
    /// Frames compared out of every group, so a caller can say how thorough the
    /// measurement was.
    pub subsample: u32,
}

impl VmafScore {
    // AI-FUNC-SUMMARY: Converts a score to a VMAF point value; returns the score as a float; side effects: none.
    pub fn points(&self) -> f64 {
        self.hundredths as f64 / 100.0
    }
}

/// How one measurement should be run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VmafOptions {
    /// Compare one frame out of every `subsample`. One compares every frame.
    pub subsample: u32,
    /// Worker threads. Zero matches the machine, because the filter itself
    /// defaults to single-threaded, which is far too slow for whole files.
    pub threads: u32,
}

impl Default for VmafOptions {
    // AI-FUNC-SUMMARY: Builds default measurement options; returns sampled measurement on all cores; side effects: none.
    fn default() -> Self {
        Self {
            subsample: 5,
            threads: 0,
        }
    }
}

/// What a run learned about the FFmpeg build it is using.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VmafSupport {
    pub available: bool,
    pub hdr_v1_available: bool,
}

// AI-FUNC-SUMMARY: Checks whether the FFmpeg build supports Netflix HDR VMAF v1 models (`hdr_vmaf_v1.1.8_3d0h`); returns true when the synthetic filter probe succeeds; side effects: runs ffmpeg on a 1-frame synthetic source.
pub fn detect_vmaf_hdr_v1_support(ffmpeg: &str) -> bool {
    let output = process_command(ffmpeg)
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-f",
            "lavfi",
            "-i",
            "color=c=black:s=64x64:d=0.04,format=yuv420p10le",
            "-f",
            "lavfi",
            "-i",
            "color=c=black:s=64x64:d=0.04,format=yuv420p10le",
            "-lavfi",
            "[0:v][1:v]libvmaf=model=version=hdr_vmaf_v1.1.8_3d0h",
            "-f",
            "null",
            "-",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    output.is_ok_and(|out| out.status.success())
}

// AI-FUNC-SUMMARY: Checks whether the FFmpeg build can measure quality (including HDR VMAF v1 support); returns support with a reason when unavailable; side effects: runs ffmpeg to inspect the filter.
pub fn detect_vmaf_support(ffmpeg: &str) -> (VmafSupport, Option<String>) {
    let output = process_command(ffmpeg)
        .args(["-hide_banner", "-loglevel", "error", "-h", "filter=libvmaf"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    match output {
        Ok(output) => {
            let mut text = String::from_utf8_lossy(&output.stdout).to_string();
            text.push_str(&String::from_utf8_lossy(&output.stderr));
            if text.contains("libvmaf") && text.contains("n_subsample") {
                let hdr_v1_available = detect_vmaf_hdr_v1_support(ffmpeg);
                (
                    VmafSupport {
                        available: true,
                        hdr_v1_available,
                    },
                    None,
                )
            } else {
                (
                    VmafSupport {
                        available: false,
                        hdr_v1_available: false,
                    },
                    Some(
                        "this FFmpeg build has no libvmaf filter, so quality cannot be measured"
                            .to_string(),
                    ),
                )
            }
        }
        Err(err) => (
            VmafSupport {
                available: false,
                hdr_v1_available: false,
            },
            Some(format!(
                "failed to ask ffmpeg about the libvmaf filter: {err}"
            )),
        ),
    }
}

// AI-FUNC-SUMMARY: Checks whether this FFmpeg build carries the fast built-in xpsnr filter; returns true when present; side effects: runs ffmpeg once to inspect the filter.
pub fn detect_xpsnr_support(ffmpeg: &str) -> bool {
    let output = process_command(ffmpeg)
        .args(["-hide_banner", "-loglevel", "error", "-h", "filter=xpsnr"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    let Ok(output) = output else {
        return false;
    };
    let mut text = String::from_utf8_lossy(&output.stdout).to_string();
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    output.status.success() && text.contains("Filter xpsnr")
}

// AI-FUNC-SUMMARY:
// Purpose: Checks whether this FFmpeg build can score on the graphics card.
// Inputs: The ffmpeg command.
// Returns: True when the CUDA variant of the filter is present.
// Side effects: Runs ffmpeg once to list the filter.
// Notes: Almost no build has it. It needs libvmaf compiled with CUDA and ffmpeg configured --enable-nonfree, and a nonfree build cannot be redistributed, so nobody ships one.
pub fn detect_vmaf_cuda_support(ffmpeg: &str) -> bool {
    let output = process_command(ffmpeg)
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-h",
            "filter=libvmaf_cuda",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    let Ok(output) = output else {
        return false;
    };
    let mut text = String::from_utf8_lossy(&output.stdout).to_string();
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    text.contains("libvmaf_cuda") && text.contains("n_subsample")
}

// AI-FUNC-SUMMARY:
// Purpose: Reports whether a source can be compared on the graphics card at all.
// Inputs: The source video.
// Returns: True only for eight-bit 4:2:0, which is all the CUDA filter accepts.
// Side effects: None.
pub fn can_score_on_gpu(video: &VideoInfo) -> bool {
    comparison_pixel_format(video) == "yuv420p"
}

// AI-FUNC-SUMMARY:
// Purpose: Builds the arguments that compare two files entirely on the graphics card.
// Inputs: The two files, the source video, the measurement settings and whether to report progress.
// Returns: The argument list.
// Side effects: None.
// Notes: The frames stay in graphics memory from decode to score, which is the opposite of the processor path and the reason for -hwaccel_output_format here.
pub fn build_vmaf_cuda_args(
    distorted: &Path,
    reference: &Path,
    video: &VideoInfo,
    options: VmafOptions,
    progress: bool,
) -> Vec<String> {
    let model = model_for(video);
    let subsample = options.subsample.max(1);

    // scale_cuda does on the card what format= does on the processor. There is
    // no n_threads: the work is on the card, not on worker threads.
    let filter = format!(
        "[0:v:0]settb=AVTB,setpts=PTS-STARTPTS,scale_cuda=format=yuv420p[dist];\
         [1:v:0]settb=AVTB,setpts=PTS-STARTPTS,scale_cuda=format=yuv420p[ref];\
         [dist][ref]libvmaf_cuda=model=version={model}:n_subsample={subsample}:shortest=1:ts_sync_mode=nearest"
    );

    let mut args = vec![
        "-hide_banner".to_string(),
        "-nostdin".to_string(),
        "-loglevel".to_string(),
        "info".to_string(),
        "-nostats".to_string(),
    ];
    if progress {
        args.extend(PROGRESS_ARGS.iter().map(|flag| (*flag).to_string()));
    }
    for path in [distorted, reference] {
        args.extend([
            "-hwaccel".to_string(),
            "cuda".to_string(),
            "-hwaccel_output_format".to_string(),
            "cuda".to_string(),
            "-i".to_string(),
            path.to_string_lossy().to_string(),
        ]);
    }
    args.extend([
        "-lavfi".to_string(),
        filter,
        "-f".to_string(),
        "null".to_string(),
        "-".to_string(),
    ]);
    args
}

/// Minimum duration for segmented spot checking; shorter clips are checked in full.
const MIN_SEGMENTED_SECONDS: f64 = 60.0;
/// Duration of each segmented spot-check window in seconds.
const SEGMENT_WINDOW_SECONDS: f64 = 6.0;
/// Maximum number of windows in a segmented spot check.
const MAX_SEGMENT_WINDOWS: usize = 6;

// AI-FUNC-SUMMARY: Plans evenly spaced short windows across a video for fast segmented quality checking; returns windows or none when the file is short enough to check whole; side effects: none.
pub fn segmented_plan(duration_seconds: f64) -> Option<Vec<SegmentWindow>> {
    if !duration_seconds.is_finite() || duration_seconds < MIN_SEGMENTED_SECONDS {
        return None;
    }

    let count = ((duration_seconds / 180.0).ceil() as usize).clamp(3, MAX_SEGMENT_WINDOWS);
    let usable_start = duration_seconds * 0.10;
    let usable_end = (duration_seconds * 0.90 - SEGMENT_WINDOW_SECONDS).max(usable_start);
    if usable_end <= usable_start {
        return None;
    }

    let windows = (0..count)
        .map(|index| {
            let fraction = if count == 1 {
                0.5
            } else {
                index as f64 / (count - 1) as f64
            };
            SegmentWindow {
                start_seconds: usable_start + (usable_end - usable_start) * fraction,
                duration_seconds: SEGMENT_WINDOW_SECONDS,
                pool: SegmentPool::Stratified,
            }
        })
        .collect();
    Some(windows)
}

// AI-FUNC-SUMMARY: Parses `ffprobe` packet CSV lines (`pts_time,size,flags`) into packet header samples; returns valid non-negative samples in stream order; side effects: none.
pub fn parse_packet_samples(text: &str) -> Vec<PacketSample> {
    text.lines()
        .filter_map(|line| {
            let mut parts = line.split(',');
            let pts_seconds = parts.next()?.trim().parse::<f64>().ok()?;
            if !pts_seconds.is_finite() || pts_seconds < 0.0 {
                return None;
            }
            let size_bytes = parts.next()?.trim().parse::<u64>().ok()?;
            let flags = parts.next().unwrap_or("").trim();
            Some(PacketSample {
                pts_seconds,
                size_bytes,
                is_keyframe: flags.starts_with('K'),
            })
        })
        .collect()
}

// AI-FUNC-SUMMARY: Scans video packet headers using `ffprobe` without decoding frames; returns parsed packet samples or empty on failure; side effects: spawns ffprobe.
pub fn probe_packets(ffprobe: &str, path: &Path) -> Vec<PacketSample> {
    let output = process_command(ffprobe)
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "packet=pts_time,size,flags",
            "-of",
            "csv=p=0",
        ])
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    let Ok(output) = output else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    parse_packet_samples(&String::from_utf8_lossy(&output.stdout))
}

#[derive(Debug, Clone, Copy)]
struct PacketBin {
    start: f64,
    src_bytes: u64,
    dist_bytes: u64,
    ratio: f64,
}

// AI-FUNC-SUMMARY: Builds an isolated Dual-Pool segmented plan (Pool A complexity-stratified + Pool B bitrate-starvation/spike risk probes) from zero-decode packet scans; returns snapped windows or none when short or insufficient; side effects: none.
pub fn dual_pool_plan(
    source_packets: &[PacketSample],
    distorted_packets: &[PacketSample],
    duration_seconds: f64,
) -> Option<Vec<SegmentWindow>> {
    if !duration_seconds.is_finite()
        || duration_seconds < 90.0
        || source_packets.is_empty()
        || distorted_packets.is_empty()
    {
        return None;
    }

    let start_bound = duration_seconds * 0.05;
    let end_bound = (duration_seconds * 0.95 - SEGMENT_WINDOW_SECONDS).max(start_bound);
    if end_bound <= start_bound {
        return None;
    }

    let num_bins = ((end_bound - start_bound) / SEGMENT_WINDOW_SECONDS).floor() as usize + 1;
    let mut bins: Vec<PacketBin> = (0..num_bins)
        .map(|idx| PacketBin {
            start: start_bound + idx as f64 * SEGMENT_WINDOW_SECONDS,
            src_bytes: 0,
            dist_bytes: 0,
            ratio: 0.0,
        })
        .collect();

    let max_pts = start_bound + num_bins as f64 * SEGMENT_WINDOW_SECONDS;
    for pkt in source_packets {
        if pkt.pts_seconds >= start_bound && pkt.pts_seconds < max_pts {
            let idx = ((pkt.pts_seconds - start_bound) / SEGMENT_WINDOW_SECONDS).floor() as usize;
            if let Some(bin) = bins.get_mut(idx) {
                bin.src_bytes = bin.src_bytes.saturating_add(pkt.size_bytes);
            }
        }
    }
    for pkt in distorted_packets {
        if pkt.pts_seconds >= start_bound && pkt.pts_seconds < max_pts {
            let idx = ((pkt.pts_seconds - start_bound) / SEGMENT_WINDOW_SECONDS).floor() as usize;
            if let Some(bin) = bins.get_mut(idx) {
                bin.dist_bytes = bin.dist_bytes.saturating_add(pkt.size_bytes);
            }
        }
    }

    let mut valid: Vec<PacketBin> = bins
        .into_iter()
        .filter(|b| b.src_bytes > 0 && b.dist_bytes > 0)
        .map(|mut b| {
            b.ratio = b.dist_bytes as f64 / b.src_bytes.max(1) as f64;
            b
        })
        .collect();
    if valid.len() < 6 {
        return None;
    }

    let overlaps = |chosen: &[SegmentWindow], start: f64| {
        chosen
            .iter()
            .any(|w| (w.start_seconds - start).abs() < SEGMENT_WINDOW_SECONDS - 1e-6)
    };

    let mut selected: Vec<SegmentWindow> = Vec::with_capacity(10);

    // Pool A: complexity-stratified (up to 6 windows: 2 from each tertile of source bitrate).
    valid.sort_by(|a, b| {
        a.src_bytes
            .cmp(&b.src_bytes)
            .then_with(|| a.start.total_cmp(&b.start))
    });
    let len = valid.len();
    let t1 = len / 3;
    let t2 = (2 * len) / 3;
    for slice in [&valid[0..t1], &valid[t1..t2], &valid[t2..len]] {
        if slice.is_empty() {
            continue;
        }
        let targets = [slice.len() / 3, (2 * slice.len()) / 3];
        let mut picked = 0usize;
        for &target_idx in &targets {
            if picked >= 2 {
                break;
            }
            // Probe target index first, then scan outward for a non-overlapping bin.
            let mut candidate_order = Vec::with_capacity(slice.len());
            candidate_order.push(target_idx.min(slice.len() - 1));
            for delta in 1..slice.len() {
                if let Some(right) = target_idx.checked_add(delta)
                    && right < slice.len()
                {
                    candidate_order.push(right);
                }
                if let Some(left) = target_idx.checked_sub(delta) {
                    candidate_order.push(left);
                }
            }
            for idx in candidate_order {
                let bin = slice[idx];
                if !overlaps(&selected, bin.start) {
                    selected.push(SegmentWindow {
                        start_seconds: bin.start,
                        duration_seconds: SEGMENT_WINDOW_SECONDS,
                        pool: SegmentPool::Stratified,
                    });
                    picked += 1;
                    break;
                }
            }
        }
    }

    // Pool B: risk probes (up to 2 bitrate-starvation lows + up to 2 source-complexity spikes).
    let mut by_ratio = valid.clone();
    by_ratio.sort_by(|a, b| {
        a.ratio
            .total_cmp(&b.ratio)
            .then_with(|| a.start.total_cmp(&b.start))
    });
    let mut starvation_picked = 0usize;
    for bin in by_ratio {
        if starvation_picked >= 2 {
            break;
        }
        if !overlaps(&selected, bin.start) {
            selected.push(SegmentWindow {
                start_seconds: bin.start,
                duration_seconds: SEGMENT_WINDOW_SECONDS,
                pool: SegmentPool::Risk,
            });
            starvation_picked += 1;
        }
    }

    let mut by_spike = valid;
    by_spike.sort_by(|a, b| {
        b.src_bytes
            .cmp(&a.src_bytes)
            .then_with(|| a.start.total_cmp(&b.start))
    });
    let mut spike_picked = 0usize;
    for bin in by_spike {
        if spike_picked >= 2 {
            break;
        }
        if !overlaps(&selected, bin.start) {
            selected.push(SegmentWindow {
                start_seconds: bin.start,
                duration_seconds: SEGMENT_WINDOW_SECONDS,
                pool: SegmentPool::Risk,
            });
            spike_picked += 1;
        }
    }

    // Snap all selected windows onto the distorted stream's keyframes while
    // preserving pool identity and prioritizing Pool A over Pool B on collisions.
    let mut keyframes: Vec<f64> = distorted_packets
        .iter()
        .filter(|p| p.is_keyframe && p.pts_seconds.is_finite() && p.pts_seconds >= 0.0)
        .map(|p| p.pts_seconds)
        .collect();
    keyframes.sort_by(|a, b| a.total_cmp(b));
    keyframes.dedup_by(|a, b| (*a - *b).abs() < 1e-4);

    let mut snapped: Vec<SegmentWindow> = selected
        .into_iter()
        .map(|window| {
            let start_seconds = snap_to_keyframe(window, &keyframes, duration_seconds);
            SegmentWindow {
                start_seconds,
                duration_seconds: window.duration_seconds,
                pool: window.pool,
            }
        })
        .collect();

    snapped.sort_by(|a, b| a.start_seconds.total_cmp(&b.start_seconds));
    let mut final_windows: Vec<SegmentWindow> = Vec::with_capacity(snapped.len());
    for window in snapped {
        if let Some(prev) = final_windows.last_mut()
            && (window.start_seconds - prev.start_seconds).abs() < 1.0
        {
            if prev.pool == SegmentPool::Risk && window.pool == SegmentPool::Stratified {
                *prev = window;
            }
            continue;
        }
        final_windows.push(window);
    }

    if final_windows
        .iter()
        .any(|w| w.pool == SegmentPool::Stratified)
    {
        Some(final_windows)
    } else {
        None
    }
}

// AI-FUNC-SUMMARY: Snaps one segment window onto the nearest distorted keyframe within bounds; returns the snapped start time in seconds; side effects: none.
fn snap_to_keyframe(window: SegmentWindow, keyframes: &[f64], duration_seconds: f64) -> f64 {
    if keyframes.is_empty() {
        return window.start_seconds;
    }
    keyframes
        .iter()
        .copied()
        .filter(|&k| k + window.duration_seconds <= duration_seconds + 0.5)
        .min_by(|a, b| {
            (a - window.start_seconds)
                .abs()
                .total_cmp(&(b - window.start_seconds).abs())
        })
        .or_else(|| {
            keyframes
                .iter()
                .copied()
                .filter(|&k| k <= window.start_seconds)
                .max_by(|a, b| a.total_cmp(b))
        })
        .unwrap_or(window.start_seconds)
}

// AI-FUNC-SUMMARY: Parses keyframe timestamps from ffprobe CSV output; returns sorted unique non-negative timestamps; side effects: none.
pub fn parse_keyframe_timestamps(text: &str) -> Vec<f64> {
    let mut times: Vec<f64> = text
        .lines()
        .filter_map(|line| {
            let token = line.split(',').next()?.trim();
            let value = token.parse::<f64>().ok()?;
            (value.is_finite() && value >= 0.0).then_some(value)
        })
        .collect();
    times.sort_by(|a, b| a.total_cmp(b));
    times.dedup_by(|a, b| (*a - *b).abs() < 1e-4);
    times
}

// AI-FUNC-SUMMARY: Probes decoded keyframe timestamps from a video file using ffprobe without full decoding; returns sorted timestamps or empty on failure; side effects: spawns ffprobe.
pub fn probe_keyframes(ffprobe: &str, path: &Path) -> Vec<f64> {
    let output = process_command(ffprobe)
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-skip_frame",
            "nokey",
            "-show_entries",
            "frame=pts_time",
            "-of",
            "csv=p=0",
        ])
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    let Ok(output) = output else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    parse_keyframe_timestamps(&String::from_utf8_lossy(&output.stdout))
}

// AI-FUNC-SUMMARY: Snaps planned segment windows onto exact keyframes of the distorted file and deduplicates close hits; returns aligned windows; side effects: none.
pub fn align_windows_to_keyframes(
    windows: &[SegmentWindow],
    keyframes: &[f64],
    duration_seconds: f64,
) -> Vec<SegmentWindow> {
    if keyframes.is_empty() {
        return windows.to_vec();
    }

    let mut aligned: Vec<SegmentWindow> = Vec::with_capacity(windows.len());
    for window in windows {
        let snapped = keyframes
            .iter()
            .copied()
            .filter(|&k| k + window.duration_seconds <= duration_seconds + 0.5)
            .min_by(|a, b| {
                (a - window.start_seconds)
                    .abs()
                    .total_cmp(&(b - window.start_seconds).abs())
            })
            .or_else(|| {
                keyframes
                    .iter()
                    .copied()
                    .filter(|&k| k <= window.start_seconds)
                    .max_by(|a, b| a.total_cmp(b))
            })
            .unwrap_or(window.start_seconds);

        if aligned
            .last()
            .is_some_and(|prev| (snapped - prev.start_seconds).abs() < 1.0)
        {
            continue;
        }
        aligned.push(SegmentWindow {
            start_seconds: snapped,
            duration_seconds: window.duration_seconds,
            pool: window.pool,
        });
    }

    if aligned.is_empty() {
        windows.to_vec()
    } else {
        aligned
    }
}

// AI-FUNC-SUMMARY: Escapes a file path so FFmpeg `-lavfi` accepts it as a filter option value on Windows and POSIX; returns escaped path string; side effects: none.
pub fn escape_filter_path(path: &Path) -> String {
    let raw = path.to_string_lossy().replace('\\', "/");
    let mut out = String::with_capacity(raw.len() + 8);
    for ch in raw.chars() {
        match ch {
            ':' | '\'' | ',' | ';' => {
                out.push('\\');
                out.push(ch);
            }
            _ => out.push(ch),
        }
    }
    out
}

// AI-FUNC-SUMMARY: Formats optional `log_fmt=json:log_path=...` suffix for `libvmaf`; returns suffix or empty string; side effects: none.
fn vmaf_json_log_suffix(log_path: Option<&Path>) -> String {
    match log_path {
        Some(path) => format!(":log_fmt=json:log_path={}", escape_filter_path(path)),
        None => String::new(),
    }
}

// AI-FUNC-SUMMARY: Parses per-frame `(frameNum, vmaf)` scores out of a libvmaf JSON log without external dependencies; returns sorted-by-appearance pairs; side effects: none.
pub fn parse_vmaf_json_frames(json: &str) -> Vec<(usize, f64)> {
    let Some(frames_key) = find_json_key_value_start(json, "frames") else {
        return Vec::new();
    };
    let rest = json[frames_key..].trim_start();
    let Some(arr_body) = rest.strip_prefix('[') else {
        return Vec::new();
    };

    let mut results = Vec::new();
    let bytes = arr_body.as_bytes();
    let mut i = 0usize;
    let mut in_string = false;
    let mut escape = false;

    while i < bytes.len() {
        let ch = bytes[i];
        if in_string {
            if escape {
                escape = false;
            } else if ch == b'\\' {
                escape = true;
            } else if ch == b'"' {
                in_string = false;
            }
            i += 1;
            continue;
        }
        match ch {
            b'"' => in_string = true,
            b']' => break,
            b'{' => {
                let obj_start = i;
                let Some(obj_end) = find_matching_brace(arr_body, obj_start) else {
                    break;
                };
                let frame_obj = &arr_body[obj_start..=obj_end];
                if let Some(pair) = parse_frame_object(frame_obj) {
                    results.push(pair);
                }
                i = obj_end + 1;
                continue;
            }
            _ => {}
        }
        i += 1;
    }
    results
}

// AI-FUNC-SUMMARY: Computes arithmetic mean, harmonic mean, P05, min, and worst frame index from per-frame VMAF scores; returns stats or none when empty; side effects: none.
pub fn summarize_frame_scores(frames: &[(usize, f64)]) -> Option<FrameStats> {
    let valid: Vec<(usize, f64)> = frames
        .iter()
        .copied()
        .filter(|&(_, score)| score.is_finite())
        .map(|(idx, score)| (idx, score.clamp(0.0, 100.0)))
        .collect();
    if valid.is_empty() {
        return None;
    }

    let count = valid.len();
    let mut sum = 0.0_f64;
    let mut recip_sum = 0.0_f64;
    let mut worst = valid[0];
    let mut sorted = Vec::with_capacity(count);

    for &(frame_idx, score) in &valid {
        sum += score;
        recip_sum += 1.0 / score.max(1.0);
        sorted.push(score);
        if score < worst.1 {
            worst = (frame_idx, score);
        }
    }

    sorted.sort_by(|a, b| a.total_cmp(b));
    let p05_idx = ((0.05 * (count - 1) as f64).floor() as usize).min(count - 1);
    let mean = sum / count as f64;
    let harmonic = count as f64 / recip_sum;

    let to_hundredths = |val: f64| (val * 100.0).round().clamp(0.0, 10_000.0) as u32;

    Some(FrameStats {
        mean_hundredths: to_hundredths(mean),
        harmonic_mean_hundredths: to_hundredths(harmonic),
        p05_hundredths: to_hundredths(sorted[p05_idx]),
        min_hundredths: to_hundredths(sorted[0]),
        worst_frame_index: worst.0,
        frame_count: count,
    })
}

// AI-FUNC-SUMMARY: Locates the start byte offset of a JSON object member's value after `:`; returns offset or none; side effects: none.
fn find_json_key_value_start(text: &str, key: &str) -> Option<usize> {
    let needle = format!("\"{key}\"");
    let mut search_from = 0usize;
    while let Some(rel) = text[search_from..].find(&needle) {
        let after_key = search_from + rel + needle.len();
        let trimmed = text[after_key..].trim_start();
        if let Some(after_colon) = trimmed.strip_prefix(':') {
            let offset = text.len() - after_colon.len();
            return Some(offset);
        }
        search_from = after_key;
    }
    None
}

// AI-FUNC-SUMMARY: Finds the closing `}` byte index matching an opening `{`; returns index or none; side effects: none.
fn find_matching_brace(text: &str, open_idx: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escape = false;
    for (idx, &b) in bytes.iter().enumerate().skip(open_idx) {
        if in_string {
            if escape {
                escape = false;
            } else if b == b'\\' {
                escape = true;
            } else if b == b'"' {
                in_string = false;
            }
            continue;
        }
        match b {
            b'"' => in_string = true,
            b'{' => depth += 1,
            b'}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(idx);
                }
            }
            _ => {}
        }
    }
    None
}

// AI-FUNC-SUMMARY: Extracts `(frameNum, vmaf)` from one JSON frame object slice; returns pair or none; side effects: none.
fn parse_frame_object(frame_obj: &str) -> Option<(usize, f64)> {
    let frame_num_pos = find_json_key_value_start(frame_obj, "frameNum")?;
    let frame_num_str: String = frame_obj[frame_num_pos..]
        .trim_start()
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    let frame_num = frame_num_str.parse::<usize>().ok()?;

    let metrics_start = find_json_key_value_start(frame_obj, "metrics")?;
    let metrics_rel = frame_obj[metrics_start..].find('{')?;
    let metrics_open = metrics_start + metrics_rel;
    let metrics_close = find_matching_brace(frame_obj, metrics_open)?;
    let metrics_obj = &frame_obj[metrics_open..=metrics_close];

    for key in ["vmaf", "vmaf_v1", "hdr_vmaf"] {
        if let Some(score) = parse_json_number_for_key(metrics_obj, key) {
            return Some((frame_num, score));
        }
    }
    None
}

// AI-FUNC-SUMMARY: Parses a finite JSON numeric value for a given key inside an object slice; returns float or none; side effects: none.
fn parse_json_number_for_key(obj: &str, key: &str) -> Option<f64> {
    let pos = find_json_key_value_start(obj, key)?;
    let token: String = obj[pos..]
        .trim_start()
        .chars()
        .take_while(|c| c.is_ascii_digit() || matches!(c, '.' | '-' | '+' | 'e' | 'E'))
        .collect();
    if token.is_empty() {
        return None;
    }
    token.parse::<f64>().ok().filter(|v| v.is_finite())
}

// AI-FUNC-SUMMARY: Builds ffmpeg arguments for one seeked segmented VMAF window using keyframe seek on distorted input and hybrid seek+trim on reference input; returns the argument list; side effects: none.
#[allow(clippy::too_many_arguments)]
pub fn build_segmented_vmaf_args(
    distorted: &Path,
    reference: &Path,
    video: &VideoInfo,
    options: VmafOptions,
    window: SegmentWindow,
    progress: bool,
    accel: (Option<HwAccel>, Option<HwAccel>),
    hdr_v1_available: bool,
    log_path: Option<&Path>,
) -> Vec<String> {
    let format = comparison_pixel_format(video);
    let model = model_for_with_support(video, hdr_v1_available);
    let threads = if options.threads == 0 {
        std::thread::available_parallelism()
            .map(|value| value.get() as u32)
            .unwrap_or(1)
    } else {
        options.threads
    };
    let subsample = options.subsample.max(1);
    let lead = 5.0_f64;
    let ref_coarse = (window.start_seconds - lead).max(0.0);
    let ref_trim_start = (window.start_seconds - ref_coarse).max(0.0);
    let duration = format!("{:.3}", window.duration_seconds);
    let log_suffix = vmaf_json_log_suffix(log_path);

    let filter = format!(
        "[0:v:0]settb=AVTB,setpts=PTS-STARTPTS,format={format}[dist];\
         [1:v:0]trim=start={ref_trim_start:.3}:duration={duration},settb=AVTB,setpts=PTS-STARTPTS,format={format}[ref];\
         [dist][ref]libvmaf=model=version={model}:n_threads={threads}:n_subsample={subsample}:shortest=1:ts_sync_mode=nearest{log_suffix}"
    );

    let mut args = vec![
        "-hide_banner".to_string(),
        "-nostdin".to_string(),
        "-loglevel".to_string(),
        "info".to_string(),
        "-nostats".to_string(),
    ];
    if progress {
        args.extend(PROGRESS_ARGS.iter().map(|flag| (*flag).to_string()));
    }

    if let Some(method) = accel.0 {
        args.push("-hwaccel".to_string());
        args.push(method.as_str().to_string());
    }
    args.extend([
        "-ss".to_string(),
        format!("{:.3}", window.start_seconds),
        "-t".to_string(),
        duration.clone(),
        "-i".to_string(),
        distorted.to_string_lossy().to_string(),
    ]);

    if let Some(method) = accel.1 {
        args.push("-hwaccel".to_string());
        args.push(method.as_str().to_string());
    }
    args.extend([
        "-ss".to_string(),
        format!("{ref_coarse:.3}"),
        "-t".to_string(),
        format!("{:.3}", ref_trim_start + window.duration_seconds + 1.0),
        "-i".to_string(),
        reference.to_string_lossy().to_string(),
    ]);

    args.extend([
        "-lavfi".to_string(),
        filter,
        "-f".to_string(),
        "null".to_string(),
        "-".to_string(),
    ]);
    args
}

// AI-FUNC-SUMMARY: Builds ffmpeg arguments to measure perceptual quality with the fast built-in xpsnr filter, applying 10-bit portrait transpose guard and hybrid reference alignment when windowed; returns the argument list; side effects: none.
pub fn build_xpsnr_args(
    distorted: &Path,
    reference: &Path,
    video: &VideoInfo,
    window: Option<SegmentWindow>,
    progress: bool,
    accel: (Option<HwAccel>, Option<HwAccel>),
) -> Vec<String> {
    let format = comparison_pixel_format(video);
    let portrait_guard =
        video.height > video.width && pixel_format_bit_depth(format).unwrap_or(8) > 8;
    let transpose = if portrait_guard { ",transpose=1" } else { "" };

    let mut args = vec![
        "-hide_banner".to_string(),
        "-nostdin".to_string(),
        "-loglevel".to_string(),
        "info".to_string(),
        "-nostats".to_string(),
    ];
    if progress {
        args.extend(PROGRESS_ARGS.iter().map(|flag| (*flag).to_string()));
    }

    let filter = if let Some(window) = window {
        let lead = 5.0_f64;
        let ref_coarse = (window.start_seconds - lead).max(0.0);
        let ref_trim_start = (window.start_seconds - ref_coarse).max(0.0);
        let duration = format!("{:.3}", window.duration_seconds);

        if let Some(method) = accel.0 {
            args.push("-hwaccel".to_string());
            args.push(method.as_str().to_string());
        }
        args.extend([
            "-ss".to_string(),
            format!("{:.3}", window.start_seconds),
            "-t".to_string(),
            duration.clone(),
            "-i".to_string(),
            distorted.to_string_lossy().to_string(),
        ]);

        if let Some(method) = accel.1 {
            args.push("-hwaccel".to_string());
            args.push(method.as_str().to_string());
        }
        args.extend([
            "-ss".to_string(),
            format!("{ref_coarse:.3}"),
            "-t".to_string(),
            format!("{:.3}", ref_trim_start + window.duration_seconds + 1.0),
            "-i".to_string(),
            reference.to_string_lossy().to_string(),
        ]);

        format!(
            "[0:v:0]settb=AVTB,setpts=PTS-STARTPTS,format={format}{transpose}[dist];\
             [1:v:0]trim=start={ref_trim_start:.3}:duration={duration},settb=AVTB,setpts=PTS-STARTPTS,format={format}{transpose}[ref];\
             [dist][ref]xpsnr"
        )
    } else {
        for (path, method) in [(distorted, accel.0), (reference, accel.1)] {
            if let Some(method) = method {
                args.push("-hwaccel".to_string());
                args.push(method.as_str().to_string());
            }
            args.extend(["-i".to_string(), path.to_string_lossy().to_string()]);
        }

        format!(
            "[0:v:0]settb=AVTB,setpts=PTS-STARTPTS,format={format}{transpose}[dist];\
             [1:v:0]settb=AVTB,setpts=PTS-STARTPTS,format={format}{transpose}[ref];\
             [dist][ref]xpsnr"
        )
    };

    args.extend([
        "-lavfi".to_string(),
        filter,
        "-f".to_string(),
        "null".to_string(),
        "-".to_string(),
    ]);
    args
}

// AI-FUNC-SUMMARY: Parses FFmpeg xpsnr summary output and maps weighted dB onto a VMAF-equivalent score in hundredths; returns none when no xpsnr line was printed; side effects: none.
pub fn parse_xpsnr_score(text: &str) -> Option<u32> {
    let mut found = None;
    for line in text.lines() {
        let Some(pos) = line.find("XPSNR") else {
            continue;
        };
        let rest = &line[pos..];
        let Some(y) = extract_xpsnr_channel(rest, "y:") else {
            continue;
        };
        let u = extract_xpsnr_channel(rest, "u:").unwrap_or(y);
        let v = extract_xpsnr_channel(rest, "v:").unwrap_or(y);
        if y.is_infinite() && u.is_infinite() && v.is_infinite() {
            found = Some(10000);
            continue;
        }
        let clamp_inf = |val: f64| if val.is_infinite() { 60.0 } else { val };
        let weighted_db = (4.0 * clamp_inf(y) + clamp_inf(u) + clamp_inf(v)) / 6.0;
        found = Some(xpsnr_db_to_vmaf_hundredths(weighted_db));
    }
    found
}

// AI-FUNC-SUMMARY: Turns finished xpsnr output into a VMAF-equivalent score; returns the score or a user-facing error; side effects: none.
pub fn xpsnr_score_from_output(
    succeeded: bool,
    text: &str,
    subsample: u32,
) -> Result<VmafScore, String> {
    if !succeeded {
        return Err(format!(
            "quality measurement failed: {}",
            first_error_line(text)
        ));
    }

    parse_xpsnr_score(text)
        .map(|hundredths| VmafScore {
            hundredths,
            subsample: subsample.max(1),
        })
        .ok_or_else(|| "quality measurement produced no XPSNR score".to_string())
}

// AI-FUNC-SUMMARY: Extracts one channel dB value from an FFmpeg XPSNR summary line; returns infinity for lossless `inf` or the parsed float; side effects: none.
fn extract_xpsnr_channel(line: &str, key: &str) -> Option<f64> {
    let idx = line.find(key)? + key.len();
    let token = line[idx..].split_whitespace().next()?;
    if token.eq_ignore_ascii_case("inf") {
        return Some(f64::INFINITY);
    }
    token.parse::<f64>().ok().filter(|val| val.is_finite())
}

// AI-FUNC-SUMMARY: Maps weighted XPSNR decibels monotonically onto VMAF-equivalent hundredths for uniform review thresholds; returns score in 5000..=10000; side effects: none.
fn xpsnr_db_to_vmaf_hundredths(db: f64) -> u32 {
    const KNOTS: [(f64, f64); 6] = [
        (28.0, 5000.0),
        (35.0, 8000.0),
        (40.0, 9000.0),
        (43.0, 9500.0),
        (46.0, 9750.0),
        (50.0, 9950.0),
    ];
    if db <= KNOTS[0].0 {
        return 5000;
    }
    if db >= KNOTS[KNOTS.len() - 1].0 {
        return 9950;
    }
    for pair in KNOTS.windows(2) {
        let (x0, y0) = pair[0];
        let (x1, y1) = pair[1];
        if db <= x1 {
            let t = (db - x0) / (x1 - x0);
            return (y0 + (y1 - y0) * t).round() as u32;
        }
    }
    9950
}

// AI-FUNC-SUMMARY: Chooses the VMAF model that matches the source resolution; returns the built-in model name; side effects: none.
pub fn model_for(video: &VideoInfo) -> &'static str {
    // Netflix ships a separate model trained for 4K viewing distances. Above
    // 1440p the 1080p model consistently reads high.
    if video.width.max(video.height) > 2560 || video.height > 1440 {
        "vmaf_4k_v0.6.1"
    } else {
        "vmaf_v0.6.1"
    }
}

// AI-FUNC-SUMMARY: Chooses the VMAF model matching resolution and HDR transfer capability, routing PQ (`smpte2084`) sources to `hdr_vmaf_v1.1.8` when available; returns the built-in model name; side effects: none.
pub fn model_for_with_support(video: &VideoInfo, hdr_v1_available: bool) -> &'static str {
    let is_pq = video
        .color_transfer
        .as_deref()
        .is_some_and(|t| t.eq_ignore_ascii_case("smpte2084"));
    if hdr_v1_available && is_pq {
        if video.width.max(video.height) > 2560 || video.height > 1440 {
            "hdr_vmaf_v1.1.8_1d5h_2160"
        } else {
            "hdr_vmaf_v1.1.8_3d0h"
        }
    } else {
        model_for(video)
    }
}

// AI-FUNC-SUMMARY: Reports whether HDR accuracy is handled natively or warrants a caveat sentence based on libvmaf capabilities; returns caveat or none; side effects: none.
pub fn accuracy_caveat_with_support(
    video: &VideoInfo,
    hdr_v1_available: bool,
) -> Option<&'static str> {
    let is_pq = video
        .color_transfer
        .as_deref()
        .is_some_and(|t| t.eq_ignore_ascii_case("smpte2084"));
    if hdr_v1_available && is_pq {
        None
    } else {
        accuracy_caveat(video)
    }
}

// AI-FUNC-SUMMARY: Chooses a pixel format both comparison inputs can be converted to; returns a planar YUV format libvmaf accepts; side effects: none.
pub fn comparison_pixel_format(video: &VideoInfo) -> &'static str {
    // libvmaf only accepts planar YUV at 8, 10, 12 or 16 bits. Anything else,
    // including RGB and the semi-planar formats hardware decoders produce, has
    // to be converted first, and both inputs must land on the same format.
    let source = video
        .pix_fmt
        .as_deref()
        .map(normalized_pix_fmt)
        .unwrap_or("yuv420p");
    let depth = pixel_format_bit_depth(source).unwrap_or(8);

    let family =
        if source.starts_with("yuv444") || source.starts_with("gbr") || source.contains("rgb") {
            "yuv444p"
        } else if source.starts_with("yuv422") {
            "yuv422p"
        } else {
            "yuv420p"
        };

    match (family, depth) {
        ("yuv444p", d) if d > 8 => "yuv444p10le",
        ("yuv422p", d) if d > 8 => "yuv422p10le",
        (_, d) if d > 8 => "yuv420p10le",
        ("yuv444p", _) => "yuv444p",
        ("yuv422p", _) => "yuv422p",
        _ => "yuv420p",
    }
}

// AI-FUNC-SUMMARY: Builds the ffmpeg arguments that compare a converted file against its source; returns the argument list; side effects: none.
#[allow(clippy::too_many_arguments)]
pub fn build_vmaf_args(
    distorted: &Path,
    reference: &Path,
    video: &VideoInfo,
    options: VmafOptions,
    progress: bool,
    // One method per input, because the two files are in different codecs: the
    // converted one is in the target codec, and hardware that decodes the
    // source may refuse what was just produced from it.
    accel: (Option<HwAccel>, Option<HwAccel>),
    hdr_v1_available: bool,
    log_path: Option<&Path>,
) -> Vec<String> {
    let format = comparison_pixel_format(video);
    let model = model_for_with_support(video, hdr_v1_available);
    let threads = if options.threads == 0 {
        std::thread::available_parallelism()
            .map(|value| value.get() as u32)
            .unwrap_or(1)
    } else {
        options.threads
    };
    let subsample = options.subsample.max(1);
    let log_suffix = vmaf_json_log_suffix(log_path);

    // Input 0 is the distorted video and input 1 the reference; the filter is
    // documented that way round and silently reports a different number if the
    // inputs are swapped. Resetting the timebase and start time on both keeps a
    // source that does not begin at zero from scoring far too low.
    let filter = format!(
        "[0:v:0]settb=AVTB,setpts=PTS-STARTPTS,format={format}[dist];\
         [1:v:0]settb=AVTB,setpts=PTS-STARTPTS,format={format}[ref];\
         [dist][ref]libvmaf=model=version={model}:n_threads={threads}:n_subsample={subsample}:shortest=1:ts_sync_mode=nearest{log_suffix}"
    );

    let mut args = vec![
        "-hide_banner".to_string(),
        "-nostdin".to_string(),
        // info is what makes libvmaf print its score line at all, and -nostats
        // only silences ffmpeg's own status line, which would otherwise be
        // mixed into the text the score is read from.
        "-loglevel".to_string(),
        "info".to_string(),
        "-nostats".to_string(),
    ];
    if progress {
        // -nostats does not suppress this: the two are separate switches, and
        // the null muxer opens no file, so stdout is free for the report.
        args.extend(PROGRESS_ARGS.iter().map(|flag| (*flag).to_string()));
    }
    // -hwaccel is a per-input option, so it goes before each of the two files.
    // What must never appear is -hwaccel_output_format: leaving it out is what
    // makes ffmpeg copy decoded frames back into ordinary memory, which is
    // where the filter and the format conversion below need them.
    let mut input = |path: &Path, accel: Option<HwAccel>| {
        if let Some(accel) = accel {
            args.push("-hwaccel".to_string());
            args.push(accel.as_str().to_string());
        }
        args.push("-i".to_string());
        args.push(path.to_string_lossy().to_string());
    };
    input(distorted, accel.0);
    input(reference, accel.1);

    args.extend([
        "-lavfi".to_string(),
        filter,
        "-f".to_string(),
        "null".to_string(),
        "-".to_string(),
    ]);
    args
}

// AI-FUNC-SUMMARY: Turns finished ffmpeg output into a score; returns the score or a user-facing error; side effects: none.
pub fn score_from_output(succeeded: bool, text: &str, subsample: u32) -> Result<VmafScore, String> {
    if !succeeded {
        return Err(format!(
            "quality measurement failed: {}",
            first_error_line(text)
        ));
    }

    parse_vmaf_score(text)
        .map(|hundredths| VmafScore {
            hundredths,
            subsample: subsample.max(1),
        })
        .ok_or_else(|| "quality measurement produced no score".to_string())
}

// AI-FUNC-SUMMARY: Reads the pooled score out of ffmpeg output; returns the score in hundredths or none when no score was printed; side effects: none.
pub fn parse_vmaf_score(text: &str) -> Option<u32> {
    // The filter prints one summary line: "VMAF score: 95.123456". Take the
    // last one so a retry inside the same output cannot be misread.
    let mut found = None;
    for line in text.lines() {
        let Some(position) = line.find("VMAF score:") else {
            continue;
        };
        let rest = line[position + "VMAF score:".len()..].trim();
        let digits: String = rest
            .chars()
            .take_while(|character| character.is_ascii_digit() || *character == '.')
            .collect();
        if let Ok(value) = digits.parse::<f64>()
            && value.is_finite()
            && (0.0..=100.0).contains(&value)
        {
            found = Some((value * 100.0).round() as u32);
        }
    }
    found
}

// AI-FUNC-SUMMARY: Reports whether a source is one VMAF cannot judge reliably; returns a caveat sentence or none; side effects: none.
pub fn accuracy_caveat(video: &VideoInfo) -> Option<&'static str> {
    let transfer = video.color_transfer.as_deref().unwrap_or("");
    if transfer.eq_ignore_ascii_case("smpte2084") || transfer.eq_ignore_ascii_case("arib-std-b67") {
        // The public VMAF models were trained on standard-range video.
        Some("this video is HDR, so the quality score is indicative only")
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // AI-FUNC-SUMMARY: Builds a source description for tests; returns the value with the given size and format; side effects: none.
    fn video(width: u32, height: u32, pix_fmt: &str) -> VideoInfo {
        VideoInfo {
            width,
            height,
            pix_fmt: Some(pix_fmt.to_string()),
            ..Default::default()
        }
    }

    #[test]
    // AI-FUNC-SUMMARY: Verifies the 4K model is chosen only above 1440p; returns nothing; side effects: none.
    fn picks_a_model_for_the_resolution() {
        assert_eq!(model_for(&video(1920, 1080, "yuv420p")), "vmaf_v0.6.1");
        assert_eq!(model_for(&video(2560, 1440, "yuv420p")), "vmaf_v0.6.1");
        assert_eq!(model_for(&video(3840, 2160, "yuv420p")), "vmaf_4k_v0.6.1");
    }

    #[test]
    // AI-FUNC-SUMMARY: Verifies every source format maps to one libvmaf accepts; returns nothing; side effects: none.
    fn picks_a_comparable_pixel_format() {
        assert_eq!(
            comparison_pixel_format(&video(1920, 1080, "yuv420p")),
            "yuv420p"
        );
        assert_eq!(
            comparison_pixel_format(&video(1920, 1080, "yuvj420p")),
            "yuv420p"
        );
        assert_eq!(
            comparison_pixel_format(&video(1920, 1080, "yuv422p10le")),
            "yuv422p10le"
        );
        assert_eq!(
            comparison_pixel_format(&video(1920, 1080, "yuv444p12le")),
            "yuv444p10le"
        );
        assert_eq!(
            comparison_pixel_format(&video(1920, 1080, "gbrp")),
            "yuv444p"
        );
        assert_eq!(
            comparison_pixel_format(&video(1920, 1080, "nv12")),
            "yuv420p"
        );
    }

    #[test]
    // AI-FUNC-SUMMARY: Verifies the comparison command puts the distorted file first and asks for no hardware decoding; returns nothing; side effects: none.
    fn builds_a_comparison_command() {
        let args = build_vmaf_args(
            Path::new("/tmp/out.mp4"),
            Path::new("/videos/clip.mkv"),
            &video(1920, 1080, "yuv420p"),
            VmafOptions {
                subsample: 5,
                threads: 4,
            },
            false,
            (None, None),
            false,
            None,
        );

        let joined = args.join(" ");
        let distorted = args.iter().position(|arg| arg == "/tmp/out.mp4").unwrap();
        let reference = args
            .iter()
            .position(|arg| arg == "/videos/clip.mkv")
            .unwrap();

        assert!(distorted < reference);
        assert!(joined.contains("n_threads=4"));
        assert!(joined.contains("n_subsample=5"));
        assert!(joined.contains("model=version=vmaf_v0.6.1"));
        assert!(joined.contains("setpts=PTS-STARTPTS"));
        assert!(joined.contains("[dist][ref]libvmaf"));
        assert!(!joined.contains("-hwaccel"));
        assert!(!joined.contains("-progress"));
        assert!(joined.ends_with("-f null -"));
    }

    #[test]
    // AI-FUNC-SUMMARY: Verifies a measurement can report its progress without losing the switches its score depends on; returns nothing; side effects: none.
    fn asks_for_progress_when_requested() {
        let args = build_vmaf_args(
            Path::new("/tmp/out.mp4"),
            Path::new("/videos/clip.mkv"),
            &video(1920, 1080, "yuv420p"),
            VmafOptions::default(),
            true,
            (None, None),
            false,
            None,
        );

        let joined = args.join(" ");
        assert!(joined.contains("-progress pipe:1"));
        assert!(joined.contains("-stats_period 1"));
        // The score is printed at info level, and -nostats silences only
        // ffmpeg's own status line, not the progress report.
        assert!(joined.contains("-loglevel info"));
        assert!(joined.contains("-nostats"));

        let progress = args.iter().position(|arg| arg == "-progress").unwrap();
        let first_input = args.iter().position(|arg| arg == "-i").unwrap();
        assert!(progress < first_input);
    }

    #[test]
    // AI-FUNC-SUMMARY: Verifies finished output is turned into a score or a plain error; returns nothing; side effects: none.
    fn reads_a_score_out_of_finished_output() {
        let measured = score_from_output(true, "VMAF score: 95.50\n", 5).unwrap();
        assert_eq!(measured.hundredths, 9550);
        assert_eq!(measured.subsample, 5);

        let failed = score_from_output(false, "Error opening input file\n", 1).unwrap_err();
        assert!(failed.starts_with("quality measurement failed:"));

        let scoreless = score_from_output(true, "nothing useful\n", 1).unwrap_err();
        assert_eq!(scoreless, "quality measurement produced no score");
    }

    #[test]
    // AI-FUNC-SUMMARY: Verifies each input gets its own decoder and that frames still come back to ordinary memory; returns nothing; side effects: none.
    fn puts_a_decoder_before_each_input() {
        let args = build_vmaf_args(
            Path::new("/tmp/out.mp4"),
            Path::new("/videos/clip.mkv"),
            &video(1920, 1080, "yuv420p"),
            VmafOptions::default(),
            false,
            (None, Some(HwAccel::D3d11va)),
            false,
            None,
        );

        let joined = args.join(" ");
        // Without this the filter receives frames the graphics card still owns,
        // and the comparison fails outright.
        assert!(!joined.contains("-hwaccel_output_format"));
        assert_eq!(args.iter().filter(|arg| *arg == "-hwaccel").count(), 1);

        let accel = args.iter().position(|arg| arg == "-hwaccel").unwrap();
        let reference = args
            .iter()
            .position(|arg| arg == "/videos/clip.mkv")
            .unwrap();
        assert_eq!(accel + 2, reference - 1);

        let both = build_vmaf_args(
            Path::new("/tmp/out.mp4"),
            Path::new("/videos/clip.mkv"),
            &video(1920, 1080, "yuv420p"),
            VmafOptions::default(),
            false,
            (Some(HwAccel::Cuda), Some(HwAccel::Cuda)),
            false,
            None,
        );
        assert_eq!(both.iter().filter(|arg| *arg == "-hwaccel").count(), 2);
    }

    #[test]
    // AI-FUNC-SUMMARY: Verifies the card-only comparison keeps its frames on the card and converts them there; returns nothing; side effects: none.
    fn builds_a_comparison_that_stays_on_the_card() {
        let args = build_vmaf_cuda_args(
            Path::new("/tmp/out.mp4"),
            Path::new("/videos/clip.mkv"),
            &video(1920, 1080, "yuv420p"),
            VmafOptions::default(),
            false,
        );

        let joined = args.join(" ");
        // The opposite of the processor path: here the frames must stay put.
        assert_eq!(
            args.iter()
                .filter(|arg| *arg == "-hwaccel_output_format")
                .count(),
            2
        );
        assert!(joined.contains("scale_cuda=format=yuv420p"));
        assert!(joined.contains("[dist][ref]libvmaf_cuda"));
        // Worker threads are a processor idea; the card has none.
        assert!(!joined.contains("n_threads"));
        assert!(joined.contains("n_subsample=5"));
        assert!(joined.ends_with("-f null -"));
    }

    #[test]
    // AI-FUNC-SUMMARY: Verifies only eight-bit 4:2:0 is offered to the card, which is all its filter takes; returns nothing; side effects: none.
    fn only_eight_bit_four_two_zero_goes_to_the_card() {
        assert!(can_score_on_gpu(&video(1920, 1080, "yuv420p")));
        assert!(!can_score_on_gpu(&video(1920, 1080, "yuv420p10le")));
        assert!(!can_score_on_gpu(&video(1920, 1080, "yuv444p")));
    }

    #[test]
    // AI-FUNC-SUMMARY: Verifies the comparison itself does not depend on how the frames were decoded; returns nothing; side effects: none.
    fn the_filter_is_the_same_either_way() {
        let filter_of = |accel| {
            let args = build_vmaf_args(
                Path::new("/tmp/out.mp4"),
                Path::new("/videos/clip.mkv"),
                &video(3840, 2160, "yuv420p10le"),
                VmafOptions::default(),
                false,
                accel,
                false,
                None,
            );
            let index = args.iter().position(|arg| arg == "-lavfi").unwrap();
            args[index + 1].clone()
        };

        assert_eq!(
            filter_of((None, None)),
            filter_of((Some(HwAccel::D3d11va), Some(HwAccel::Cuda)))
        );
    }

    #[test]
    // AI-FUNC-SUMMARY: Verifies score parsing accepts real output and rejects noise; returns nothing; side effects: none.
    fn parses_a_score() {
        assert_eq!(
            parse_vmaf_score("[libvmaf @ 0x55] VMAF score: 95.123456\n"),
            Some(9512)
        );
        assert_eq!(parse_vmaf_score("VMAF score: 100.000000"), Some(10000));
        assert_eq!(parse_vmaf_score("VMAF score: 0.000000"), Some(0));
        assert_eq!(parse_vmaf_score("no score here"), None);
        assert_eq!(parse_vmaf_score("VMAF score: nonsense"), None);
    }

    #[test]
    // AI-FUNC-SUMMARY: Verifies the last score wins when output holds several; returns nothing; side effects: none.
    fn takes_the_last_score() {
        let text = "VMAF score: 80.000000\nVMAF score: 92.500000\n";
        assert_eq!(parse_vmaf_score(text), Some(9250));
    }

    #[test]
    // AI-FUNC-SUMMARY: Verifies HDR sources carry an accuracy caveat and others do not; returns nothing; side effects: none.
    fn flags_hdr_sources() {
        let mut hdr = video(3840, 2160, "yuv420p10le");
        hdr.color_transfer = Some("smpte2084".to_string());
        assert!(accuracy_caveat(&hdr).is_some());

        let sdr = video(1920, 1080, "yuv420p");
        assert!(accuracy_caveat(&sdr).is_none());
    }

    #[test]
    // AI-FUNC-SUMMARY: Verifies a score converts back to VMAF points; returns nothing; side effects: none.
    fn reports_points() {
        let score = VmafScore {
            hundredths: 9512,
            subsample: 1,
        };
        assert!((score.points() - 95.12).abs() < 0.001);
        let _ = PathBuf::new();
    }

    #[test]
    // AI-FUNC-SUMMARY: Verifies segmented quality planning skips short clips and spreads windows through long files; returns nothing; side effects: none.
    fn plans_segmented_windows() {
        assert_eq!(segmented_plan(30.0), None);
        let medium = segmented_plan(300.0).unwrap();
        assert_eq!(medium.len(), 3);
        assert!(medium[0].start_seconds >= 30.0);
        assert!(
            medium
                .windows(2)
                .all(|w| w[0].start_seconds < w[1].start_seconds)
        );

        let long = segmented_plan(3600.0).unwrap();
        assert_eq!(long.len(), MAX_SEGMENT_WINDOWS);
    }

    #[test]
    // AI-FUNC-SUMMARY: Verifies segmented VMAF and XPSNR commands put hybrid seek flags, trim alignment, and 10-bit portrait guards cleanly; returns nothing; side effects: none.
    fn builds_segmented_and_xpsnr_commands() {
        let win = SegmentWindow {
            start_seconds: 120.0,
            duration_seconds: 6.0,
            pool: SegmentPool::Stratified,
        };
        let vmaf_args = build_segmented_vmaf_args(
            Path::new("/tmp/out.mp4"),
            Path::new("/videos/in.mp4"),
            &video(1920, 1080, "yuv420p"),
            VmafOptions {
                subsample: 3,
                threads: 4,
            },
            win,
            false,
            (None, None),
            false,
            Some(Path::new("C:\\tmp\\vmaf.json")),
        );
        let vmaf_joined = vmaf_args.join(" ");
        assert_eq!(vmaf_args.iter().filter(|a| *a == "-ss").count(), 2);
        assert_eq!(vmaf_args.iter().filter(|a| *a == "-t").count(), 2);
        assert!(vmaf_joined.contains("-ss 120.000 -t 6.000 -i /tmp/out.mp4"));
        assert!(vmaf_joined.contains("-ss 115.000 -t 12.000 -i /videos/in.mp4"));
        assert!(vmaf_joined.contains("trim=start=5.000:duration=6.000"));
        assert!(vmaf_joined.contains("n_subsample=3"));
        assert!(vmaf_joined.contains(":log_fmt=json:log_path=C\\:/tmp/vmaf.json"));

        let xpsnr_args = build_xpsnr_args(
            Path::new("/tmp/out.mp4"),
            Path::new("/videos/in.mp4"),
            &video(1920, 1080, "yuv420p10le"),
            Some(win),
            true,
            (None, None),
        );
        let joined = xpsnr_args.join(" ");
        assert!(joined.contains("format=yuv420p10le[dist]"));
        assert!(!joined.contains("transpose=1"));
        assert!(joined.contains("trim=start=5.000:duration=6.000"));
        assert!(joined.contains("[dist][ref]xpsnr"));
        assert!(joined.contains("-progress pipe:1"));

        let portrait_xpsnr = build_xpsnr_args(
            Path::new("/tmp/out.mp4"),
            Path::new("/videos/in.mp4"),
            &video(1080, 1920, "yuv420p10le"),
            None,
            false,
            (None, None),
        );
        let portrait_joined = portrait_xpsnr.join(" ");
        assert!(portrait_joined.contains("format=yuv420p10le,transpose=1[dist]"));
        assert!(portrait_joined.contains("format=yuv420p10le,transpose=1[ref]"));
    }

    #[test]
    // AI-FUNC-SUMMARY: Verifies keyframe CSV parsing and segment window alignment to distorted keyframes; returns nothing; side effects: none.
    fn parses_and_aligns_keyframes() {
        let parsed = parse_keyframe_timestamps("12.500\n0.000,\nN/A\n5.000\n5.00001\n-1.0\n");
        assert_eq!(parsed, vec![0.0, 5.0, 12.5]);

        let windows = vec![
            SegmentWindow {
                start_seconds: 10.0,
                duration_seconds: 6.0,
                pool: SegmentPool::Stratified,
            },
            SegmentWindow {
                start_seconds: 12.3,
                duration_seconds: 6.0,
                pool: SegmentPool::Stratified,
            },
            SegmentWindow {
                start_seconds: 50.0,
                duration_seconds: 6.0,
                pool: SegmentPool::Stratified,
            },
        ];
        let aligned = align_windows_to_keyframes(&windows, &[0.0, 11.8, 48.0, 96.0], 100.0);
        assert_eq!(aligned.len(), 2);
        assert!((aligned[0].start_seconds - 11.8).abs() < 1e-6);
        assert!((aligned[1].start_seconds - 48.0).abs() < 1e-6);
        assert_eq!(align_windows_to_keyframes(&windows, &[], 100.0), windows);
    }

    #[test]
    // AI-FUNC-SUMMARY: Verifies XPSNR parsing maps dB values monotonically and handles inf; returns nothing; side effects: none.
    fn parses_xpsnr_scores() {
        assert_eq!(parse_xpsnr_score("no xpsnr"), None);
        assert_eq!(
            parse_xpsnr_score("[Parsed_xpsnr_4] XPSNR  y: inf  u: inf  v: inf  (minimum: inf)"),
            Some(10000)
        );
        let low = parse_xpsnr_score("XPSNR  y: 35.0000  u: 35.0000  v: 35.0000").unwrap();
        let target = parse_xpsnr_score("XPSNR  y: 43.0000  u: 43.0000  v: 43.0000").unwrap();
        let high = parse_xpsnr_score("XPSNR  y: 48.0000  u: 48.0000  v: 48.0000").unwrap();
        assert_eq!(low, 8000);
        assert_eq!(target, 9500);
        assert!(high > target);
    }

    #[test]
    // AI-FUNC-SUMMARY: Verifies packet header CSV parsing and Dual-Pool stratified/risk planning without overlap; returns nothing; side effects: none.
    fn parses_packet_samples_and_builds_dual_pool_plan() {
        let parsed =
            parse_packet_samples("0.000,12000,K__\nN/A,500,___\n-1.0,200,___\n0.500,4500,___\n");
        assert_eq!(parsed.len(), 2);
        assert!(parsed[0].is_keyframe);
        assert!(!parsed[1].is_keyframe);
        assert_eq!(parsed[1].size_bytes, 4500);

        // Construct 180s (~15 windows in [9s, 165s]) of synthetic packet pairs.
        let mut src_pkts = Vec::new();
        let mut dist_pkts = Vec::new();
        for idx in 0..30 {
            let pts = idx as f64 * 6.0 + 1.0;
            let src_size = 50_000 + (idx as u64 % 10) * 10_000;
            // Deliberately inject one extreme starvation window and one high spike.
            let (src_size, dist_size) = if idx == 6 {
                (120_000, 12_000) // lowest ratio R(t) = 0.10
            } else if idx == 20 {
                (500_000, 220_000) // highest source spike
            } else {
                (src_size, src_size / 2)
            };
            src_pkts.push(PacketSample {
                pts_seconds: pts,
                size_bytes: src_size,
                is_keyframe: true,
            });
            dist_pkts.push(PacketSample {
                pts_seconds: pts,
                size_bytes: dist_size,
                is_keyframe: true,
            });
        }

        assert_eq!(dual_pool_plan(&src_pkts, &dist_pkts, 80.0), None);
        let plan = dual_pool_plan(&src_pkts, &dist_pkts, 180.0).unwrap();
        let pool_a: Vec<_> = plan
            .iter()
            .filter(|w| w.pool == SegmentPool::Stratified)
            .collect();
        let pool_b: Vec<_> = plan
            .iter()
            .filter(|w| w.pool == SegmentPool::Risk)
            .collect();
        assert!(!pool_a.is_empty() && pool_a.len() <= 6);
        assert!(!pool_b.is_empty() && pool_b.len() <= 4);
        assert!(
            plan.windows(2)
                .all(|pair| pair[0].start_seconds < pair[1].start_seconds)
        );
    }

    #[test]
    // AI-FUNC-SUMMARY: Verifies libvmaf JSON frame extraction and collapse summary metrics (harmonic mean, P05, min, worst frame index); returns nothing; side effects: none.
    fn parses_vmaf_json_and_summarizes_collapse() {
        let mut frames_json = String::from("{\"version\":\"3.2.1\",\"frames\":[");
        for idx in 0..20 {
            let score = if idx == 13 { 30.0 } else { 96.0 };
            if idx > 0 {
                frames_json.push(',');
            }
            frames_json.push_str(&format!(
                "{{\"frameNum\":{idx},\"metrics\":{{\"vmaf\":{score:.2}}}}}"
            ));
        }
        frames_json.push_str("]}");

        let parsed = parse_vmaf_json_frames(&frames_json);
        assert_eq!(parsed.len(), 20);
        let stats = summarize_frame_scores(&parsed).unwrap();
        assert_eq!(stats.frame_count, 20);
        assert_eq!(stats.worst_frame_index, 13);
        assert_eq!(stats.min_hundredths, 3000);
        assert_eq!(stats.p05_hundredths, 3000);
        assert!(stats.mean_hundredths > 9000);
        assert!(stats.harmonic_mean_hundredths < stats.mean_hundredths - 500);

        let v1_json = r#"{"frames":[{"frameNum":4,"metrics":{"vmaf_v1":94.25}}]}"#;
        assert_eq!(parse_vmaf_json_frames(v1_json), vec![(4, 94.25)]);
    }

    #[test]
    // AI-FUNC-SUMMARY: Verifies filter path escaping handles Windows backslashes, colons, and separators; returns nothing; side effects: none.
    fn escapes_windows_and_posix_filter_paths() {
        assert_eq!(
            escape_filter_path(Path::new("C:\\tmp\\a.json")),
            "C\\:/tmp/a.json"
        );
        assert_eq!(
            escape_filter_path(Path::new("/var/tmp/clip,1.json")),
            "/var/tmp/clip\\,1.json"
        );
    }

    #[test]
    // AI-FUNC-SUMMARY: Verifies HDR PQ sources route to `hdr_vmaf_v1.1.8` models and drop SDR caveats only when supported; returns nothing; side effects: none.
    fn routes_hdr_pq_to_v1_when_supported() {
        let sdr = video(1920, 1080, "yuv420p");
        assert_eq!(model_for_with_support(&sdr, true), "vmaf_v0.6.1");
        assert_eq!(accuracy_caveat_with_support(&sdr, true), None);

        let mut pq_1080 = video(1920, 1080, "yuv420p10le");
        pq_1080.color_transfer = Some("smpte2084".to_string());
        assert_eq!(model_for_with_support(&pq_1080, false), "vmaf_v0.6.1");
        assert!(accuracy_caveat_with_support(&pq_1080, false).is_some());
        assert_eq!(
            model_for_with_support(&pq_1080, true),
            "hdr_vmaf_v1.1.8_3d0h"
        );
        assert_eq!(accuracy_caveat_with_support(&pq_1080, true), None);

        let mut pq_4k = video(3840, 2160, "yuv420p10le");
        pq_4k.color_transfer = Some("smpte2084".to_string());
        assert_eq!(
            model_for_with_support(&pq_4k, true),
            "hdr_vmaf_v1.1.8_1d5h_2160"
        );

        let mut hlg = video(3840, 2160, "yuv420p10le");
        hlg.color_transfer = Some("arib-std-b67".to_string());
        assert_eq!(model_for_with_support(&hlg, true), "vmaf_4k_v0.6.1");
        assert!(accuracy_caveat_with_support(&hlg, true).is_some());
    }
}
