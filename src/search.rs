//! Finding the quality setting that just reaches the target.
//!
//! Encoders do not agree on what a given quality number looks like, and the
//! same number produces very different results on different footage. Rather
//! than guess, this encodes a few short samples at trial settings, measures
//! each one, and interpolates towards the setting that lands just above the
//! target. The whole file is then encoded once, at that setting.

use std::fs;
use std::path::{Path, PathBuf};

use super::{
    EncoderKind, EncoderQuality, Phase, StreamedRun, StreamedRunner, TargetCodec, VideoInfo,
    VmafOptions, codec, first_error_line, vmaf,
};

/// How samples are taken from a source.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct SamplePlan {
    /// Length of each sample in seconds.
    pub(crate) duration: f64,
    /// Number of samples to take, spread evenly through the file.
    pub(crate) count: usize,
}

/// The most trial encodes one search will run.
pub(crate) const MAX_ITERATIONS: usize = 6;

// AI-FUNC-SUMMARY:
// Purpose: Counts the ffmpeg runs a search may need, as the denominator its progress is measured against.
// Inputs: How many samples the file was divided into.
// Returns: The worst-case number of runs.
// Side effects: None.
// Notes: One extraction per sample, then an encode and a measurement per sample per iteration. Most searches stop long before this, which is why the phase closes early rather than counting down.
pub(crate) fn search_steps(samples: usize) -> usize {
    samples * (1 + 2 * MAX_ITERATIONS)
}
/// Seconds of video in each sample.
const SAMPLE_SECONDS: f64 = 20.0;
/// Aim for roughly one sample per this many seconds of source.
const SECONDS_PER_SAMPLE: f64 = 720.0;
/// Never take more than this many samples, however long the file is.
const MAX_SAMPLES: usize = 6;
/// Below this length, trial encodes cost about as much as the real one, so the
/// starting estimate is used instead.
const MIN_SEARCHABLE_SECONDS: f64 = 90.0;

// AI-FUNC-SUMMARY: Decides how many samples to take from a source and how long each should be; returns the plan, or none when the file is too short to sample usefully; side effects: none.
pub(crate) fn sample_plan(duration_seconds: f64) -> Option<SamplePlan> {
    if duration_seconds < MIN_SEARCHABLE_SECONDS {
        return None;
    }

    let count = ((duration_seconds / SECONDS_PER_SAMPLE).ceil() as usize).clamp(1, MAX_SAMPLES);
    let total_sampled = SAMPLE_SECONDS * count as f64;

    // Sampling most of a short file costs more than simply encoding it, so in
    // that case the caller should search on the whole thing instead.
    if total_sampled >= duration_seconds * 0.85 {
        return None;
    }

    Some(SamplePlan {
        duration: SAMPLE_SECONDS,
        count,
    })
}

// AI-FUNC-SUMMARY: Works out where each sample starts, spread evenly and away from the very beginning and end; returns one start time per sample; side effects: none.
pub(crate) fn sample_offsets(duration_seconds: f64, plan: SamplePlan) -> Vec<f64> {
    let usable = (duration_seconds - plan.duration).max(0.0);
    (0..plan.count)
        .map(|index| {
            // Spread the samples through the middle of the file: opening titles
            // and closing credits are not representative of the rest.
            let fraction = (index as f64 + 1.0) / (plan.count as f64 + 1.0);
            usable * fraction
        })
        .collect()
}

// AI-FUNC-SUMMARY: Picks the next quality value to try from the results so far; returns the value, or none when the search has converged; side effects: none.
pub(crate) fn next_quality(
    tried: &[(f64, u32)],
    target: u32,
    range: codec::QualityRange,
) -> Option<f64> {
    if tried.is_empty() {
        return Some(midpoint(range));
    }

    // The best point that still clears the target, and the worst that does not.
    // Lower quality values mean better quality, so the answer lies between them.
    let mut passing: Option<(f64, u32)> = None;
    let mut failing: Option<(f64, u32)> = None;
    for &(value, score) in tried {
        if score >= target {
            // Prefer the largest passing value, which is the smallest file.
            if passing.is_none_or(|(best, _)| value > best) {
                passing = Some((value, score));
            }
        } else if failing.is_none_or(|(worst, _)| value < worst) {
            failing = Some((value, score));
        }
    }

    let next = match (passing, failing) {
        // A point on each side: interpolate between them by score.
        (Some((pass_value, pass_score)), Some((fail_value, fail_score))) => {
            if (fail_value - pass_value).abs() <= range.step {
                return None;
            }
            let span = f64::from(pass_score) - f64::from(fail_score);
            let fraction = if span.abs() < f64::EPSILON {
                0.5
            } else {
                ((f64::from(pass_score) - f64::from(target)) / span).clamp(0.05, 0.95)
            };
            pass_value + (fail_value - pass_value) * fraction
        }
        // Everything passed so far, so try a larger value for a smaller file.
        (Some((pass_value, _)), None) => {
            if pass_value >= range.max - range.step {
                return None;
            }
            pass_value + (range.max - pass_value) * 0.5
        }
        // Nothing passed, so try a smaller value for better quality.
        (None, Some((fail_value, _))) => {
            if fail_value <= range.min + range.step {
                return None;
            }
            fail_value - (fail_value - range.min) * 0.5
        }
        (None, None) => midpoint(range),
    };

    let snapped = snap(next, range);
    // Never repeat a setting that has already been measured.
    if tried
        .iter()
        .any(|(value, _)| (value - snapped).abs() < range.step / 2.0)
    {
        return None;
    }
    Some(snapped)
}

// AI-FUNC-SUMMARY: Chooses the final setting from every trial; returns the largest value that reached the target, or the best-scoring value when none did; side effects: none.
pub(crate) fn best_quality(tried: &[(f64, u32)], target: u32) -> Option<(f64, u32, bool)> {
    let passing = tried
        .iter()
        .filter(|(_, score)| *score >= target)
        .max_by(|left, right| left.0.total_cmp(&right.0));

    if let Some(&(value, score)) = passing {
        return Some((value, score, true));
    }

    // Nothing reached the target, so fall back to whichever trial looked best
    // and let the caller decide what to do about it.
    tried
        .iter()
        .max_by_key(|(_, score)| *score)
        .map(|&(value, score)| (value, score, false))
}

// AI-FUNC-SUMMARY: Rounds a quality value onto the steps its encoder accepts; returns the snapped value inside the range; side effects: none.
fn snap(value: f64, range: codec::QualityRange) -> f64 {
    let stepped = (value / range.step).round() * range.step;
    stepped.clamp(range.min, range.max)
}

// AI-FUNC-SUMMARY: Reports the middle of a quality range, snapped to its steps; returns the starting value for a search; side effects: none.
fn midpoint(range: codec::QualityRange) -> f64 {
    snap((range.min + range.max) / 2.0, range)
}

// AI-FUNC-SUMMARY: Builds the ffmpeg arguments that copy one sample out of a source; returns the argument list; side effects: none.
pub(crate) fn build_sample_args(
    input: &Path,
    start: f64,
    duration: f64,
    output: &Path,
) -> Vec<String> {
    vec![
        "-hide_banner".to_string(),
        "-nostdin".to_string(),
        "-loglevel".to_string(),
        "error".to_string(),
        "-y".to_string(),
        // Seeking before the input is fast and lands on a keyframe, which is
        // what a representative sample wants.
        "-ss".to_string(),
        format!("{start:.3}"),
        "-i".to_string(),
        input.to_string_lossy().to_string(),
        "-t".to_string(),
        format!("{duration:.3}"),
        "-map".to_string(),
        "0:v:0".to_string(),
        "-c".to_string(),
        "copy".to_string(),
        "-f".to_string(),
        "matroska".to_string(),
        output.to_string_lossy().to_string(),
    ]
}

// AI-FUNC-SUMMARY: Builds the ffmpeg arguments that encode one sample at a trial quality; returns the argument list; side effects: none.
pub(crate) fn build_trial_args(
    sample: &Path,
    encoder: &str,
    quality_args: &[String],
    pix_fmt: Option<&str>,
    output: &Path,
) -> Vec<String> {
    let mut args = vec![
        "-hide_banner".to_string(),
        "-nostdin".to_string(),
        "-loglevel".to_string(),
        "error".to_string(),
        "-y".to_string(),
        "-i".to_string(),
        sample.to_string_lossy().to_string(),
        "-map".to_string(),
        "0:v:0".to_string(),
        "-an".to_string(),
        "-c:v".to_string(),
        encoder.to_string(),
    ];
    args.extend(quality_args.iter().cloned());
    if let Some(pix_fmt) = pix_fmt {
        args.extend(["-pix_fmt:v:0".to_string(), pix_fmt.to_string()]);
    }
    args.extend([
        "-f".to_string(),
        "matroska".to_string(),
        output.to_string_lossy().to_string(),
    ]);
    args
}

/// The files one search created, removed when the search finishes.
pub(crate) struct SampleSet {
    paths: Vec<PathBuf>,
}

impl SampleSet {
    // AI-FUNC-SUMMARY: Reports the sample files that were extracted; returns their paths; side effects: none.
    pub(crate) fn paths(&self) -> &[PathBuf] {
        &self.paths
    }
}

impl Drop for SampleSet {
    // AI-FUNC-SUMMARY: Removes the extracted samples; returns none; side effects: deletes temporary files.
    fn drop(&mut self) {
        for path in &self.paths {
            let _ = fs::remove_file(path);
        }
    }
}

// AI-FUNC-SUMMARY: Copies short samples out of a source into the working directory; returns the samples, or an error when none could be taken; side effects: runs ffmpeg once per sample and writes temporary files.
pub(crate) fn extract_samples(
    input: &Path,
    video: &VideoInfo,
    plan: SamplePlan,
    work_dir: &Path,
    stamp: u128,
    run: StreamedRunner<'_>,
) -> Result<SampleSet, String> {
    let mut paths = Vec::new();
    let mut set = SampleSet { paths: Vec::new() };

    for (index, start) in sample_offsets(video.duration_seconds, plan)
        .into_iter()
        .enumerate()
    {
        let path = work_dir.join(format!("{}sample-{stamp}-{index}.mkv", super::TEMP_PREFIX));
        let args = build_sample_args(input, start, plan.duration, &path);
        let output = run(
            &args,
            StreamedRun {
                phase: Phase::Choosing,
                duration_seconds: plan.duration,
                step: index + 1,
                steps: search_steps(plan.count),
            },
        )?;

        // A sample that could not be taken is skipped rather than fatal: the
        // remaining samples still describe the file well enough.
        if output.status.success() && path.is_file() {
            paths.push(path);
        }
    }

    if paths.is_empty() {
        return Err("could not take any samples from this file".to_string());
    }

    set.paths = paths;
    Ok(set)
}

/// One candidate setting, and everything needed to try it on the samples.
pub(crate) struct Trial<'a> {
    pub(crate) samples: &'a [PathBuf],
    pub(crate) encoder: &'a str,
    pub(crate) kind: EncoderKind,
    pub(crate) quality: f64,
    pub(crate) codec: TargetCodec,
    pub(crate) video: &'a VideoInfo,
    /// Which pass of the search this is, counting from zero, so its runs can be
    /// placed inside the phase's worst case.
    pub(crate) iteration: usize,
    /// How long each sample is, as the denominator for each run.
    pub(crate) duration: f64,
}

// AI-FUNC-SUMMARY: Encodes every sample at one quality setting and measures the result; returns the average score across the samples; side effects: runs ffmpeg twice per sample and writes then deletes temporary files.
pub(crate) fn measure_trial(trial: Trial<'_>, run: StreamedRunner<'_>) -> Result<u32, String> {
    let Trial {
        samples,
        encoder,
        kind,
        quality,
        codec,
        video,
        iteration,
        duration,
    } = trial;
    let quality_args = trial_quality_args(kind, quality);
    let pix_fmt = codec::required_pixel_format(encoder);
    let mut total = 0u64;
    let mut measured = 0u32;

    // One extraction per sample happened before this, and each iteration
    // encodes and then measures every sample.
    let steps = search_steps(samples.len());
    let base = samples.len() + iteration * 2 * samples.len();

    for (index, sample) in samples.iter().enumerate() {
        let encoded = super::with_added_suffix(sample, ".trial.mkv");
        let args = build_trial_args(sample, encoder, &quality_args, pix_fmt, &encoded);
        let encode = run(
            &args,
            StreamedRun {
                phase: Phase::Choosing,
                duration_seconds: duration,
                step: base + index * 2 + 1,
                steps,
            },
        );

        let encode = match encode {
            Ok(encode) => encode,
            Err(err) => {
                let _ = fs::remove_file(&encoded);
                return Err(err);
            }
        };

        if !encode.status.success() {
            let detail = first_error_line(&encode.stderr);
            let _ = fs::remove_file(&encoded);
            return Err(format!("trial encode failed: {detail}"));
        }

        // Trials are measured on every frame: the samples are short, and a
        // subsampled score on 20 seconds is too noisy to steer a search.
        let options = VmafOptions {
            subsample: 1,
            threads: 0,
        };
        // Trials stay on the processor: a sample is twenty seconds, and creating
        // a decoding device costs more than software decoding that much video,
        // up to thirty-six times per file.
        let args = vmaf::build_vmaf_args(&encoded, sample, video, options, true, (None, None));
        let measurement = run(
            &args,
            StreamedRun {
                phase: Phase::Choosing,
                duration_seconds: duration,
                step: base + index * 2 + 2,
                steps,
            },
        );
        let _ = fs::remove_file(&encoded);

        let score = measurement.and_then(|measurement| {
            vmaf::score_from_output(measurement.status.success(), &measurement.stderr, 1)
        });
        if let Ok(score) = score {
            total += u64::from(score.hundredths);
            measured += 1;
        }
    }

    let _ = codec;
    if measured == 0 {
        return Err("no sample could be measured".to_string());
    }

    Ok((total / u64::from(measured)) as u32)
}

// AI-FUNC-SUMMARY: Builds the quality arguments for one trial encode at a fast preset; returns the argument list; side effects: none.
fn trial_quality_args(kind: EncoderKind, quality: f64) -> Vec<String> {
    // Trials use the same quality value the real encode will, but the fastest
    // preset each encoder offers. A preset changes size more than it changes
    // quality, so the score still predicts the final result closely enough.
    let value = match kind {
        EncoderKind::X265 => format!("{quality:.1}"),
        _ => format!("{:.0}", quality.round()),
    };

    match kind {
        EncoderKind::SvtAv1 => vec![
            "-crf".to_string(),
            value,
            "-preset".to_string(),
            "8".to_string(),
        ],
        EncoderKind::LibAom => vec![
            "-crf".to_string(),
            value,
            "-b:v".to_string(),
            "0".to_string(),
            "-cpu-used".to_string(),
            "6".to_string(),
        ],
        EncoderKind::Vulkan => vec![
            "-crf".to_string(),
            value,
            "-b:v".to_string(),
            "0".to_string(),
        ],
        EncoderKind::X265 => vec![
            "-crf".to_string(),
            value,
            "-preset".to_string(),
            "veryfast".to_string(),
        ],
        EncoderKind::Rav1e => vec![
            "-qp".to_string(),
            value,
            "-speed".to_string(),
            "9".to_string(),
        ],
        EncoderKind::Vvenc => vec![
            "-qp".to_string(),
            value,
            "-preset".to_string(),
            "faster".to_string(),
        ],
        EncoderKind::Hardware | EncoderKind::MediaCodec => {
            vec!["-b:v".to_string(), format!("{}k", quality.round() as u64)]
        }
    }
}

// AI-FUNC-SUMMARY: Converts a searched value back into the quality setting the encode will use; returns the setting; side effects: none.
pub(crate) fn quality_from_value(kind: EncoderKind, value: f64) -> EncoderQuality {
    match codec::quality_style(kind) {
        // For hardware encoders the searched value is a bitrate in kilobits.
        codec::QualityStyle::Bitrate => EncoderQuality::Bitrate((value.max(1.0) as u64) * 1000),
        _ => EncoderQuality::Constant(value),
    }
}

// AI-FUNC-SUMMARY: Lists the bitrates a hardware search should try, from smallest to largest; returns bitrates in kilobits per second; side effects: none.
pub(crate) fn hardware_bitrate_ladder(source_bits_per_second: u64) -> Vec<f64> {
    // Hardware encoders take a bitrate rather than a quality value, so the
    // search walks a ladder of fractions of the source bitrate instead of
    // interpolating on a quality scale.
    let source = (source_bits_per_second as f64 / 1000.0).max(1.0);
    [0.15, 0.25, 0.4, 0.6, 0.85]
        .into_iter()
        .map(|fraction| (source * fraction).round().max(100.0))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // AI-FUNC-SUMMARY: Builds the quality range used across the search tests; returns a CRF-style range; side effects: none.
    fn crf_range() -> codec::QualityRange {
        codec::quality_range(EncoderKind::SvtAv1)
    }

    #[test]
    // AI-FUNC-SUMMARY: Verifies sample counts scale with duration and short files opt out; returns nothing; side effects: none.
    fn plans_samples_by_duration() {
        // A short clip is encoded directly rather than sampled.
        assert_eq!(sample_plan(45.0), None);
        // Ten minutes gets a single sample.
        assert_eq!(sample_plan(600.0).unwrap().count, 1);
        // Half an hour gets three samples.
        assert_eq!(sample_plan(1800.0).unwrap().count, 3);
        // A very long file is capped.
        assert_eq!(sample_plan(36000.0).unwrap().count, MAX_SAMPLES);
        assert_eq!(sample_plan(0.0), None);
    }

    #[test]
    // AI-FUNC-SUMMARY: Verifies samples avoid the opening and closing of a file; returns nothing; side effects: none.
    fn spreads_samples_through_the_middle() {
        let plan = SamplePlan {
            duration: 20.0,
            count: 3,
        };
        let offsets = sample_offsets(1000.0, plan);

        assert_eq!(offsets.len(), 3);
        assert!(offsets[0] > 0.0);
        assert!(offsets[0] < offsets[1] && offsets[1] < offsets[2]);
        assert!(offsets[2] + plan.duration <= 1000.0);
    }

    #[test]
    // AI-FUNC-SUMMARY: Verifies the first trial starts in the middle of the range; returns nothing; side effects: none.
    fn starts_in_the_middle() {
        let range = crf_range();
        let first = next_quality(&[], 9500, range).unwrap();
        assert!((first - 33.0).abs() < 1.0);
    }

    #[test]
    // AI-FUNC-SUMMARY: Verifies a failing trial makes the search ask for better quality; returns nothing; side effects: none.
    fn moves_towards_better_quality_after_a_miss() {
        let range = crf_range();
        let next = next_quality(&[(33.0, 9000)], 9500, range).unwrap();
        assert!(next < 33.0);
    }

    #[test]
    // AI-FUNC-SUMMARY: Verifies a comfortable pass makes the search ask for a smaller file; returns nothing; side effects: none.
    fn moves_towards_smaller_files_after_a_pass() {
        let range = crf_range();
        let next = next_quality(&[(33.0, 9800)], 9500, range).unwrap();
        assert!(next > 33.0);
    }

    #[test]
    // AI-FUNC-SUMMARY: Verifies the search stops once the bracket is one step wide; returns nothing; side effects: none.
    fn stops_when_the_bracket_closes() {
        let range = crf_range();
        assert_eq!(
            next_quality(&[(30.0, 9600), (31.0, 9400)], 9500, range),
            None
        );
    }

    #[test]
    // AI-FUNC-SUMMARY: Verifies the search never repeats a setting it already measured; returns nothing; side effects: none.
    fn never_repeats_a_setting() {
        let range = crf_range();
        let mut tried = vec![(33.0, 9550)];
        for _ in 0..MAX_ITERATIONS {
            let Some(next) = next_quality(&tried, 9500, range) else {
                break;
            };
            assert!(
                !tried.iter().any(|(value, _)| (value - next).abs() < 0.5),
                "search repeated {next}"
            );
            tried.push((next, 9500));
        }
    }

    #[test]
    // AI-FUNC-SUMMARY: Verifies the chosen setting is the smallest file that still reaches the target; returns nothing; side effects: none.
    fn picks_the_largest_passing_value() {
        let tried = [(28.0, 9700), (32.0, 9550), (35.0, 9300)];
        let (value, score, reached) = best_quality(&tried, 9500).unwrap();
        assert_eq!(value, 32.0);
        assert_eq!(score, 9550);
        assert!(reached);
    }

    #[test]
    // AI-FUNC-SUMMARY: Verifies a search that never reaches the target reports its best attempt; returns nothing; side effects: none.
    fn falls_back_to_the_best_attempt() {
        let tried = [(30.0, 9200), (34.0, 9000)];
        let (value, score, reached) = best_quality(&tried, 9500).unwrap();
        assert_eq!(value, 30.0);
        assert_eq!(score, 9200);
        assert!(!reached);
        assert_eq!(best_quality(&[], 9500), None);
    }

    #[test]
    // AI-FUNC-SUMMARY: Verifies every searched value stays inside the encoder range; returns nothing; side effects: none.
    fn stays_inside_the_encoder_range() {
        for kind in [
            EncoderKind::SvtAv1,
            EncoderKind::X265,
            EncoderKind::Rav1e,
            EncoderKind::Vvenc,
        ] {
            let range = codec::quality_range(kind);
            let mut tried: Vec<(f64, u32)> = Vec::new();
            for iteration in 0..MAX_ITERATIONS {
                let Some(next) = next_quality(&tried, 9500, range) else {
                    break;
                };
                assert!(next >= range.min && next <= range.max, "{kind:?} {next}");
                // Alternate pass and fail so the search explores both directions.
                tried.push((next, if iteration % 2 == 0 { 9600 } else { 9200 }));
            }
        }
    }

    #[test]
    // AI-FUNC-SUMMARY: Verifies the sample command copies the stream without re-encoding; returns nothing; side effects: none.
    fn builds_a_sample_command() {
        let args = build_sample_args(
            Path::new("/videos/clip.mkv"),
            120.0,
            20.0,
            Path::new("/tmp/sample.mkv"),
        );
        let joined = args.join(" ");

        assert!(joined.contains("-ss 120.000"));
        assert!(joined.contains("-t 20.000"));
        assert!(joined.contains("-c copy"));
        // Seeking has to come before the input to be fast.
        let ss = args.iter().position(|arg| arg == "-ss").unwrap();
        let input = args.iter().position(|arg| arg == "-i").unwrap();
        assert!(ss < input);
    }

    #[test]
    // AI-FUNC-SUMMARY: Verifies a trial command drops audio and applies a forced pixel format; returns nothing; side effects: none.
    fn builds_a_trial_command() {
        let args = build_trial_args(
            Path::new("/tmp/sample.mkv"),
            "libvvenc",
            &["-qp".to_string(), "32".to_string()],
            Some("yuv420p10le"),
            Path::new("/tmp/trial.mkv"),
        );
        let joined = args.join(" ");

        assert!(joined.contains("-c:v libvvenc"));
        assert!(joined.contains("-qp 32"));
        assert!(joined.contains("-pix_fmt:v:0 yuv420p10le"));
        assert!(joined.contains("-an"));
    }

    #[test]
    // AI-FUNC-SUMMARY: Verifies a searched value becomes the right kind of quality setting; returns nothing; side effects: none.
    fn converts_values_to_settings() {
        assert_eq!(
            quality_from_value(EncoderKind::SvtAv1, 30.0),
            EncoderQuality::Constant(30.0)
        );
        assert_eq!(
            quality_from_value(EncoderKind::Hardware, 4000.0),
            EncoderQuality::Bitrate(4_000_000)
        );
    }

    #[test]
    // AI-FUNC-SUMMARY: Verifies the hardware ladder rises and stays above the floor; returns nothing; side effects: none.
    fn builds_a_hardware_ladder() {
        let ladder = hardware_bitrate_ladder(10_000_000);
        assert_eq!(ladder.len(), 5);
        assert!(ladder.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(ladder.iter().all(|value| *value >= 100.0));

        // Even a tiny source produces a usable ladder.
        assert!(hardware_bitrate_ladder(1000).iter().all(|v| *v >= 100.0));
    }
}
