//! Preview frame and comparison-clip generation for pending reviews.
//!
//! All FFmpeg and FFprobe invocations go through `super::process_command` so
//! Windows applies `CREATE_NO_WINDOW` and never flashes a console window.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use super::{TargetCodec, codec, first_error_line, json_string, probe_video, process_command};

/// Basic media dimensions and duration for comparison scrubbing.
#[derive(Debug, Clone, PartialEq)]
pub struct PreviewInfo {
    pub duration_seconds: f64,
    pub width: u32,
    pub height: u32,
    pub fps: f64,
}

/// Paths produced by extracting one aligned pair of comparison frames.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FramePairResult {
    pub original_image: PathBuf,
    pub review_image: PathBuf,
    pub diff_image: Option<PathBuf>,
}

/// Synchronized JPEG frame sequence produced for live in-app comparison playback.
#[derive(Debug, Clone, PartialEq)]
pub struct ClipSequenceResult {
    pub original_frames: Vec<PathBuf>,
    pub review_frames: Vec<PathBuf>,
    pub fps: u32,
}

/// Parameters controlling synchronized clip-sequence extraction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClipSequenceOptions {
    pub start_ms: u64,
    pub duration_ms: u64,
    pub fps: u32,
    pub max_height: u32,
}

// AI-FUNC-SUMMARY: Probes duration, resolution, and frame rate for review preview scrubbing; returns metadata or a user-facing error; side effects: runs ffprobe.
pub fn probe_preview_info(ffprobe: &str, path: &Path) -> Result<PreviewInfo, String> {
    let info = probe_video(ffprobe, path)?
        .ok_or_else(|| format!("no readable video stream in {}", path.to_string_lossy()))?;
    Ok(PreviewInfo {
        duration_seconds: info.duration_seconds,
        width: info.width,
        height: info.height,
        fps: if info.fps > 0.0 { info.fps } else { 24.0 },
    })
}

// AI-FUNC-SUMMARY: Serializes preview metadata as JSON for FFI callers; returns JSON text; side effects: none.
pub fn preview_info_json(info: &PreviewInfo) -> String {
    format!(
        "{{\"duration_seconds\":{:.3},\"width\":{},\"height\":{},\"fps\":{:.3}}}",
        if info.duration_seconds.is_finite() {
            info.duration_seconds
        } else {
            0.0
        },
        info.width,
        info.height,
        if info.fps.is_finite() { info.fps } else { 24.0 }
    )
}

// AI-FUNC-SUMMARY: Builds the ffmpeg arguments to extract one aligned PNG preview frame; returns the argument list; side effects: none.
pub fn build_single_frame_args(input: &Path, timestamp_seconds: f64, output: &Path) -> Vec<String> {
    vec![
        "-hide_banner".to_string(),
        "-nostdin".to_string(),
        "-loglevel".to_string(),
        "error".to_string(),
        "-y".to_string(),
        "-ss".to_string(),
        format!("{:.3}", timestamp_seconds.max(0.0)),
        "-i".to_string(),
        input.to_string_lossy().to_string(),
        "-frames:v".to_string(),
        "1".to_string(),
        "-vf".to_string(),
        "format=rgb24".to_string(),
        output.to_string_lossy().to_string(),
    ]
}

// AI-FUNC-SUMMARY: Builds the ffmpeg arguments to compute an amplified visual difference heatmap from two aligned PNG frames; returns the argument list; side effects: none.
pub fn build_diff_heatmap_args(
    original_png: &Path,
    review_png: &Path,
    output: &Path,
) -> Vec<String> {
    vec![
        "-hide_banner".to_string(),
        "-nostdin".to_string(),
        "-loglevel".to_string(),
        "error".to_string(),
        "-y".to_string(),
        "-i".to_string(),
        original_png.to_string_lossy().to_string(),
        "-i".to_string(),
        review_png.to_string_lossy().to_string(),
        "-lavfi".to_string(),
        "[0:v:0]format=gbrp[orig];[1:v:0]format=gbrp[rev];[orig][rev]blend=all_mode=difference,eq=contrast=4.0:brightness=0.06,format=rgb24".to_string(),
        "-frames:v".to_string(),
        "1".to_string(),
        output.to_string_lossy().to_string(),
    ]
}

// AI-FUNC-SUMMARY: Extracts aligned PNG frames from both original and converted files at a timestamp, plus an optional difference heatmap; returns the output image paths or an error; side effects: creates output directory and runs ffmpeg.
pub fn extract_frame_pair(
    ffmpeg: &str,
    original: &Path,
    review: &Path,
    timestamp_ms: u64,
    out_dir: &Path,
    include_diff: bool,
) -> Result<FramePairResult, String> {
    fs::create_dir_all(out_dir).map_err(|err| {
        format!(
            "failed to create preview folder {}: {err}",
            out_dir.to_string_lossy()
        )
    })?;

    let seconds = timestamp_ms as f64 / 1000.0;
    let orig_png = out_dir.join("original.png");
    let rev_png = out_dir.join("review.png");

    run_ffmpeg_quiet(
        ffmpeg,
        &build_single_frame_args(original, seconds, &orig_png),
    )?;
    run_ffmpeg_quiet(ffmpeg, &build_single_frame_args(review, seconds, &rev_png))?;

    let diff_image = if include_diff {
        let diff_png = out_dir.join("diff.png");
        run_ffmpeg_quiet(
            ffmpeg,
            &build_diff_heatmap_args(&orig_png, &rev_png, &diff_png),
        )?;
        Some(diff_png)
    } else {
        None
    };

    Ok(FramePairResult {
        original_image: orig_png,
        review_image: rev_png,
        diff_image,
    })
}

// AI-FUNC-SUMMARY: Serializes extracted frame pair paths as JSON; returns JSON text; side effects: none.
pub fn frame_pair_json(pair: &FramePairResult) -> String {
    let diff = pair
        .diff_image
        .as_ref()
        .map(|p| json_string(&p.to_string_lossy()))
        .unwrap_or_else(|| "null".to_string());
    format!(
        "{{\"original_image\":{},\"review_image\":{},\"diff_image\":{}}}",
        json_string(&pair.original_image.to_string_lossy()),
        json_string(&pair.review_image.to_string_lossy()),
        diff
    )
}

// AI-FUNC-SUMMARY: Builds the ffmpeg arguments to extract a short synchronized JPEG frame sequence for live in-app comparison playback; returns the argument list; side effects: none.
pub fn build_clip_frames_args(
    input: &Path,
    start_seconds: f64,
    duration_seconds: f64,
    fps: u32,
    max_height: u32,
    pattern: &Path,
) -> Vec<String> {
    let fps = fps.clamp(5, 30);
    let max_height = max_height.clamp(240, 1080);
    vec![
        "-hide_banner".to_string(),
        "-nostdin".to_string(),
        "-loglevel".to_string(),
        "error".to_string(),
        "-y".to_string(),
        "-ss".to_string(),
        format!("{:.3}", start_seconds.max(0.0)),
        "-t".to_string(),
        format!("{:.3}", duration_seconds.clamp(1.0, 15.0)),
        "-i".to_string(),
        input.to_string_lossy().to_string(),
        "-an".to_string(),
        "-vf".to_string(),
        format!("fps={fps},scale=-2:'min(ih,{max_height})',format=yuvj420p"),
        "-q:v".to_string(),
        "4".to_string(),
        pattern.to_string_lossy().to_string(),
    ]
}

// AI-FUNC-SUMMARY: Extracts synchronized JPEG frame sequences from original and converted videos for live in-app playback; returns paired frame paths or an error; side effects: writes JPEG files into out_dir and runs ffmpeg twice.
pub fn extract_clip_sequence(
    ffmpeg: &str,
    original: &Path,
    review: &Path,
    options: ClipSequenceOptions,
    out_dir: &Path,
) -> Result<ClipSequenceResult, String> {
    fs::create_dir_all(out_dir).map_err(|err| {
        format!(
            "failed to create clip sequence folder {}: {err}",
            out_dir.to_string_lossy()
        )
    })?;
    clear_frame_files(out_dir);

    let start_s = options.start_ms as f64 / 1000.0;
    let dur_s = (options.duration_ms as f64 / 1000.0).clamp(1.0, 15.0);
    let fps = options.fps.clamp(5, 30);

    let orig_pattern = out_dir.join("orig_%04d.jpg");
    let rev_pattern = out_dir.join("rev_%04d.jpg");

    run_ffmpeg_quiet(
        ffmpeg,
        &build_clip_frames_args(
            original,
            start_s,
            dur_s,
            fps,
            options.max_height,
            &orig_pattern,
        ),
    )?;
    run_ffmpeg_quiet(
        ffmpeg,
        &build_clip_frames_args(
            review,
            start_s,
            dur_s,
            fps,
            options.max_height,
            &rev_pattern,
        ),
    )?;

    let mut orig_frames = collect_prefixed_jpegs(out_dir, "orig_");
    let mut rev_frames = collect_prefixed_jpegs(out_dir, "rev_");
    let paired = orig_frames.len().min(rev_frames.len());
    if paired == 0 {
        return Err("could not extract synchronized preview frames".to_string());
    }
    orig_frames.truncate(paired);
    rev_frames.truncate(paired);

    Ok(ClipSequenceResult {
        original_frames: orig_frames,
        review_frames: rev_frames,
        fps,
    })
}

// AI-FUNC-SUMMARY: Serializes a synchronized clip sequence as JSON for FFI callers; returns JSON text; side effects: none.
pub fn clip_sequence_json(seq: &ClipSequenceResult) -> String {
    let encode_list = |items: &[PathBuf]| {
        items
            .iter()
            .map(|p| json_string(&p.to_string_lossy()))
            .collect::<Vec<_>>()
            .join(",")
    };
    format!(
        "{{\"fps\":{},\"original_frames\":[{}],\"review_frames\":[{}]}}",
        seq.fps,
        encode_list(&seq.original_frames),
        encode_list(&seq.review_frames)
    )
}

// AI-FUNC-SUMMARY: Exports a short side-by-side MP4 comparison clip for external playback; returns the output path or an error; side effects: spawns ffmpeg and writes out_path.
pub fn export_comparison_clip(
    ffmpeg: &str,
    original: &Path,
    review: &Path,
    start_ms: u64,
    duration_ms: u64,
    out_path: &Path,
) -> Result<PathBuf, String> {
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            format!(
                "failed to create export folder {}: {err}",
                parent.to_string_lossy()
            )
        })?;
    }

    let start_s = format!("{:.3}", start_ms as f64 / 1000.0);
    let dur_s = format!("{:.3}", (duration_ms as f64 / 1000.0).clamp(1.0, 20.0));
    let filter = "[0:v:0]settb=AVTB,setpts=PTS-STARTPTS,scale=-2:'min(ih,720)',format=yuv420p[orig];\
                  [1:v:0]settb=AVTB,setpts=PTS-STARTPTS,format=yuv420p[rev];\
                  [rev][orig]scale2ref=w=iw:h=ih[rev_s][orig_s];\
                  [orig_s][rev_s]hstack=inputs=2[out]";

    let build = |encoder_args: &[&str]| -> Vec<String> {
        let mut args = vec![
            "-hide_banner".to_string(),
            "-nostdin".to_string(),
            "-loglevel".to_string(),
            "error".to_string(),
            "-y".to_string(),
            "-ss".to_string(),
            start_s.clone(),
            "-t".to_string(),
            dur_s.clone(),
            "-i".to_string(),
            original.to_string_lossy().to_string(),
            "-ss".to_string(),
            start_s.clone(),
            "-t".to_string(),
            dur_s.clone(),
            "-i".to_string(),
            review.to_string_lossy().to_string(),
            "-lavfi".to_string(),
            filter.to_string(),
            "-map".to_string(),
            "[out]".to_string(),
            "-an".to_string(),
        ];
        args.extend(encoder_args.iter().map(|s| (*s).to_string()));
        args.extend([
            "-movflags".to_string(),
            "+faststart".to_string(),
            out_path.to_string_lossy().to_string(),
        ]);
        args
    };

    // Prefer H.264 for universal player support, falling back to MPEG-4 if the
    // FFmpeg build has no GPL x264 encoder.
    let first = run_ffmpeg_quiet(
        ffmpeg,
        &build(&["-c:v", "libx264", "-crf", "20", "-preset", "veryfast"]),
    );
    if first.is_err() {
        run_ffmpeg_quiet(ffmpeg, &build(&["-c:v", "mpeg4", "-q:v", "4"]))?;
    }

    Ok(out_path.to_path_buf())
}

// AI-FUNC-SUMMARY: Lists runtime-verified encoders for a target codec as JSON; returns JSON document; side effects: runs ffmpeg encoder probes.
pub fn list_usable_encoders_json(ffmpeg: &str, target_codec: TargetCodec) -> String {
    let output = process_command(ffmpeg)
        .args(["-hide_banner", "-encoders"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    let Ok(output) = output else {
        return "{\"encoders\":[]}".to_string();
    };
    let text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let mut usable = Vec::new();
    for (name, kind) in codec::encoder_candidates(target_codec) {
        if !super::encoder_list_contains(&text, name) {
            continue;
        }
        let encoder = super::Encoder {
            name: (*name).to_string(),
            kind: *kind,
        };
        if super::check_encoder_runtime(ffmpeg, &encoder).is_ok() {
            usable.push(json_string(name));
        }
    }
    format!("{{\"encoders\":[{}]}}", usable.join(","))
}

// AI-FUNC-SUMMARY: Runs a short non-streaming ffmpeg command through the shared hidden-window launcher; returns success or the first error line; side effects: spawns ffmpeg.
fn run_ffmpeg_quiet(ffmpeg: &str, args: &[String]) -> Result<(), String> {
    let output = process_command(ffmpeg)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|err| format!("failed to start ffmpeg: {err}"))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!(
            "ffmpeg preview failed: {}",
            first_error_line(&stderr)
        ))
    }
}

// AI-FUNC-SUMMARY: Removes previously generated preview JPEG frames from a folder; returns nothing; side effects: deletes files matching orig_/rev_ JPEG names.
fn clear_frame_files(dir: &Path) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if (name.starts_with("orig_") || name.starts_with("rev_")) && name.ends_with(".jpg") {
            let _ = fs::remove_file(entry.path());
        }
    }
}

// AI-FUNC-SUMMARY: Collects sorted JPEG frame files matching a prefix in a directory; returns sorted paths; side effects: reads directory entries.
fn collect_prefixed_jpegs(dir: &Path, prefix: &str) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut paths: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            let name = path.file_name()?.to_str()?;
            (name.starts_with(prefix) && name.ends_with(".jpg")).then_some(path)
        })
        .collect();
    paths.sort();
    paths
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    // AI-FUNC-SUMMARY: Verifies preview frame and clip sequence argument builders include expected seeking and filter flags; returns nothing; side effects: none.
    fn builds_preview_commands() {
        let frame_args = build_single_frame_args(
            Path::new("/videos/clip.mp4"),
            12.345,
            Path::new("/tmp/frame.png"),
        );
        let joined = frame_args.join(" ");
        assert!(joined.contains("-ss 12.345"));
        assert!(joined.contains("-frames:v 1"));
        assert!(joined.contains("format=rgb24"));

        let seq_args = build_clip_frames_args(
            Path::new("/videos/clip.mp4"),
            5.0,
            5.0,
            15,
            720,
            Path::new("/tmp/orig_%04d.jpg"),
        );
        let seq_joined = seq_args.join(" ");
        assert!(seq_joined.contains("fps=15"));
        assert!(seq_joined.contains("min(ih,720)"));
        assert!(seq_joined.contains("-an"));
    }

    #[test]
    // AI-FUNC-SUMMARY: Verifies preview JSON serializers format paths and scalars cleanly; returns nothing; side effects: none.
    fn serializes_preview_json() {
        let info = PreviewInfo {
            duration_seconds: 95.5,
            width: 1920,
            height: 1080,
            fps: 23.976,
        };
        let info_text = preview_info_json(&info);
        assert!(info_text.contains("\"duration_seconds\":95.500"));
        assert!(info_text.contains("\"width\":1920"));

        let pair = FramePairResult {
            original_image: PathBuf::from("/tmp/orig.png"),
            review_image: PathBuf::from("/tmp/rev.png"),
            diff_image: None,
        };
        let pair_text = frame_pair_json(&pair);
        assert!(pair_text.contains("\"diff_image\":null"));
    }
}
