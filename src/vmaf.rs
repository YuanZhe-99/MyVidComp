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
    HwAccel, PROGRESS_ARGS, VideoInfo, first_error_line, normalized_pix_fmt,
    pixel_format_bit_depth, process_command,
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
    progress: bool,
    // One method per input, because the two files are in different codecs: the
    // converted one is in the target codec, and hardware that decodes the
    // source may refuse what was just produced from it.
    accel: (Option<HwAccel>, Option<HwAccel>),
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
}
