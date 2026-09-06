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
    VideoInfo, first_error_line, normalized_pix_fmt, pixel_format_bit_depth, process_command,
};

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
}

// AI-FUNC-SUMMARY: Checks whether the FFmpeg build can measure quality; returns support with a reason when unavailable; side effects: runs ffmpeg once to list the filter.
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
                (VmafSupport { available: true }, None)
            } else {
                (
                    VmafSupport { available: false },
                    Some(
                        "this FFmpeg build has no libvmaf filter, so quality cannot be measured"
                            .to_string(),
                    ),
                )
            }
        }
        Err(err) => (
            VmafSupport { available: false },
            Some(format!(
                "failed to ask ffmpeg about the libvmaf filter: {err}"
            )),
        ),
    }
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
pub fn build_vmaf_args(
    distorted: &Path,
    reference: &Path,
    video: &VideoInfo,
    options: VmafOptions,
) -> Vec<String> {
    let format = comparison_pixel_format(video);
    let model = model_for(video);
    let threads = if options.threads == 0 {
        std::thread::available_parallelism()
            .map(|value| value.get() as u32)
            .unwrap_or(1)
    } else {
        options.threads
    };
    let subsample = options.subsample.max(1);

    // Input 0 is the distorted video and input 1 the reference; the filter is
    // documented that way round and silently reports a different number if the
    // inputs are swapped. Resetting the timebase and start time on both keeps a
    // source that does not begin at zero from scoring far too low.
    let filter = format!(
        "[0:v:0]settb=AVTB,setpts=PTS-STARTPTS,format={format}[dist];\
         [1:v:0]settb=AVTB,setpts=PTS-STARTPTS,format={format}[ref];\
         [dist][ref]libvmaf=model=version={model}:n_threads={threads}:n_subsample={subsample}:shortest=1:ts_sync_mode=nearest"
    );

    vec![
        "-hide_banner".to_string(),
        "-nostdin".to_string(),
        "-loglevel".to_string(),
        "info".to_string(),
        "-nostats".to_string(),
        // No -hwaccel here: the filter needs frames in system memory, and a
        // decoder that hands back hardware frames makes the graph fail.
        "-i".to_string(),
        distorted.to_string_lossy().to_string(),
        "-i".to_string(),
        reference.to_string_lossy().to_string(),
        "-lavfi".to_string(),
        filter,
        "-f".to_string(),
        "null".to_string(),
        "-".to_string(),
    ]
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

// AI-FUNC-SUMMARY: Measures one converted file against its source; returns the score or a user-facing error; side effects: runs ffmpeg to completion, reading both files.
pub fn measure_vmaf(
    ffmpeg: &str,
    distorted: &Path,
    reference: &Path,
    video: &VideoInfo,
    options: VmafOptions,
) -> Result<VmafScore, String> {
    let args = build_vmaf_args(distorted, reference, video, options);
    let output = process_command(ffmpeg)
        .args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|err| format!("failed to run ffmpeg to measure quality: {err}"))?;

    let mut text = String::from_utf8_lossy(&output.stderr).to_string();
    text.push_str(&String::from_utf8_lossy(&output.stdout));

    if !output.status.success() {
        return Err(format!(
            "quality measurement failed: {}",
            first_error_line(&text)
        ));
    }

    parse_vmaf_score(&text)
        .map(|hundredths| VmafScore {
            hundredths,
            subsample: options.subsample.max(1),
        })
        .ok_or_else(|| "quality measurement produced no score".to_string())
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
        assert!(joined.ends_with("-f null -"));
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
}
