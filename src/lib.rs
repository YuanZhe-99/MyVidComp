use std::collections::{HashSet, VecDeque};
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io::{self, BufRead, BufReader, IsTerminal, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

mod codec;
mod ffi;
mod options;
mod review;
mod search;
mod vmaf;

// The codec tables are internal: they describe encoder families the pipeline
// knows about, not a public contract.
pub use ffi::{FfiEventCallback, FfiRunOptionsV1, MYVIDCOMP_FFI_ABI_VERSION};
pub use review::{ReviewDecision, ReviewItem, list_reviews, resolve_review, review_list_json};
pub use vmaf::{VmafOptions, VmafScore};

pub use options::{
    DEFAULT_QUALITY_TARGET, DEFAULT_REVIEW_MARGIN, EncoderPreference, Preservation, QualityCheck,
    QualityMode, TargetCodec, format_quality, normalize_encoder_preference_choice,
    normalize_preservation_choice, normalize_quality_check_choice, normalize_quality_mode_choice,
    normalize_target_codec_choice, parse_encoder_preference, parse_preservation,
    parse_quality_check, parse_quality_mode, parse_quality_points, parse_quality_target,
    parse_target_codec, validate_quality_target,
};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// Prefix for temporary conversion outputs created by this build.
const TEMP_PREFIX: &str = ".myvidcomp-";
/// Prefix used by PVAC, the previous name of this project. Still recognised so
/// interrupted work from an older install can be reused and cleaned up.
const LEGACY_TEMP_PREFIX: &str = ".pvac-";
/// Suffix for the short-lived recovery copy of an original during an in-place commit.
const RECOVERY_SUFFIX: &str = ".myvidcomp.recover";
/// Suffix appended to a converted file that is waiting for the user to review it.
const REVIEW_SUFFIX: &str = ".myvidcomp-review";

// AI-FUNC-SUMMARY: Creates a child-process command without a console window on Windows; returns a configurable command; side effects: none until the caller starts the process.
fn process_command(program: impl AsRef<OsStr>) -> Command {
    #[cfg_attr(not(windows), expect(unused_mut, reason = "only Windows sets a flag"))]
    let mut command = Command::new(program);
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);
    command
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum OutputFormat {
    #[default]
    Mp4,
    MkvFallback,
}

impl OutputFormat {
    // AI-FUNC-SUMMARY: Maps an output format to its stable wire value; returns static label; side effects: none.
    pub fn as_str(&self) -> &'static str {
        match self {
            OutputFormat::Mp4 => "mp4",
            OutputFormat::MkvFallback => "mkv-fallback",
        }
    }

    // AI-FUNC-SUMMARY: Maps an output format to its human-readable log label; returns static label; side effects: none.
    fn label(&self) -> &'static str {
        match self {
            OutputFormat::Mp4 => "MP4 output",
            OutputFormat::MkvFallback => "MP4 with MKV fallback",
        }
    }
}

// AI-FUNC-SUMMARY: Parses an output-format wire value; returns the format or a user-facing error; side effects: none.
fn parse_output_format(value: &str) -> Result<OutputFormat, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "mp4" => Ok(OutputFormat::Mp4),
        "mkv-fallback" | "mkv_fallback" | "mkvfallback" => Ok(OutputFormat::MkvFallback),
        other => Err(format!(
            "invalid output format: {other}. Supported values: mp4, mkv-fallback"
        )),
    }
}

// AI-FUNC-SUMMARY: Normalizes an optional output-format value; returns none for blank/auto/default or a parsed format; side effects: none.
fn normalize_output_format_choice(value: &str) -> Result<Option<OutputFormat>, String> {
    let value = value.trim();
    if value.is_empty()
        || value.eq_ignore_ascii_case("auto")
        || value.eq_ignore_ascii_case("default")
    {
        Ok(None)
    } else {
        parse_output_format(value).map(Some)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunOptions {
    pub target_folder: PathBuf,
    pub count: isize,
    pub keep_original: bool,
    pub dry_run: bool,
    pub ffmpeg: String,
    pub ffprobe: String,
    pub tmp_dir: Option<PathBuf>,
    pub encoder: Option<String>,
    pub encoder_preference: EncoderPreference,
    pub output_format: OutputFormat,
    pub target_codec: TargetCodec,
    pub preservation: Preservation,
    pub quality_mode: QualityMode,
    pub quality_check: QualityCheck,
    /// Quality target as a VMAF score in hundredths, for example 9500 for 95.0.
    pub quality_target: u32,
    /// How far below the target a result may land before it goes to review,
    /// also in hundredths.
    pub review_margin: u32,
    /// Worker threads for quality measurement. Zero means "match the machine".
    pub quality_threads: u32,
}

impl Default for RunOptions {
    // AI-FUNC-SUMMARY: Builds default embedded run options; returns options matching CLI defaults; side effects: probes bundled runtime binary locations.
    fn default() -> Self {
        Self {
            target_folder: PathBuf::new(),
            count: -1,
            keep_original: true,
            dry_run: false,
            ffmpeg: default_runtime_binary("ffmpeg"),
            ffprobe: default_runtime_binary("ffprobe"),
            tmp_dir: None,
            encoder: None,
            encoder_preference: EncoderPreference::Auto,
            output_format: OutputFormat::Mp4,
            target_codec: TargetCodec::Av1,
            preservation: Preservation::Flexible,
            quality_mode: QualityMode::Search,
            quality_check: QualityCheck::Sampled,
            quality_target: DEFAULT_QUALITY_TARGET,
            review_margin: DEFAULT_REVIEW_MARGIN,
            quality_threads: 0,
        }
    }
}

impl RunOptions {
    // AI-FUNC-SUMMARY: Reports the score below which a result must be reviewed; returns the threshold in hundredths; side effects: none.
    pub fn review_threshold(&self) -> u32 {
        self.quality_target.saturating_sub(self.review_margin)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RunSummary {
    pub converted: usize,
    pub skipped: usize,
    pub failed: usize,
    pub source_bytes: u64,
    pub output_bytes: u64,
}

impl RunSummary {
    // AI-FUNC-SUMMARY: Counts conversion attempts represented by the summary; returns converted plus failed files; side effects: none.
    pub fn conversion_attempts(&self) -> usize {
        self.converted + self.failed
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidatePreview {
    pub input_path: PathBuf,
    pub output_path: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileStatus {
    Converted,
    Failed,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    Log {
        level: LogLevel,
        message: String,
    },
    /// The settings this run will use, sent once before any file is touched so
    /// a front end can show what it is about to do.
    SettingsSelected {
        codec: TargetCodec,
        preservation: Preservation,
        encoder_preference: EncoderPreference,
        output_format: OutputFormat,
        quality_mode: QualityMode,
        quality_check: QualityCheck,
        /// Quality target in hundredths of a VMAF point.
        quality_target: u32,
        /// Score below which a result goes to review, in hundredths.
        review_threshold: u32,
    },
    /// A capability the run needs is missing, so it fell back to something else.
    CapabilityMissing {
        capability: String,
        detail: String,
    },
    /// One step of a quality search: a trial setting and what it scored.
    QualitySearch {
        index: usize,
        iteration: usize,
        encoder: String,
        quality: String,
        /// Measured score in hundredths, when the trial produced one.
        score: Option<u32>,
    },
    /// The finished file was compared against its source.
    QualityMeasured {
        index: usize,
        /// Measured score in hundredths.
        score: u32,
        /// Frames compared out of every group.
        subsample: u32,
        /// Score the run was aiming for, in hundredths.
        target: u32,
        /// Whether the score cleared the review threshold.
        passed: bool,
        /// Set when the score should not be trusted, for example on HDR sources.
        caveat: Option<String>,
    },
    /// A conversion finished but was kept beside its original for the user to
    /// judge, either because something changed or because it scored low.
    ReviewPending {
        index: usize,
        original_path: PathBuf,
        review_path: PathBuf,
        sidecar_path: PathBuf,
        /// Measured score in hundredths, when it could be measured.
        score: Option<u32>,
        original_bytes: u64,
        review_bytes: u64,
        /// One entry per recoverable difference, as `kind: detail`.
        deviations: Vec<String>,
    },
    EncoderCandidateEvaluated {
        encoder: String,
        advertised: bool,
        usable: bool,
        detail: Option<String>,
    },
    EncoderSelected {
        encoder: String,
    },
    ScanStarted {
        target_folder: PathBuf,
    },
    ScanFinished {
        candidates: usize,
        skipped: usize,
    },
    DryRun {
        encoder: String,
        limit: Option<usize>,
        candidates: Vec<CandidatePreview>,
        skipped: usize,
    },
    FileSkipped {
        path: PathBuf,
        reason: String,
    },
    FileStarted {
        index: usize,
        total: Option<usize>,
        input_path: PathBuf,
        output_path: PathBuf,
        encoder: String,
        quality: String,
    },
    FileAttempt {
        index: usize,
        attempt: usize,
        total_attempts: usize,
        encoder: String,
        quality: String,
        plan: String,
        target_pix_fmt: Option<String>,
        fallback_reason: Option<String>,
    },
    FileProgress {
        index: usize,
        percent: f64,
        speed: Option<String>,
        eta_seconds: u64,
    },
    CopyProgress {
        index: usize,
        percent: f64,
        copied_bytes: u64,
        total_bytes: u64,
        eta_seconds: u64,
    },
    Stage {
        index: usize,
        stage: String,
    },
    FileFinished {
        index: usize,
        input_path: PathBuf,
        status: FileStatus,
        message: String,
        source_bytes: Option<u64>,
        output_bytes: Option<u64>,
    },
    StopRequested,
    Summary {
        summary: RunSummary,
    },
}

pub trait EventSink {
    // AI-FUNC-SUMMARY: Receives a structured MyVidComp workflow event; returns none; side effects: defined by the implementer.
    fn on_event(&mut self, event: Event);
}

impl<F> EventSink for F
where
    F: FnMut(Event),
{
    // AI-FUNC-SUMMARY: Forwards a MyVidComp workflow event into a closure sink; returns none; side effects: runs the closure.
    fn on_event(&mut self, event: Event) {
        self(event);
    }
}

#[derive(Debug, Clone)]
pub struct CancellationToken {
    requested: Arc<AtomicBool>,
}

impl Default for CancellationToken {
    // AI-FUNC-SUMMARY: Constructs a non-cancelled token; returns the token; side effects: allocates shared atomic state.
    fn default() -> Self {
        Self::new()
    }
}

impl CancellationToken {
    // AI-FUNC-SUMMARY: Constructs a non-cancelled token; returns the token; side effects: allocates shared atomic state.
    pub fn new() -> Self {
        Self {
            requested: Arc::new(AtomicBool::new(false)),
        }
    }

    // AI-FUNC-SUMMARY: Requests graceful cancellation after the active file completes; returns none; side effects: updates shared atomic state.
    pub fn request_stop(&self) {
        self.requested.store(true, Ordering::Relaxed);
    }

    // AI-FUNC-SUMMARY: Checks whether graceful cancellation has been requested; returns a boolean; side effects: none.
    pub fn is_stop_requested(&self) -> bool {
        self.requested.load(Ordering::Relaxed)
    }
}

struct NoopEventSink;

impl EventSink for NoopEventSink {
    // AI-FUNC-SUMMARY: Discards a structured MyVidComp workflow event; returns none; side effects: none.
    fn on_event(&mut self, _event: Event) {}
}

// AI-FUNC-SUMMARY: Serializes a MyVidComp event to JSON for FFI callbacks; returns compact JSON text; side effects: none.
fn event_json(event: &Event) -> String {
    match event {
        Event::Log { level, message } => format!(
            "{{\"type\":\"log\",\"level\":{},\"message\":{}}}",
            json_string(log_level_label(*level)),
            json_string(message)
        ),
        Event::SettingsSelected {
            codec,
            preservation,
            encoder_preference,
            output_format,
            quality_mode,
            quality_check,
            quality_target,
            review_threshold,
        } => format!(
            "{{\"type\":\"settings_selected\",\"codec\":{},\"preservation\":{},\"encoder_preference\":{},\"output_format\":{},\"quality_mode\":{},\"quality_check\":{},\"quality_target\":{quality_target},\"review_threshold\":{review_threshold}}}",
            json_string(codec.as_str()),
            json_string(preservation.as_str()),
            json_string(encoder_preference.as_str()),
            json_string(output_format.as_str()),
            json_string(quality_mode.as_str()),
            json_string(quality_check.as_str())
        ),
        Event::CapabilityMissing { capability, detail } => format!(
            "{{\"type\":\"capability_missing\",\"capability\":{},\"detail\":{}}}",
            json_string(capability),
            json_string(detail)
        ),
        Event::QualitySearch {
            index,
            iteration,
            encoder,
            quality,
            score,
        } => format!(
            "{{\"type\":\"quality_search\",\"index\":{index},\"iteration\":{iteration},\"encoder\":{},\"quality\":{},\"score\":{}}}",
            json_string(encoder),
            json_string(quality),
            json_optional_u32(*score)
        ),
        Event::QualityMeasured {
            index,
            score,
            subsample,
            target,
            passed,
            caveat,
        } => format!(
            "{{\"type\":\"quality_measured\",\"index\":{index},\"score\":{score},\"subsample\":{subsample},\"target\":{target},\"passed\":{passed},\"caveat\":{}}}",
            json_optional_string(caveat.as_deref())
        ),
        Event::ReviewPending {
            index,
            original_path,
            review_path,
            sidecar_path,
            score,
            original_bytes,
            review_bytes,
            deviations,
        } => format!(
            "{{\"type\":\"review_pending\",\"index\":{index},\"original_path\":{},\"review_path\":{},\"sidecar_path\":{},\"score\":{},\"original_bytes\":{original_bytes},\"review_bytes\":{review_bytes},\"deviations\":[{}]}}",
            json_path(original_path),
            json_path(review_path),
            json_path(sidecar_path),
            json_optional_u32(*score),
            deviations
                .iter()
                .map(|value| json_string(value))
                .collect::<Vec<_>>()
                .join(",")
        ),
        Event::EncoderCandidateEvaluated {
            encoder,
            advertised,
            usable,
            detail,
        } => format!(
            "{{\"type\":\"encoder_candidate_evaluated\",\"encoder\":{},\"advertised\":{},\"usable\":{},\"detail\":{}}}",
            json_string(encoder),
            advertised,
            usable,
            json_optional_string(detail.as_deref())
        ),
        Event::EncoderSelected { encoder } => format!(
            "{{\"type\":\"encoder_selected\",\"encoder\":{}}}",
            json_string(encoder)
        ),
        Event::ScanStarted { target_folder } => format!(
            "{{\"type\":\"scan_started\",\"target_folder\":{}}}",
            json_path(target_folder)
        ),
        Event::ScanFinished {
            candidates,
            skipped,
        } => format!(
            "{{\"type\":\"scan_finished\",\"candidates\":{},\"skipped\":{}}}",
            candidates, skipped
        ),
        Event::DryRun {
            encoder,
            limit,
            candidates,
            skipped,
        } => format!(
            "{{\"type\":\"dry_run\",\"encoder\":{},\"limit\":{},\"candidates\":[{}],\"skipped\":{}}}",
            json_string(encoder),
            json_optional_usize(*limit),
            candidates
                .iter()
                .map(candidate_json)
                .collect::<Vec<_>>()
                .join(","),
            skipped
        ),
        Event::FileSkipped { path, reason } => format!(
            "{{\"type\":\"file_skipped\",\"path\":{},\"reason\":{}}}",
            json_path(path),
            json_string(reason)
        ),
        Event::FileStarted {
            index,
            total,
            input_path,
            output_path,
            encoder,
            quality,
        } => format!(
            "{{\"type\":\"file_started\",\"index\":{},\"total\":{},\"input_path\":{},\"output_path\":{},\"encoder\":{},\"quality\":{}}}",
            index,
            json_optional_usize(*total),
            json_path(input_path),
            json_path(output_path),
            json_string(encoder),
            json_string(quality)
        ),
        Event::FileAttempt {
            index,
            attempt,
            total_attempts,
            encoder,
            quality,
            plan,
            target_pix_fmt,
            fallback_reason,
        } => format!(
            "{{\"type\":\"file_attempt\",\"index\":{},\"attempt\":{},\"total_attempts\":{},\"encoder\":{},\"quality\":{},\"plan\":{},\"target_pix_fmt\":{},\"fallback_reason\":{}}}",
            index,
            attempt,
            total_attempts,
            json_string(encoder),
            json_string(quality),
            json_string(plan),
            json_optional_string(target_pix_fmt.as_deref()),
            json_optional_string(fallback_reason.as_deref())
        ),
        Event::FileProgress {
            index,
            percent,
            speed,
            eta_seconds,
        } => format!(
            "{{\"type\":\"file_progress\",\"index\":{},\"percent\":{},\"speed\":{},\"eta_seconds\":{}}}",
            index,
            json_f64(*percent),
            json_optional_string(speed.as_deref()),
            eta_seconds
        ),
        Event::CopyProgress {
            index,
            percent,
            copied_bytes,
            total_bytes,
            eta_seconds,
        } => format!(
            "{{\"type\":\"copy_progress\",\"index\":{},\"percent\":{},\"copied_bytes\":{},\"total_bytes\":{},\"eta_seconds\":{}}}",
            index,
            json_f64(*percent),
            copied_bytes,
            total_bytes,
            eta_seconds
        ),
        Event::Stage { index, stage } => format!(
            "{{\"type\":\"stage\",\"index\":{},\"stage\":{}}}",
            index,
            json_string(stage)
        ),
        Event::FileFinished {
            index,
            input_path,
            status,
            message,
            source_bytes,
            output_bytes,
        } => format!(
            "{{\"type\":\"file_finished\",\"index\":{},\"input_path\":{},\"status\":{},\"message\":{},\"source_bytes\":{},\"output_bytes\":{}}}",
            index,
            json_path(input_path),
            json_string(file_status_label(*status)),
            json_string(message),
            json_optional_u64(*source_bytes),
            json_optional_u64(*output_bytes)
        ),
        Event::StopRequested => "{\"type\":\"stop_requested\"}".to_string(),
        Event::Summary { summary } => format!(
            "{{\"type\":\"summary\",\"summary\":{}}}",
            summary_json(summary)
        ),
    }
}

// AI-FUNC-SUMMARY: Serializes a dry-run candidate preview to JSON; returns compact JSON text; side effects: none.
fn candidate_json(candidate: &CandidatePreview) -> String {
    format!(
        "{{\"input_path\":{},\"output_path\":{}}}",
        json_path(&candidate.input_path),
        json_path(&candidate.output_path)
    )
}

// AI-FUNC-SUMMARY: Serializes a run summary to JSON; returns compact JSON text; side effects: none.
fn summary_json(summary: &RunSummary) -> String {
    format!(
        "{{\"converted\":{},\"skipped\":{},\"failed\":{},\"source_bytes\":{},\"output_bytes\":{}}}",
        summary.converted,
        summary.skipped,
        summary.failed,
        summary.source_bytes,
        summary.output_bytes
    )
}

// AI-FUNC-SUMMARY: Serializes a path value to a JSON string; returns escaped JSON text; side effects: none.
fn json_path(path: &Path) -> String {
    json_string(&path.to_string_lossy())
}

// AI-FUNC-SUMMARY: Serializes a string value to JSON; returns escaped JSON string text; side effects: none.
fn json_string(value: &str) -> String {
    let mut output = String::with_capacity(value.len() + 2);
    output.push('"');
    for ch in value.chars() {
        match ch {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            ch if ch <= '\u{1f}' => output.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => output.push(ch),
        }
    }
    output.push('"');
    output
}

// AI-FUNC-SUMMARY: Serializes an optional string to JSON; returns string or null JSON text; side effects: none.
fn json_optional_string(value: Option<&str>) -> String {
    value.map(json_string).unwrap_or_else(|| "null".to_string())
}

// AI-FUNC-SUMMARY: Serializes an optional 32-bit count for JSON output; returns the number or null; side effects: none.
fn json_optional_u32(value: Option<u32>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_string())
}
// AI-FUNC-SUMMARY: Serializes an optional usize to JSON; returns number or null JSON text; side effects: none.
fn json_optional_usize(value: Option<usize>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_string())
}

// AI-FUNC-SUMMARY: Serializes an optional u64 to JSON; returns number or null JSON text; side effects: none.
fn json_optional_u64(value: Option<u64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_string())
}

// AI-FUNC-SUMMARY: Serializes a finite float to JSON; returns a number string with fallback for non-finite values; side effects: none.
fn json_f64(value: f64) -> String {
    if value.is_finite() {
        format!("{value:.3}")
    } else {
        "0.000".to_string()
    }
}

// AI-FUNC-SUMMARY: Maps a log level to its JSON label; returns static label; side effects: none.
fn log_level_label(level: LogLevel) -> &'static str {
    match level {
        LogLevel::Info => "info",
        LogLevel::Warning => "warning",
        LogLevel::Error => "error",
    }
}

// AI-FUNC-SUMMARY: Maps a file status to its JSON label; returns static label; side effects: none.
fn file_status_label(status: FileStatus) -> &'static str {
    match status {
        FileStatus::Converted => "converted",
        FileStatus::Failed => "failed",
    }
}

// AI-FUNC-SUMMARY: Runs the MyVidComp command-line workflow; returns process result through normal CLI control flow; side effects: reads config, runs ffmpeg/ffprobe, writes logs and files.
pub fn main_entry() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

// AI-FUNC-SUMMARY: Runs the MyVidComp command-line workflow; returns process result through normal CLI control flow; side effects: reads config, starts optional graceful-exit listener, runs ffmpeg/ffprobe, writes logs and files.
fn run() -> Result<(), String> {
    let cli = Cli::parse(env::args().skip(1))?;
    if cli.help {
        println!("{}", usage());
        return Ok(());
    }

    if cli.version {
        println!("{}", version_label());
        return Ok(());
    }

    eprintln!("{}", version_label());

    // The review commands work on files an earlier run left behind, so they
    // never start a conversion.
    if cli.list_reviews {
        return print_reviews(&cli.target_folder);
    }
    if let Some(decision) = cli.resolve_reviews.clone() {
        return resolve_all_reviews(&cli.target_folder, &decision);
    }

    let mut events = NoopEventSink;
    run_workflow(
        RunOptions::from(cli),
        &mut events,
        CancellationToken::new(),
        true,
    )?;
    Ok(())
}

// AI-FUNC-SUMMARY: Prints every conversion in a folder that is waiting for a decision; returns success; side effects: reads the folder and writes to stdout.
fn print_reviews(folder: &Path) -> Result<(), String> {
    let items = list_reviews(folder);
    if items.is_empty() {
        println!("Nothing is waiting for a decision.");
        return Ok(());
    }

    println!("{} conversion(s) waiting for a decision:", items.len());
    for item in &items {
        println!("  {}", review::review_summary_line(item));
        for deviation in &item.deviations {
            println!("    - {}", deviation.detail);
        }
        println!(
            "    keep the new file:  {}",
            item.review_path.to_string_lossy()
        );
        println!(
            "    keep the original:  {}",
            item.original_path.to_string_lossy()
        );
    }
    println!(
        "\nApply one decision to all of them with --resolve-reviews keep-new|keep-original|keep-both."
    );
    Ok(())
}

// AI-FUNC-SUMMARY: Applies one decision to every pending review in a folder; returns success or the first failure; side effects: renames and deletes reviewed files.
fn resolve_all_reviews(folder: &Path, decision: &str) -> Result<(), String> {
    // Reject a bad decision before touching anything.
    review::parse_review_decision(decision)?;

    let items = list_reviews(folder);
    if items.is_empty() {
        println!("Nothing is waiting for a decision.");
        return Ok(());
    }

    let mut resolved = 0usize;
    let mut failures = Vec::new();
    for item in &items {
        match resolve_review(&item.sidecar_path, decision) {
            Ok(()) => {
                resolved += 1;
                println!("  {}", item.original_path.to_string_lossy());
            }
            Err(err) => failures.push(err),
        }
    }

    println!("Resolved {resolved} of {} conversion(s).", items.len());
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures.join("; "))
    }
}

// AI-FUNC-SUMMARY:
// Purpose: Runs MyVidComp from an embedded caller using structured events instead of terminal UI.
// Inputs: Run options, event sink, and cancellation token.
// Returns: Final conversion summary or an error message.
// Side effects: Scans files, runs ffmpeg/ffprobe, writes temporary and output files, and emits workflow events.
// Notes: Cancellation is graceful; the active file is allowed to finish before the next-file boundary observes the stop request.
pub fn run_with_events(
    options: RunOptions,
    events: &mut dyn EventSink,
    cancellation: CancellationToken,
) -> Result<RunSummary, String> {
    run_workflow(options, events, cancellation, false)
}

// AI-FUNC-SUMMARY:
// Purpose: Executes the shared MyVidComp workflow for both terminal CLI and embedded callers.
// Inputs: Run options, event sink, cancellation token, and whether terminal output should be rendered.
// Returns: Final conversion summary or an error message.
// Side effects: Scans files, runs ffmpeg/ffprobe, writes temporary and output files, may start the CLI graceful-exit listener, and may print terminal UI.
// Notes: The workflow checks cancellation only before starting each file so active ffmpeg work is not interrupted.
fn run_workflow(
    options: RunOptions,
    events: &mut dyn EventSink,
    cancellation: CancellationToken,
    terminal: bool,
) -> Result<RunSummary, String> {
    validate_run_options(&options)?;
    events.on_event(Event::Log {
        level: LogLevel::Info,
        message: version_label(),
    });
    events.on_event(Event::SettingsSelected {
        codec: options.target_codec,
        preservation: options.preservation,
        encoder_preference: options.encoder_preference,
        output_format: options.output_format,
        quality_mode: options.quality_mode,
        quality_check: options.quality_check,
        quality_target: options.quality_target,
        review_threshold: options.review_threshold(),
    });
    if terminal {
        eprintln!("Output codec: {}", options.target_codec.label());
        eprintln!("Preservation: {}", options.preservation.label());
        eprintln!("Encoders: {}", options.encoder_preference.label());
        eprintln!("Container: {}", options.output_format.label());
        eprintln!(
            "Quality: target {} by {}",
            format_quality(options.quality_target),
            options.quality_mode.label()
        );
    }

    ensure_binary(&options.ffmpeg, "ffmpeg")?;
    ensure_binary(&options.ffprobe, "ffprobe")?;

    let encoders = EncoderSelection::detect_with_events(
        &options.ffmpeg,
        options.encoder.as_deref(),
        options.target_codec,
        events,
        terminal,
    )?;
    let preferred_encoder = encoders.preferred();
    events.on_event(Event::EncoderSelected {
        encoder: preferred_encoder.name.clone(),
    });
    events.on_event(Event::ScanStarted {
        target_folder: options.target_folder.clone(),
    });

    let files = discover_files(&options.target_folder).map_err(|err| {
        format!(
            "failed to scan {}: {err}",
            options.target_folder.to_string_lossy()
        )
    })?;

    let mut skipped = 0usize;
    let mut candidates = Vec::new();
    if let Some(tmp_dir) = &options.tmp_dir {
        ensure_tmp_dir(tmp_dir)?;
    }

    for path in files {
        if has_added_suffix(&path, ".old") {
            skipped += 1;
            events.on_event(Event::FileSkipped {
                path,
                reason: "kept copy of an already converted file".to_string(),
            });
            continue;
        }

        // Both halves of a pending review are left alone until the user says
        // which one to keep: converting the review file again would be
        // circular, and converting the original again would produce a second
        // pending pair.
        if review::is_review_artifact(&path) || review::has_pending_review(&path) {
            skipped += 1;
            events.on_event(Event::FileSkipped {
                path,
                reason: "waiting for you to choose which copy to keep".to_string(),
            });
            continue;
        }

        if is_candidate_video_path(&path) {
            candidates.push(CandidateItem::new(path));
        }
    }
    events.on_event(Event::ScanFinished {
        candidates: candidates.len(),
        skipped,
    });

    if options.dry_run {
        if let Some(message) = count_candidate_info(options.count, candidates.len()) {
            if terminal {
                println!("{message}");
            }
            events.on_event(Event::Log {
                level: LogLevel::Info,
                message,
            });
        }
        if terminal {
            print_dry_run(&candidates, skipped, preferred_encoder, options.count);
        }
        let previews = candidates
            .iter()
            .map(|candidate| CandidatePreview {
                input_path: candidate.input_path.clone(),
                output_path: candidate.output_path(),
            })
            .collect();
        events.on_event(Event::DryRun {
            encoder: preferred_encoder.name.clone(),
            limit: conversion_limit(options.count),
            candidates: previews,
            skipped,
        });
        let summary = RunSummary {
            skipped,
            ..RunSummary::default()
        };
        events.on_event(Event::Summary {
            summary: summary.clone(),
        });
        return Ok(summary);
    }

    if candidates.is_empty() {
        let message = format!("No candidate video files found. Skipped videos: {skipped}.");
        if terminal {
            println!("{message}");
        }
        events.on_event(Event::Log {
            level: LogLevel::Info,
            message,
        });
        let summary = RunSummary {
            skipped,
            ..RunSummary::default()
        };
        events.on_event(Event::Summary {
            summary: summary.clone(),
        });
        return Ok(summary);
    }

    let graceful_exit = if terminal {
        Some(GracefulExit::start(cancellation.clone()))
    } else {
        None
    };
    let prompt_active = graceful_exit
        .as_ref()
        .map(GracefulExit::prompt_active_flag)
        .unwrap_or_else(|| Arc::new(AtomicBool::new(false)));
    let mut ui = ProgressUi::new(
        conversion_limit(options.count),
        prompt_active,
        events,
        terminal,
    );

    // Ask once whether this ffmpeg can measure quality. Without it the run
    // still works, it just cannot score results or judge a flexible conversion.
    let (support, reason) = vmaf::detect_vmaf_support(&options.ffmpeg);
    ui.set_codec(options.target_codec);
    ui.set_quality_available(support.available);
    if let Some(reason) = reason {
        ui.emit_capability_missing("libvmaf", &reason);
    }

    let mut summary = RunSummary {
        skipped,
        ..RunSummary::default()
    };

    for candidate in &candidates {
        if cancellation.is_stop_requested() {
            break;
        }

        if conversion_limit_reached(options.count, summary.conversion_attempts()) {
            break;
        }

        let Some(video) = probe_video(&options.ffprobe, &candidate.input_path)? else {
            summary.skipped += 1;
            ui.emit_file_skipped(&candidate.input_path, "no readable video stream");
            continue;
        };

        if video
            .codec_name
            .eq_ignore_ascii_case(options.target_codec.ffprobe_name())
        {
            summary.skipped += 1;
            ui.emit_file_skipped(
                &candidate.input_path,
                &format!("already {}", options.target_codec.label()),
            );
            continue;
        }

        let streams = match probe_streams(&options.ffprobe, &candidate.input_path) {
            Ok(streams) => streams,
            Err(err) => {
                summary.skipped += 1;
                ui.emit_file_skipped(
                    &candidate.input_path,
                    &format!("cannot safely inspect stream layout: {err}"),
                );
                continue;
            }
        };
        let chapters = match probe_chapters(&options.ffprobe, &candidate.input_path) {
            Ok(chapters) => chapters,
            Err(err) => {
                summary.skipped += 1;
                ui.emit_file_skipped(
                    &candidate.input_path,
                    &format!("cannot safely inspect chapter metadata: {err}"),
                );
                continue;
            }
        };
        let item = match WorkItem::new(
            candidate.input_path.clone(),
            video,
            streams,
            chapters,
            ItemPolicy::from_options(&options),
        ) {
            Ok(item) => item,
            Err(reason) => {
                summary.skipped += 1;
                ui.emit_file_skipped(&candidate.input_path, &skip_reason_label(&reason));
                continue;
            }
        };

        match &item.chapter_policy {
            ChapterPolicy::ConfirmedEmpty => ui.log_info(format!(
                "Info: suppressing confirmed empty QuickTime chapter carrier in {}",
                item.input_path.to_string_lossy()
            )),
            ChapterPolicy::ConfirmedMeaningful(chapters) => ui.log_info(format!(
                "Info: re-authoring {} chapter(s) from confirmed QuickTime chapter carrier in {}",
                chapters.len(),
                item.input_path.to_string_lossy()
            )),
            ChapterPolicy::Ordinary => {}
        }

        if let Some(graceful_exit) = &graceful_exit {
            graceful_exit.wait_until_prompt_inactive();
        }
        if cancellation.is_stop_requested() {
            break;
        }

        ui.start_file(summary.conversion_attempts() + 1, &item, preferred_encoder);
        match transcode_item(&options, &item, &encoders, &mut ui) {
            Ok(result) => {
                summary.converted += 1;
                summary.source_bytes += result.source_bytes;
                summary.output_bytes += result.output_bytes;
                let message = format!("converted; size {}", conversion_size_label(result));
                ui.emit_file_finished(
                    &item.input_path,
                    FileStatus::Converted,
                    &message,
                    Some(result),
                );
                ui.finish_file(&message);
            }
            Err(err) => {
                summary.failed += 1;
                ui.emit_file_finished(&item.input_path, FileStatus::Failed, "failed", None);
                ui.finish_file("failed");
                ui.log_error(format!(
                    "failed: {}: {err}",
                    item.input_path.to_string_lossy()
                ));
            }
        }
    }

    if let Some(graceful_exit) = &graceful_exit {
        graceful_exit.wait_until_prompt_inactive();
    }
    if cancellation.is_stop_requested() {
        ui.emit_stop_requested();
        if terminal {
            println!("Graceful exit requested; stopped after the current file.");
        }
    }

    if let Some(message) = count_limit_info(options.count, summary.conversion_attempts()) {
        if terminal {
            println!("{message}");
        }
        ui.emit_log(LogLevel::Info, message);
    }
    ui.emit_summary(&summary);
    if terminal {
        print_summary(&summary);
    }
    Ok(summary)
}

// AI-FUNC-SUMMARY: Validates embedded run options before workflow side effects; returns success or a user-facing error; side effects: inspects target directory metadata.
fn validate_run_options(options: &RunOptions) -> Result<(), String> {
    if options.count < -1 {
        return Err("count must be -1 or greater".to_string());
    }

    if !options.target_folder.is_dir() {
        return Err(format!(
            "target folder does not exist or is not a directory: {}",
            options.target_folder.to_string_lossy()
        ));
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Cli {
    target_folder: PathBuf,
    count: isize,
    keep_original: bool,
    dry_run: bool,
    help: bool,
    version: bool,
    ffmpeg: String,
    ffprobe: String,
    tmp_dir: Option<PathBuf>,
    encoder: Option<String>,
    encoder_preference: EncoderPreference,
    output_format: OutputFormat,
    target_codec: TargetCodec,
    preservation: Preservation,
    quality_mode: QualityMode,
    quality_check: QualityCheck,
    quality_target: u32,
    review_margin: u32,
    quality_threads: u32,
    list_reviews: bool,
    resolve_reviews: Option<String>,
}

impl Cli {
    // AI-FUNC-SUMMARY: Parses parse input; returns parsed values or errors; side effects: none.
    fn parse<I, S>(args: I) -> Result<Self, String>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self::parse_with_default_config(args, Path::new("config.yaml"))
    }

    // AI-FUNC-SUMMARY: Parses with default config input; returns parsed values or errors; side effects: none.
    fn parse_with_default_config<I, S>(args: I, default_config_path: &Path) -> Result<Self, String>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut target_folder = None;
        let mut count = None;
        let mut keep_original = None;
        let mut dry_run = None;
        let mut help = false;
        let mut version = false;
        let mut ffmpeg_override = None;
        let mut ffprobe_override = None;
        let mut tmp_dir = None;
        let mut encoder_override = None;
        let mut encoder_preference_override = None;
        let mut output_format_override = None;
        let mut target_codec_override = None;
        let mut preservation_override = None;
        let mut quality_mode_override = None;
        let mut quality_check_override = None;
        let mut quality_target_override = None;
        let mut review_margin_override = None;
        let mut quality_threads_override = None;
        let mut list_reviews = false;
        let mut resolve_reviews = None;
        let mut config_path = None;

        let args = args.into_iter().map(Into::into).collect::<Vec<String>>();
        let no_cli_args = args.is_empty();
        let mut iter = args.into_iter().peekable();
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "-h" | "--help" => help = true,
                "-V" | "--version" => version = true,
                "--dry-run" => dry_run = Some(true),
                "--no-dry-run" => dry_run = Some(false),
                "--keep-original" => keep_original = Some(true),
                "--no-keep-original" => keep_original = Some(false),
                "--count" => {
                    let raw = iter
                        .next()
                        .ok_or_else(|| "--count requires a value".to_string())?;
                    let parsed = raw
                        .parse::<isize>()
                        .map_err(|_| format!("invalid --count value: {raw}"))?;
                    if parsed < -1 {
                        return Err("--count must be -1 or greater".to_string());
                    }
                    count = Some(parsed);
                }
                "--config" => {
                    config_path = Some(PathBuf::from(
                        iter.next()
                            .ok_or_else(|| "--config requires a path".to_string())?,
                    ));
                }
                "--ffmpeg" => {
                    ffmpeg_override = Some(
                        iter.next()
                            .ok_or_else(|| "--ffmpeg requires a path".to_string())?,
                    );
                }
                "--ffprobe" => {
                    ffprobe_override = Some(
                        iter.next()
                            .ok_or_else(|| "--ffprobe requires a path".to_string())?,
                    );
                }
                "--tmp-dir" => {
                    tmp_dir = Some(PathBuf::from(
                        iter.next()
                            .ok_or_else(|| "--tmp-dir requires a path".to_string())?,
                    ));
                }
                "--encoder" => {
                    let raw = iter
                        .next()
                        .ok_or_else(|| "--encoder requires a value".to_string())?;
                    encoder_override = Some(normalize_encoder_choice(&raw));
                }
                // --conversion-mode was the name before encoder choice and
                // preservation became separate settings. Keep accepting it.
                "--encoder-preference" | "--conversion-mode" => {
                    let raw = iter
                        .next()
                        .ok_or_else(|| format!("{arg} requires a value"))?;
                    encoder_preference_override = Some(normalize_encoder_preference_choice(&raw)?);
                }
                "--output-format" => {
                    let raw = iter
                        .next()
                        .ok_or_else(|| "--output-format requires a value".to_string())?;
                    output_format_override = Some(normalize_output_format_choice(&raw)?);
                }
                "--codec" => {
                    let raw = iter
                        .next()
                        .ok_or_else(|| "--codec requires a value".to_string())?;
                    target_codec_override = Some(normalize_target_codec_choice(&raw)?);
                }
                "--preservation" => {
                    let raw = iter
                        .next()
                        .ok_or_else(|| "--preservation requires a value".to_string())?;
                    preservation_override = Some(normalize_preservation_choice(&raw)?);
                }
                "--quality-mode" => {
                    let raw = iter
                        .next()
                        .ok_or_else(|| "--quality-mode requires a value".to_string())?;
                    quality_mode_override = Some(normalize_quality_mode_choice(&raw)?);
                }
                "--quality-check" => {
                    let raw = iter
                        .next()
                        .ok_or_else(|| "--quality-check requires a value".to_string())?;
                    quality_check_override = Some(normalize_quality_check_choice(&raw)?);
                }
                "--quality" => {
                    let raw = iter
                        .next()
                        .ok_or_else(|| "--quality requires a value".to_string())?;
                    quality_target_override = Some(parse_quality_target(&raw)?);
                }
                "--review-margin" => {
                    let raw = iter
                        .next()
                        .ok_or_else(|| "--review-margin requires a value".to_string())?;
                    review_margin_override = Some(parse_quality_points(&raw)?);
                }
                "--quality-threads" => {
                    let raw = iter
                        .next()
                        .ok_or_else(|| "--quality-threads requires a value".to_string())?;
                    quality_threads_override = Some(
                        raw.parse::<u32>()
                            .map_err(|_| format!("invalid --quality-threads value: {raw}"))?,
                    );
                }
                "--list-reviews" => list_reviews = true,
                "--resolve-reviews" => {
                    resolve_reviews = Some(
                        iter.next()
                            .ok_or_else(|| "--resolve-reviews requires a decision".to_string())?,
                    );
                }
                _ if arg.starts_with('-') => {
                    return Err(format!("unknown option: {arg}"));
                }
                _ => {
                    if target_folder.replace(PathBuf::from(&arg)).is_some() {
                        return Err(format!("unexpected extra positional argument: {arg}"));
                    }
                }
            }
        }

        if help || version {
            return Ok(Self {
                target_folder: PathBuf::new(),
                count: count.unwrap_or(-1),
                keep_original: keep_original.unwrap_or(true),
                dry_run: dry_run.unwrap_or(false),
                help,
                version,
                ffmpeg: ffmpeg_override.unwrap_or_else(|| default_runtime_binary("ffmpeg")),
                ffprobe: ffprobe_override.unwrap_or_else(|| default_runtime_binary("ffprobe")),
                tmp_dir,
                encoder: encoder_override.flatten(),
                encoder_preference: encoder_preference_override.flatten().unwrap_or_default(),
                output_format: output_format_override.flatten().unwrap_or_default(),
                target_codec: target_codec_override.flatten().unwrap_or_default(),
                preservation: preservation_override.flatten().unwrap_or_default(),
                quality_mode: quality_mode_override.flatten().unwrap_or_default(),
                quality_check: quality_check_override.flatten().unwrap_or_default(),
                quality_target: quality_target_override.unwrap_or(DEFAULT_QUALITY_TARGET),
                review_margin: review_margin_override.unwrap_or(DEFAULT_REVIEW_MARGIN),
                quality_threads: quality_threads_override.unwrap_or(0),
                list_reviews,
                resolve_reviews,
            });
        }

        let config_path = config_path.unwrap_or_else(|| default_config_path.to_path_buf());
        let should_read_config = config_path.is_file() || no_cli_args;
        let config = if should_read_config {
            Config::read(&config_path)?
        } else {
            Config::default()
        };

        let target_folder = target_folder
            .or(config.target_folder)
            .ok_or_else(|| missing_target_message(&config_path, no_cli_args))?;

        if !target_folder.is_dir() {
            return Err(format!(
                "target folder does not exist or is not a directory: {}",
                target_folder.to_string_lossy()
            ));
        }

        Ok(Self {
            target_folder,
            count: count.or(config.count).unwrap_or(-1),
            keep_original: keep_original.or(config.keep_original).unwrap_or(true),
            dry_run: dry_run.or(config.dry_run).unwrap_or(false),
            help,
            version,
            ffmpeg: ffmpeg_override
                .or(config.ffmpeg)
                .unwrap_or_else(|| default_runtime_binary("ffmpeg")),
            ffprobe: ffprobe_override
                .or(config.ffprobe)
                .unwrap_or_else(|| default_runtime_binary("ffprobe")),
            tmp_dir: tmp_dir.or(config.tmp_dir),
            encoder: encoder_override.unwrap_or(config.encoder),
            encoder_preference: encoder_preference_override
                .unwrap_or(config.encoder_preference)
                .unwrap_or_default(),
            output_format: output_format_override
                .unwrap_or(config.output_format)
                .unwrap_or_default(),
            target_codec: target_codec_override
                .unwrap_or(config.target_codec)
                .unwrap_or_default(),
            preservation: preservation_override
                .unwrap_or(config.preservation)
                .unwrap_or_default(),
            quality_mode: quality_mode_override
                .unwrap_or(config.quality_mode)
                .unwrap_or_default(),
            quality_check: quality_check_override
                .unwrap_or(config.quality_check)
                .unwrap_or_default(),
            quality_target: quality_target_override
                .or(config.quality_target)
                .unwrap_or(DEFAULT_QUALITY_TARGET),
            review_margin: review_margin_override
                .or(config.review_margin)
                .unwrap_or(DEFAULT_REVIEW_MARGIN),
            quality_threads: quality_threads_override
                .or(config.quality_threads)
                .unwrap_or(0),
            list_reviews,
            resolve_reviews,
        })
    }
}

impl From<Cli> for RunOptions {
    // AI-FUNC-SUMMARY: Converts parsed CLI options into embedded run options; returns public run options; side effects: none.
    fn from(cli: Cli) -> Self {
        Self {
            target_folder: cli.target_folder,
            count: cli.count,
            keep_original: cli.keep_original,
            dry_run: cli.dry_run,
            ffmpeg: cli.ffmpeg,
            ffprobe: cli.ffprobe,
            tmp_dir: cli.tmp_dir,
            encoder: cli.encoder,
            encoder_preference: cli.encoder_preference,
            output_format: cli.output_format,
            target_codec: cli.target_codec,
            preservation: cli.preservation,
            quality_mode: cli.quality_mode,
            quality_check: cli.quality_check,
            quality_target: cli.quality_target,
            review_margin: cli.review_margin,
            quality_threads: cli.quality_threads,
        }
    }
}

// AI-FUNC-SUMMARY: Builds or derives usage data; returns the computed value; side effects: none.
fn usage() -> &'static str {
    "Usage: myvidcomp <folder> [options]

Converts every video in a folder to a smaller file, checks how close the result
looks to the original, and never deletes anything it is not sure about.

What to produce
  --codec av1|hevc|vvc        Output codec. AV1 makes the smallest files, H.265
                              plays on the widest range of devices, H.266 is
                              experimental and very slow. Default: av1.
  --output-format mp4|mkv-fallback
                              Use MKV when a stream cannot go in an MP4.
                              Default: mp4.

How careful to be
  --preservation strict|flexible
                              strict converts only when everything is kept
                              exactly. flexible converts anyway when only
                              recoverable details would change, and keeps both
                              files so you can choose. Default: flexible.
  --quality N                 Quality target from 50 to 100. Default: 95.
  --quality-mode search|estimate
                              search encodes short samples to find the setting
                              that just reaches the target. estimate guesses
                              from the source and skips the test encodes.
                              Default: search.
  --quality-check full|sampled|off
                              How thoroughly to compare the result with the
                              original. Default: sampled.
  --review-margin N           How far below the target a result may land before
                              it is kept for review. Default: 2.

Which encoder
  --encoder-preference auto|gpu|cpu
                              auto uses a graphics card only when it can keep
                              the source exactly. gpu prefers speed. cpu makes
                              the smallest files. Default: auto.
  --encoder NAME              Use one specific encoder and no other.

Files
  --count N                   Convert at most N files. -1 means all.
  --keep-original             Rename each original to .old instead of deleting.
  --no-keep-original          Replace each original once the result is verified.
  --dry-run                   List what would be converted and stop.
  --tmp-dir PATH              Where to write working files.
  --config PATH               Read settings from this file.
  --ffmpeg PATH, --ffprobe PATH
                              Use these tools instead of the bundled ones.

Reviewing results
  --list-reviews              List conversions waiting for a decision.
  --resolve-reviews keep-new|keep-original|keep-both
                              Apply one decision to all of them.

  -h, --help                  Show this text.
  -V, --version               Show the version."
}

// AI-FUNC-SUMMARY: Builds or derives version label data; returns the computed value; side effects: none.
fn version_label() -> String {
    format!("myvidcomp {}", env!("CARGO_PKG_VERSION"))
}

// AI-FUNC-SUMMARY: Checks missing target message predicate; returns a boolean; side effects: none.
fn missing_target_message(config_path: &Path, no_cli_args: bool) -> String {
    if no_cli_args {
        format!(
            "missing target folder: pass one on the CLI or set target_folder in {}\n\n{}",
            config_path.to_string_lossy(),
            usage()
        )
    } else {
        format!("missing target folder\n\n{}", usage())
    }
}

// AI-FUNC-SUMMARY: Provides count limit info behavior; returns the declared result; side effects: see implementation.
fn count_limit_info(count: isize, eligible_count: usize) -> Option<String> {
    if count >= 0 && count as usize > eligible_count {
        Some(format!(
            "Info: requested {count} conversion(s), only {eligible_count} eligible video(s) need conversion."
        ))
    } else {
        None
    }
}

// AI-FUNC-SUMMARY: Provides count candidate info behavior; returns the declared result; side effects: see implementation.
fn count_candidate_info(count: isize, candidate_count: usize) -> Option<String> {
    if count >= 0 && count as usize > candidate_count {
        Some(format!(
            "Info: requested {count} conversion(s), only {candidate_count} candidate video file(s) were found before probing."
        ))
    } else {
        None
    }
}

// AI-FUNC-SUMMARY: Provides conversion limit behavior; returns the declared result; side effects: see implementation.
fn conversion_limit(count: isize) -> Option<usize> {
    if count >= 0 {
        Some(count as usize)
    } else {
        None
    }
}

// AI-FUNC-SUMMARY: Provides conversion limit reached behavior; returns the declared result; side effects: see implementation.
fn conversion_limit_reached(count: isize, attempts: usize) -> bool {
    conversion_limit(count).is_some_and(|limit| attempts >= limit)
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct Config {
    target_folder: Option<PathBuf>,
    count: Option<isize>,
    keep_original: Option<bool>,
    dry_run: Option<bool>,
    ffmpeg: Option<String>,
    ffprobe: Option<String>,
    tmp_dir: Option<PathBuf>,
    encoder: Option<String>,
    encoder_preference: Option<EncoderPreference>,
    output_format: Option<OutputFormat>,
    target_codec: Option<TargetCodec>,
    preservation: Option<Preservation>,
    quality_mode: Option<QualityMode>,
    quality_check: Option<QualityCheck>,
    quality_target: Option<u32>,
    review_margin: Option<u32>,
    quality_threads: Option<u32>,
}

impl Config {
    // AI-FUNC-SUMMARY: Provides read behavior; returns the declared result; side effects: see implementation.
    fn read(path: &Path) -> Result<Self, String> {
        let text = fs::read_to_string(path)
            .map_err(|err| format!("failed to read config {}: {err}", path.to_string_lossy()))?;
        parse_config_yaml(&text)
            .map_err(|err| format!("failed to parse config {}: {err}", path.to_string_lossy()))
    }
}

// AI-FUNC-SUMMARY: Parses config yaml input; returns parsed values or errors; side effects: none.
fn parse_config_yaml(text: &str) -> Result<Config, String> {
    let mut config = Config::default();

    for (index, raw_line) in text.lines().enumerate() {
        let line_number = index + 1;
        let line = strip_yaml_comment(raw_line).trim();
        if line.is_empty() || line == "---" {
            continue;
        }

        if raw_line.starts_with(' ') || raw_line.starts_with('\t') {
            return Err(format!(
                "line {line_number}: nested YAML is not supported; use top-level key: value pairs"
            ));
        }

        let Some((key, value)) = line.split_once(':') else {
            return Err(format!("line {line_number}: expected key: value"));
        };

        let key = key.trim();
        let raw_value = value.trim();

        if key == "encoder" || key == "av1_encoder" {
            let value = if raw_value.is_empty() {
                String::new()
            } else {
                parse_yaml_scalar(raw_value).map_err(|err| format!("line {line_number}: {err}"))?
            };
            config.encoder = normalize_encoder_choice(&value);
            continue;
        }

        // Every enum-valued key accepts an empty value meaning "use the
        // default", so a shipped template can list a key without committing to
        // a value. `conversion_mode` is the old name for `encoder_preference`.
        if key == "encoder_preference" || key == "conversion_mode" {
            let value = optional_yaml_value(raw_value, line_number)?;
            config.encoder_preference = normalize_encoder_preference_choice(&value)
                .map_err(|err| format!("line {line_number}: {err}"))?;
            continue;
        }

        if key == "output_format" {
            let value = optional_yaml_value(raw_value, line_number)?;
            config.output_format = normalize_output_format_choice(&value)
                .map_err(|err| format!("line {line_number}: {err}"))?;
            continue;
        }

        if key == "target_codec" || key == "codec" {
            let value = optional_yaml_value(raw_value, line_number)?;
            config.target_codec = normalize_target_codec_choice(&value)
                .map_err(|err| format!("line {line_number}: {err}"))?;
            continue;
        }

        if key == "preservation" {
            let value = optional_yaml_value(raw_value, line_number)?;
            config.preservation = normalize_preservation_choice(&value)
                .map_err(|err| format!("line {line_number}: {err}"))?;
            continue;
        }

        if key == "quality_mode" {
            let value = optional_yaml_value(raw_value, line_number)?;
            config.quality_mode = normalize_quality_mode_choice(&value)
                .map_err(|err| format!("line {line_number}: {err}"))?;
            continue;
        }

        if key == "quality_check" {
            let value = optional_yaml_value(raw_value, line_number)?;
            config.quality_check = normalize_quality_check_choice(&value)
                .map_err(|err| format!("line {line_number}: {err}"))?;
            continue;
        }

        if key == "quality_target" {
            let value = optional_yaml_value(raw_value, line_number)?;
            if !value.trim().is_empty() {
                config.quality_target = Some(
                    parse_quality_target(&value)
                        .map_err(|err| format!("line {line_number}: {err}"))?,
                );
            }
            continue;
        }

        if key == "review_margin" {
            let value = optional_yaml_value(raw_value, line_number)?;
            if !value.trim().is_empty() {
                config.review_margin = Some(
                    parse_quality_points(&value)
                        .map_err(|err| format!("line {line_number}: {err}"))?,
                );
            }
            continue;
        }

        if key == "quality_threads" {
            let value = optional_yaml_value(raw_value, line_number)?;
            if !value.trim().is_empty() {
                config.quality_threads = Some(value.trim().parse::<u32>().map_err(|_| {
                    format!("line {line_number}: invalid quality_threads value: {value}")
                })?);
            }
            continue;
        }

        let value =
            parse_yaml_scalar(raw_value).map_err(|err| format!("line {line_number}: {err}"))?;

        match key {
            "target_folder" | "target" => config.target_folder = Some(PathBuf::from(value)),
            "count" => {
                let parsed = value
                    .parse::<isize>()
                    .map_err(|_| format!("invalid count value: {value}"))?;
                if parsed < -1 {
                    return Err("count must be -1 or greater".to_string());
                }
                config.count = Some(parsed);
            }
            "keep_original" => config.keep_original = Some(parse_yaml_bool(&value)?),
            "dry_run" => config.dry_run = Some(parse_yaml_bool(&value)?),
            "tmp_dir" | "temporary_dir" | "temp_dir" => config.tmp_dir = Some(PathBuf::from(value)),
            "ffmpeg" => config.ffmpeg = Some(value),
            "ffprobe" => config.ffprobe = Some(value),
            _ => return Err(format!("line {line_number}: unknown key: {key}")),
        }
    }

    Ok(config)
}

// AI-FUNC-SUMMARY: Provides strip yaml comment behavior; returns the declared result; side effects: see implementation.
fn strip_yaml_comment(line: &str) -> &str {
    let mut single_quoted = false;
    let mut double_quoted = false;
    let mut escaped = false;

    for (index, ch) in line.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }

        match ch {
            '\\' if double_quoted => escaped = true,
            '\'' if !double_quoted => single_quoted = !single_quoted,
            '"' if !single_quoted => double_quoted = !double_quoted,
            '#' if !single_quoted && !double_quoted => return &line[..index],
            _ => {}
        }
    }

    line
}

// AI-FUNC-SUMMARY: Reads a config value that may be left blank; returns the scalar or an empty string; side effects: none.
fn optional_yaml_value(raw_value: &str, line_number: usize) -> Result<String, String> {
    if raw_value.is_empty() {
        Ok(String::new())
    } else {
        parse_yaml_scalar(raw_value).map_err(|err| format!("line {line_number}: {err}"))
    }
}
// AI-FUNC-SUMMARY: Parses yaml scalar input; returns parsed values or errors; side effects: none.
fn parse_yaml_scalar(value: &str) -> Result<String, String> {
    if value.is_empty() {
        return Err("missing value".to_string());
    }

    if let Some(stripped) = value.strip_prefix('"') {
        let Some(inner) = stripped.strip_suffix('"') else {
            return Err("unterminated double-quoted string".to_string());
        };
        return Ok(unescape_double_quoted_yaml_scalar(inner));
    }

    if let Some(stripped) = value.strip_prefix('\'') {
        let Some(inner) = stripped.strip_suffix('\'') else {
            return Err("unterminated single-quoted string".to_string());
        };
        return Ok(inner.replace("''", "'"));
    }

    Ok(value.to_string())
}

// AI-FUNC-SUMMARY: Provides unescape double quoted yaml scalar behavior; returns the declared result; side effects: see implementation.
fn unescape_double_quoted_yaml_scalar(value: &str) -> String {
    if value.starts_with("\\\\") && !value.starts_with("\\\\\\\\") {
        let tail = value
            .trim_start_matches('\\')
            .replace("\\\"", "\"")
            .replace("\\\\", "\\");
        format!("\\\\{tail}")
    } else {
        value.replace("\\\"", "\"").replace("\\\\", "\\")
    }
}

// AI-FUNC-SUMMARY: Parses yaml bool input; returns parsed values or errors; side effects: none.
fn parse_yaml_bool(value: &str) -> Result<bool, String> {
    match value.to_ascii_lowercase().as_str() {
        "true" | "yes" | "on" | "1" => Ok(true),
        "false" | "no" | "off" | "0" => Ok(false),
        _ => Err(format!("invalid boolean value: {value}")),
    }
}

// AI-FUNC-SUMMARY: Builds or derives normalize encoder choice data; returns the computed value; side effects: none.
fn normalize_encoder_choice(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty()
        || value.eq_ignore_ascii_case("auto")
        || value.eq_ignore_ascii_case("default")
    {
        None
    } else {
        // The codec is not known yet at this point, so the alias is expanded
        // later when the encoder table for the chosen codec is consulted.
        Some(value.to_ascii_lowercase())
    }
}

#[derive(Debug, Clone, Default)]
struct VideoInfo {
    codec_name: String,
    profile: Option<String>,
    width: u32,
    height: u32,
    sample_aspect_ratio: Option<String>,
    display_aspect_ratio: Option<String>,
    field_order: Option<String>,
    fps: f64,
    nominal_fps: f64,
    avg_frame_rate: Option<String>,
    nominal_frame_rate: Option<String>,
    duration_seconds: f64,
    bit_rate: Option<u64>,
    pix_fmt: Option<String>,
    bits_per_raw_sample: Option<u8>,
    color_range: Option<String>,
    color_space: Option<String>,
    color_transfer: Option<String>,
    color_primaries: Option<String>,
    chroma_location: Option<String>,
    encoder_tag: Option<String>,
}

#[derive(Debug, Clone)]
struct StreamInfo {
    index: usize,
    codec_type: String,
    codec_name: String,
    codec_tag_string: Option<String>,
    track_id: Option<u32>,
    handler_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StreamSignature {
    codec_type: String,
    codec_name: String,
}

impl From<&StreamInfo> for StreamSignature {
    // AI-FUNC-SUMMARY: Constructs the associated value; returns a new instance; side effects: none unless validation reads path state.
    fn from(stream: &StreamInfo) -> Self {
        Self {
            codec_type: stream.codec_type.clone(),
            codec_name: stream.codec_name.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct ChapterInfo {
    start_seconds: f64,
    end_seconds: f64,
    title: String,
}

#[derive(Debug, Clone, PartialEq)]
enum ChapterPolicy {
    Ordinary,
    ConfirmedEmpty,
    ConfirmedMeaningful(Vec<ChapterInfo>),
}

impl ChapterPolicy {
    // AI-FUNC-SUMMARY: Returns FFmpeg's chapter input selector for this source policy; returns 0 only for meaningful confirmed chapters and -1 otherwise; side effects: none.
    fn map_value(&self) -> &'static str {
        match self {
            ChapterPolicy::ConfirmedMeaningful(_) => "0",
            ChapterPolicy::Ordinary | ChapterPolicy::ConfirmedEmpty => "-1",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CandidateItem {
    input_path: PathBuf,
}

impl CandidateItem {
    // AI-FUNC-SUMMARY: Constructs the associated value; returns a new instance; side effects: none unless validation reads path state.
    fn new(input_path: PathBuf) -> Self {
        Self { input_path }
    }

    // AI-FUNC-SUMMARY: Provides output path behavior; returns the declared result; side effects: see implementation.
    fn output_path(&self) -> PathBuf {
        self.input_path.with_extension("mp4")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputContainer {
    Mp4,
    Mkv,
}

impl OutputContainer {
    // AI-FUNC-SUMMARY: Returns the file extension used for this container; returns static extension without dot; side effects: none.
    fn extension(&self) -> &'static str {
        match self {
            OutputContainer::Mp4 => "mp4",
            OutputContainer::Mkv => "mkv",
        }
    }

    // AI-FUNC-SUMMARY: Returns the FFmpeg format flag for this container; returns static format name; side effects: none.
    fn ffmpeg_format(&self) -> &'static str {
        match self {
            OutputContainer::Mp4 => "mp4",
            OutputContainer::Mkv => "matroska",
        }
    }
}

#[derive(Debug, Clone)]
struct WorkItem {
    input_path: PathBuf,
    output_path: PathBuf,
    old_path: Option<PathBuf>,
    temp_path: PathBuf,
    recovery_path: PathBuf,
    output_container: OutputContainer,
    video: VideoInfo,
    mapped_stream_indexes: Vec<usize>,
    stream_signature: Vec<StreamSignature>,
    chapter_policy: ChapterPolicy,
    estimated_crf: u8,
}

/// The run settings that decide how one file is prepared for conversion.
#[derive(Debug, Clone, Copy)]
struct ItemPolicy<'a> {
    keep_original: bool,
    tmp_dir: Option<&'a Path>,
    output_format: OutputFormat,
    target_codec: TargetCodec,
    preservation: Preservation,
}

impl<'a> ItemPolicy<'a> {
    // AI-FUNC-SUMMARY: Extracts the per-file settings from a full set of run options; returns the policy; side effects: none.
    fn from_options(options: &'a RunOptions) -> Self {
        Self {
            keep_original: options.keep_original,
            tmp_dir: options.tmp_dir.as_deref(),
            output_format: options.output_format,
            target_codec: options.target_codec,
            preservation: options.preservation,
        }
    }
}

impl WorkItem {
    // AI-FUNC-SUMMARY: Constructs the associated value; returns a new instance or a skip reason; side effects: reads path state for conflict checks.
    fn new(
        input_path: PathBuf,
        video: VideoInfo,
        streams: Vec<StreamInfo>,
        chapters: Vec<ChapterInfo>,
        policy: ItemPolicy<'_>,
    ) -> Result<Self, SkipReason> {
        let ItemPolicy {
            keep_original,
            tmp_dir,
            output_format,
            target_codec,
            preservation,
        } = policy;
        let (streams, chapter_policy) =
            source_stream_policy(&input_path, streams, chapters, &video)
                .map_err(SkipReason::UnsafeReplacement)?;
        if streams.is_empty() || !streams.iter().any(|stream| stream.codec_type == "video") {
            return Err(SkipReason::UnsafeReplacement(
                "stream probe did not provide an indexed video stream for explicit mapping"
                    .to_string(),
            ));
        }
        let output_container = select_output_container(output_format, &streams)
            .map_err(SkipReason::UnsafeReplacement)?;

        let output_path = input_path.with_extension(output_container.extension());
        let old_path = if keep_original {
            Some(with_added_suffix(&input_path, ".old"))
        } else {
            None
        };
        let temp_path = temp_output_path(&input_path, tmp_dir, output_container);
        let recovery_path = with_added_suffix(&input_path, RECOVERY_SUFFIX);

        if output_path != input_path && output_path.exists() {
            return Err(SkipReason::Conflict);
        }

        if let Some(path) = &old_path
            && path.exists()
        {
            return Err(SkipReason::Conflict);
        }

        if temp_path.exists() {
            return Err(SkipReason::Conflict);
        }

        if recovery_path.exists() {
            return Err(SkipReason::Conflict);
        }

        if let Some(reason) = interlaced_source_reason(&video) {
            return Err(SkipReason::UnsafeReplacement(reason));
        }

        // Under strict preservation a codec that cannot record some colour
        // detail means the file is left alone. A flexible run converts it
        // anyway and reports the loss, because the picture itself is unchanged.
        if !preservation.allows_deviations()
            && let Some(reason) = unsupported_metadata_reason(target_codec, &video)
        {
            return Err(SkipReason::UnsafeReplacement(reason));
        }

        let estimated_crf = estimate_av1_crf(&video);
        let mapped_stream_indexes = streams.iter().map(|stream| stream.index).collect();
        let stream_signature = streams.iter().map(StreamSignature::from).collect();

        Ok(Self {
            input_path,
            output_path,
            old_path,
            temp_path,
            recovery_path,
            output_container,
            video,
            mapped_stream_indexes,
            stream_signature,
            chapter_policy,
            estimated_crf,
        })
    }
}

#[derive(Debug, Clone)]
enum SkipReason {
    Conflict,
    UnsafeReplacement(String),
}

// AI-FUNC-SUMMARY: Formats an internal skip reason for user and GUI reporting; returns a human-readable message; side effects: none.
fn skip_reason_label(reason: &SkipReason) -> String {
    match reason {
        SkipReason::Conflict => "output or backup path already exists".to_string(),
        SkipReason::UnsafeReplacement(detail) => detail.clone(),
    }
}

#[derive(Debug, Clone)]
struct Encoder {
    name: String,
    kind: EncoderKind,
}

impl Encoder {
    // AI-FUNC-SUMMARY: Checks whether an encoder is GPU-backed; returns true for hardware and Vulkan backends; side effects: none.
    fn is_gpu(&self) -> bool {
        matches!(self.kind, EncoderKind::Hardware | EncoderKind::Vulkan)
    }
}

#[derive(Debug, Clone)]
struct EncoderSelection {
    candidates: Vec<Encoder>,
    explicit: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EncoderKind {
    Hardware,
    Vulkan,
    SvtAv1,
    LibAom,
    Rav1e,
    X265,
    Vvenc,
    MediaCodec,
}

impl EncoderSelection {
    // AI-FUNC-SUMMARY: Detects all usable AV1 encoders and reports runtime probe diagnostics; returns an ordered selection or an error; side effects: runs ffmpeg probes and may emit terminal/event logs.
    fn detect_with_events(
        ffmpeg: &str,
        requested_encoder: Option<&str>,
        codec: TargetCodec,
        events: &mut dyn EventSink,
        terminal: bool,
    ) -> Result<Self, String> {
        let output = process_command(ffmpeg)
            .args(["-hide_banner", "-encoders"])
            .output()
            .map_err(|err| format!("failed to inspect ffmpeg encoders: {err}"))?;

        if !output.status.success() {
            return Err(format!(
                "ffmpeg encoder probe failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }

        let encoders = format!(
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        if let Some(requested_encoder) = requested_encoder {
            let encoder = detect_requested_encoder(ffmpeg, &encoders, requested_encoder, codec)?;
            events.on_event(Event::EncoderCandidateEvaluated {
                encoder: encoder.name.clone(),
                advertised: true,
                usable: true,
                detail: None,
            });
            return Ok(Self {
                candidates: vec![encoder],
                explicit: true,
            });
        }

        let advertised_candidates = detect_encoder_candidates_from_listing(&encoders, codec);
        if advertised_candidates.is_empty() {
            return Err(format!(
                "this ffmpeg build advertises no {} encoder",
                codec.label()
            ));
        }

        let mut failures = Vec::new();
        let mut candidates = Vec::new();
        for (name, kind) in codec::encoder_candidates(codec) {
            if !encoder_list_contains(&encoders, name) {
                events.on_event(Event::EncoderCandidateEvaluated {
                    encoder: (*name).to_string(),
                    advertised: false,
                    usable: false,
                    detail: Some("not advertised by ffmpeg".to_string()),
                });
                continue;
            }

            let encoder = Encoder {
                name: (*name).to_string(),
                kind: *kind,
            };
            match check_encoder_runtime(ffmpeg, &encoder) {
                Ok(()) => {
                    events.on_event(Event::EncoderCandidateEvaluated {
                        encoder: encoder.name.clone(),
                        advertised: true,
                        usable: true,
                        detail: None,
                    });
                    candidates.push(encoder);
                }
                Err(err) => {
                    let detail = first_error_line(&err);
                    let message = format!(
                        "Info: encoder {} is advertised but failed runtime check: {}",
                        encoder.name, detail
                    );
                    if terminal {
                        eprintln!("{message}");
                    }
                    events.on_event(Event::EncoderCandidateEvaluated {
                        encoder: encoder.name.clone(),
                        advertised: true,
                        usable: false,
                        detail: Some(detail.clone()),
                    });
                    failures.push(format!("{}: {detail}", encoder.name));
                }
            }
        }

        if candidates.is_empty() {
            return Err(format!(
                "this ffmpeg build advertises {} encoders, but none of them worked: {}",
                codec.label(),
                failures.join("; ")
            ));
        }

        Ok(Self {
            candidates,
            explicit: false,
        })
    }

    // AI-FUNC-SUMMARY: Returns the preferred exact encoder candidate; returns the first GPU-first candidate; side effects: none.
    fn preferred(&self) -> &Encoder {
        self.candidates
            .first()
            .expect("encoder selection always has at least one candidate")
    }
}

// AI-FUNC-SUMMARY: Provides detect requested encoder behavior; returns the declared result; side effects: see implementation.
fn detect_requested_encoder(
    ffmpeg: &str,
    encoders: &str,
    requested: &str,
    codec: TargetCodec,
) -> Result<Encoder, String> {
    let encoder = encoder_from_name(requested, codec)?;
    if !encoder_list_contains(encoders, &encoder.name) {
        return Err(format!(
            "requested encoder {} is not advertised by ffmpeg",
            encoder.name
        ));
    }

    check_encoder_runtime(ffmpeg, &encoder).map_err(|err| {
        format!(
            "requested encoder {} failed runtime encode check: {}",
            encoder.name,
            first_error_line(&err)
        )
    })?;

    Ok(encoder)
}

// AI-FUNC-SUMMARY: Provides encoder from name behavior; returns the declared result; side effects: see implementation.
fn encoder_from_name(name: &str, codec: TargetCodec) -> Result<Encoder, String> {
    let normalized = codec::canonical_encoder_name(name, codec);
    codec::encoder_candidates(codec)
        .iter()
        .find(|(candidate, _kind)| *candidate == normalized)
        .map(|(name, kind)| Encoder {
            name: (*name).to_string(),
            kind: *kind,
        })
        .ok_or_else(|| {
            format!(
                "unsupported encoder: {name}. Supported values: auto, {}",
                supported_encoder_names(codec).join(", ")
            )
        })
}

// AI-FUNC-SUMMARY: Provides supported encoder names behavior; returns the declared result; side effects: see implementation.
fn supported_encoder_names(codec: TargetCodec) -> Vec<&'static str> {
    codec::encoder_candidates(codec)
        .iter()
        .map(|(name, _kind)| *name)
        .collect()
}

#[cfg(test)]
// AI-FUNC-SUMMARY: Provides detect encoder from listing behavior; returns the declared result; side effects: see implementation.
fn detect_encoder_from_listing(encoders: &str) -> Option<Encoder> {
    detect_encoder_candidates_from_listing(encoders, TargetCodec::Av1)
        .into_iter()
        .next()
}

// AI-FUNC-SUMMARY: Filters the advertised encoder listing into GPU-first candidates; returns candidates in deterministic probe order; side effects: none.
fn detect_encoder_candidates_from_listing(encoders: &str, codec: TargetCodec) -> Vec<Encoder> {
    codec::encoder_candidates(codec)
        .iter()
        .filter(|(name, _kind)| encoder_list_contains(encoders, name))
        .map(|(name, kind)| Encoder {
            name: (*name).to_string(),
            kind: *kind,
        })
        .collect()
}

// AI-FUNC-SUMMARY: Validates check encoder runtime conditions; returns success, failure, or test assertion result; side effects: may inspect files or emit test failures.
fn check_encoder_runtime(ffmpeg: &str, encoder: &Encoder) -> Result<(), String> {
    let output = process_command(ffmpeg)
        .args(encoder_runtime_check_args(encoder))
        .output()
        .map_err(|err| format!("failed to runtime-check encoder {}: {err}", encoder.name))?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let detail = if !stderr.is_empty() {
        stderr
    } else if !stdout.is_empty() {
        stdout
    } else {
        format!("ffmpeg exited with status {}", output.status)
    };
    Err(detail)
}

// AI-FUNC-SUMMARY: Provides encoder runtime check args behavior; returns the declared result; side effects: see implementation.
fn encoder_runtime_check_args(encoder: &Encoder) -> Vec<String> {
    let mut args = vec![
        "-hide_banner".to_string(),
        "-loglevel".to_string(),
        "error".to_string(),
        "-nostdin".to_string(),
        "-y".to_string(),
        "-f".to_string(),
        "lavfi".to_string(),
        "-i".to_string(),
        "testsrc2=size=640x360:rate=1".to_string(),
        "-frames:v".to_string(),
        "1".to_string(),
        "-an".to_string(),
        "-c:v".to_string(),
        encoder.name.clone(),
    ];

    args.extend(encoder_runtime_quality_args(encoder));
    args.extend(["-f".to_string(), "null".to_string(), "-".to_string()]);
    args
}

// AI-FUNC-SUMMARY: Provides encoder runtime quality args behavior; returns the declared result; side effects: see implementation.
fn encoder_runtime_quality_args(encoder: &Encoder) -> Vec<String> {
    match encoder.kind {
        EncoderKind::Hardware => vec!["-b:v".to_string(), "600k".to_string()],
        EncoderKind::Vulkan => vec![
            "-crf".to_string(),
            "32".to_string(),
            "-b:v".to_string(),
            "0".to_string(),
        ],
        EncoderKind::SvtAv1 => vec![
            "-crf".to_string(),
            "32".to_string(),
            "-preset".to_string(),
            "10".to_string(),
        ],
        EncoderKind::LibAom => vec![
            "-crf".to_string(),
            "32".to_string(),
            "-b:v".to_string(),
            "0".to_string(),
            "-cpu-used".to_string(),
            "8".to_string(),
        ],
        EncoderKind::Rav1e => vec![
            "-qp".to_string(),
            "80".to_string(),
            "-speed".to_string(),
            "10".to_string(),
        ],
        EncoderKind::X265 => vec![
            "-crf".to_string(),
            "32".to_string(),
            "-preset".to_string(),
            "ultrafast".to_string(),
        ],
        EncoderKind::Vvenc => vec![
            "-qp".to_string(),
            "40".to_string(),
            "-preset".to_string(),
            "faster".to_string(),
        ],
        EncoderKind::MediaCodec => vec![
            "-b:v".to_string(),
            "600k".to_string(),
            "-bitrate_mode".to_string(),
            "1".to_string(),
        ],
    }
}

// AI-FUNC-SUMMARY: Picks the line of ffmpeg output that actually explains a failure; returns that line, or the last line when nothing looks like an error; side effects: none.
fn first_error_line(text: &str) -> String {
    let lines = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !is_ffmpeg_noise(line))
        .collect::<Vec<_>>();

    // ffmpeg reports the actual problem near the end, after pages of stream
    // descriptions and progress. Prefer a line that reads like an error, and
    // fall back to the last thing it said rather than the first.
    lines
        .iter()
        .rev()
        .find(|line| line_looks_like_an_error(line))
        .or_else(|| lines.last())
        .map(|line| (*line).to_string())
        .unwrap_or_else(|| "unknown error".to_string())
}

// AI-FUNC-SUMMARY: Checks whether a line of ffmpeg output is routine chatter rather than a message worth showing; returns true for banners, progress and stream descriptions; side effects: none.
fn is_ffmpeg_noise(line: &str) -> bool {
    const NOISE_PREFIXES: [&str; 12] = [
        "ffmpeg stats and -progress",
        "Input #",
        "Output #",
        "Stream #",
        "Stream mapping",
        "Metadata:",
        "Duration:",
        "encoder ",
        "frame=",
        "video:",
        "Press [q]",
        "configuration:",
    ];

    NOISE_PREFIXES
        .iter()
        .any(|prefix| line.starts_with(prefix))
        // Progress output written to stderr as key=value pairs.
        || line
            .split_once('=')
            .is_some_and(|(key, _)| !key.contains(' ') && !key.contains(':'))
}

// AI-FUNC-SUMMARY: Checks whether a line of ffmpeg output reads like a failure; returns true when it names an error; side effects: none.
fn line_looks_like_an_error(line: &str) -> bool {
    const MARKERS: [&str; 8] = [
        "error",
        "Error",
        "failed",
        "Failed",
        "Invalid",
        "invalid",
        "Unable",
        "not supported",
    ];

    MARKERS.iter().any(|marker| line.contains(marker))
}

// AI-FUNC-SUMMARY: Provides encoder list contains behavior; returns the declared result; side effects: see implementation.
fn encoder_list_contains(encoders: &str, name: &str) -> bool {
    encoders
        .lines()
        .any(|line| line.split_whitespace().any(|part| part == name))
}

// AI-FUNC-SUMMARY: Validates ensure binary conditions; returns success, failure, or test assertion result; side effects: may inspect files or emit test failures.
fn ensure_binary(binary: &str, label: &str) -> Result<(), String> {
    let output = process_command(binary).arg("-version").output();
    match output {
        Ok(output) if output.status.success() => Ok(()),
        Ok(output) => Err(format!(
            "{label} exists but failed to run: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Err(format!(
            "{label} was not found. Install ffmpeg, or pass --{label} PATH."
        )),
        Err(err) => Err(format!("failed to run {label}: {err}")),
    }
}

// AI-FUNC-SUMMARY: Validates ensure tmp dir conditions; returns success, failure, or test assertion result; side effects: may inspect files or emit test failures.
fn ensure_tmp_dir(path: &Path) -> Result<(), String> {
    if !path.exists() {
        fs::create_dir_all(path).map_err(|err| {
            format!(
                "failed to create temporary directory {}: {err}",
                path.to_string_lossy()
            )
        })?;
    }

    if !path.is_dir() {
        return Err(format!(
            "temporary path is not a directory: {}",
            path.to_string_lossy()
        ));
    }

    Ok(())
}

// AI-FUNC-SUMMARY: Provides default runtime binary behavior; returns the declared result; side effects: see implementation.
fn default_runtime_binary(name: &str) -> String {
    bundled_binary_candidates(name)
        .into_iter()
        .find(|candidate| candidate.is_file())
        .map(|candidate| candidate.to_string_lossy().into_owned())
        .unwrap_or_else(|| platform_binary_name(name).to_string())
}

// AI-FUNC-SUMMARY: Provides bundled binary candidates behavior; returns the declared result; side effects: see implementation.
fn bundled_binary_candidates(name: &str) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    let names = bundled_binary_names(name);

    if let Ok(exe) = env::current_exe()
        && let Some(dir) = exe.parent()
    {
        push_binary_candidates(&mut candidates, dir, &names);
    }

    if let Ok(cwd) = env::current_dir() {
        push_binary_candidates(&mut candidates, &cwd, &names);
    }

    candidates
}

// AI-FUNC-SUMMARY: Provides push binary candidates behavior; returns the declared result; side effects: see implementation.
fn push_binary_candidates(candidates: &mut Vec<PathBuf>, dir: &Path, names: &[String]) {
    for name in names {
        candidates.push(dir.join(name));
        candidates.push(dir.join("bin").join(name));
    }
}

// AI-FUNC-SUMMARY: Provides bundled binary names behavior; returns the declared result; side effects: see implementation.
fn bundled_binary_names(name: &str) -> Vec<String> {
    let platform_name = platform_binary_name(name);
    if platform_name == name {
        vec![name.to_string()]
    } else {
        vec![platform_name.to_string(), name.to_string()]
    }
}

// AI-FUNC-SUMMARY: Provides platform binary name behavior; returns the declared result; side effects: see implementation.
fn platform_binary_name(name: &str) -> &str {
    if cfg!(windows) {
        match name {
            "ffmpeg" => "ffmpeg.exe",
            "ffprobe" => "ffprobe.exe",
            _ => name,
        }
    } else {
        name
    }
}

// AI-FUNC-SUMMARY: Discovers or probes discover files data; returns collected metadata; side effects: may read filesystem or subprocess output.
fn discover_files(root: &Path) -> io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut dirs = VecDeque::from([root.to_path_buf()]);

    while let Some(dir) = dirs.pop_front() {
        visit_dir(&dir, &mut files, &mut dirs)?;
    }

    Ok(files)
}

// AI-FUNC-SUMMARY: Checks is candidate video path predicate; returns a boolean; side effects: none.
fn is_candidate_video_path(path: &Path) -> bool {
    path.extension()
        .and_then(OsStr::to_str)
        .is_some_and(is_candidate_video_extension)
}

// AI-FUNC-SUMMARY: Checks is candidate video extension predicate; returns a boolean; side effects: none.
fn is_candidate_video_extension(extension: &str) -> bool {
    matches!(
        extension.to_ascii_lowercase().as_str(),
        "3g2"
            | "3gp"
            | "asf"
            | "avi"
            | "divx"
            | "dv"
            | "f4v"
            | "flv"
            | "m2ts"
            | "m4v"
            | "mkv"
            | "mov"
            | "mp4"
            | "mpeg"
            | "mpg"
            | "mts"
            | "mxf"
            | "ogm"
            | "ogv"
            | "rm"
            | "rmvb"
            | "ts"
            | "vob"
            | "webm"
            | "wmv"
    )
}

// AI-FUNC-SUMMARY: Discovers or probes visit dir data; returns collected metadata; side effects: may read filesystem or subprocess output.
fn visit_dir(dir: &Path, files: &mut Vec<PathBuf>, dirs: &mut VecDeque<PathBuf>) -> io::Result<()> {
    let mut entries = fs::read_dir(dir)?.collect::<Result<Vec<_>, io::Error>>()?;
    entries.sort_by_key(|entry| entry.file_name().to_string_lossy().into_owned());

    let mut child_dirs = Vec::new();

    for entry in entries {
        let file_type = entry.file_type()?;
        let path = entry.path();
        if file_type.is_dir() {
            child_dirs.push(path);
        } else if file_type.is_file() {
            files.push(path);
        }
    }

    dirs.extend(child_dirs);

    Ok(())
}

// AI-FUNC-SUMMARY: Probes the primary video stream; returns metadata, none for unreadable media, or a tool/process error; side effects: runs ffprobe.
fn probe_video(ffprobe: &str, path: &Path) -> Result<Option<VideoInfo>, String> {
    let output = process_command(ffprobe)
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=codec_name,profile,width,height,sample_aspect_ratio,display_aspect_ratio,field_order,avg_frame_rate,r_frame_rate,pix_fmt,bits_per_raw_sample,bit_rate,color_range,color_space,color_transfer,color_primaries,chroma_location:stream_tags=encoder:format=duration,bit_rate",
            "-of",
            "default=noprint_wrappers=1:nokey=0",
        ])
        .arg(path)
        .output()
        .map_err(|err| {
            format!(
                "failed to run ffprobe for {}: {err}",
                path.to_string_lossy()
            )
        })?;

    if !output.status.success() {
        if !ffprobe_failure_is_infrastructure(output.status, &output.stderr) {
            return Ok(None);
        }
        return Err(probe_failure_message(
            "ffprobe failed for",
            path,
            output.status,
            &output.stderr,
        ));
    }

    let text = String::from_utf8_lossy(&output.stdout);
    parse_video_probe(&text)
}

// AI-FUNC-SUMMARY: Parses video probe input; returns parsed values or errors; side effects: none.
fn parse_video_probe(text: &str) -> Result<Option<VideoInfo>, String> {
    let mut codec_name = None;
    let mut profile = None;
    let mut width = None;
    let mut height = None;
    let mut sample_aspect_ratio = None;
    let mut display_aspect_ratio = None;
    let mut field_order = None;
    let mut fps = None;
    let mut nominal_fps = None;
    let mut avg_frame_rate = None;
    let mut nominal_frame_rate = None;
    let mut duration_seconds = None;
    let mut bit_rate = None;
    let mut pix_fmt = None;
    let mut bits_per_raw_sample = None;
    let mut color_range = None;
    let mut color_space = None;
    let mut color_transfer = None;
    let mut color_primaries = None;
    let mut chroma_location = None;
    let mut encoder_tag = None;

    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim();
        if value.is_empty() || value.eq_ignore_ascii_case("N/A") {
            continue;
        }

        match key.trim() {
            "codec_name" => codec_name = Some(value.to_string()),
            "profile" => profile = Some(value.to_string()),
            "width" => width = value.parse::<u32>().ok(),
            "height" => height = value.parse::<u32>().ok(),
            "sample_aspect_ratio" => sample_aspect_ratio = known_ratio_value(value),
            "display_aspect_ratio" => display_aspect_ratio = known_ratio_value(value),
            "field_order" => field_order = known_color_value(value),
            "avg_frame_rate" => {
                if let Some((raw, parsed)) = parse_frame_rate(value) {
                    avg_frame_rate = Some(raw);
                    fps = Some(parsed);
                }
            }
            "r_frame_rate" => {
                if let Some((raw, parsed)) = parse_frame_rate(value) {
                    nominal_frame_rate = Some(raw);
                    nominal_fps = Some(parsed);
                }
            }
            "duration" => duration_seconds = value.parse::<f64>().ok(),
            "bit_rate" if bit_rate.is_none() => bit_rate = value.parse::<u64>().ok(),
            "pix_fmt" => pix_fmt = Some(value.to_string()),
            "bits_per_raw_sample" => bits_per_raw_sample = value.parse::<u8>().ok(),
            "color_range" => color_range = known_color_value(value),
            "color_space" => color_space = known_color_value(value),
            "color_transfer" => color_transfer = known_color_value(value),
            "color_primaries" => color_primaries = known_color_value(value),
            "chroma_location" => chroma_location = known_color_value(value),
            "TAG:encoder" => encoder_tag = Some(value.to_string()),
            _ => {}
        }
    }

    let Some(codec_name) = codec_name else {
        return Ok(None);
    };

    Ok(Some(VideoInfo {
        codec_name,
        profile,
        width: width.unwrap_or(0),
        height: height.unwrap_or(0),
        sample_aspect_ratio,
        display_aspect_ratio,
        field_order,
        fps: fps.unwrap_or(0.0),
        nominal_fps: nominal_fps.unwrap_or(0.0),
        avg_frame_rate,
        nominal_frame_rate,
        duration_seconds: duration_seconds.unwrap_or(0.0),
        bit_rate,
        pix_fmt,
        bits_per_raw_sample,
        color_range,
        color_space,
        color_transfer,
        color_primaries,
        chroma_location,
        encoder_tag,
    }))
}

// AI-FUNC-SUMMARY: Provides known color value behavior; returns the declared result; side effects: see implementation.
fn known_color_value(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty()
        || value.eq_ignore_ascii_case("N/A")
        || value.eq_ignore_ascii_case("unknown")
        || value.eq_ignore_ascii_case("unspecified")
        || value.eq_ignore_ascii_case("reserved")
    {
        None
    } else {
        Some(value.to_string())
    }
}

// AI-FUNC-SUMMARY: Provides known ratio value behavior; returns the declared result; side effects: see implementation.
fn known_ratio_value(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() || value.eq_ignore_ascii_case("N/A") || value == "0:1" || value == "0/1" {
        None
    } else {
        Some(value.to_string())
    }
}

// AI-FUNC-SUMMARY: Probes stream layout and chapter-carrier metadata; returns indexed streams, an empty list for unreadable media, or a tool/process error; side effects: runs ffprobe.
fn probe_streams(ffprobe: &str, path: &Path) -> Result<Vec<StreamInfo>, String> {
    let output = process_command(ffprobe)
        .args([
            "-v",
            "error",
            "-show_entries",
            "stream=index,codec_name,codec_type,codec_tag_string,id:stream_tags=handler_name",
            "-of",
            "compact=p=0:nk=0",
        ])
        .arg(path)
        .output()
        .map_err(|err| {
            format!(
                "failed to inspect streams for {}: {err}",
                path.to_string_lossy()
            )
        })?;

    if !output.status.success() {
        return Err(probe_failure_message(
            "ffprobe failed to inspect streams for",
            path,
            output.status,
            &output.stderr,
        ));
    }

    Ok(parse_stream_probe(&String::from_utf8_lossy(&output.stdout)))
}

// AI-FUNC-SUMMARY: Distinguishes ffprobe tool/process failures from ordinary unreadable media diagnostics; returns true only for abnormal exits or known runtime failures; side effects: none.
fn ffprobe_failure_is_infrastructure(status: ExitStatus, stderr: &[u8]) -> bool {
    let detail = String::from_utf8_lossy(stderr).to_ascii_lowercase();
    if detail.trim().is_empty() || status.code().is_none_or(|code| code < 0) {
        return true;
    }

    [
        "unrecognized option",
        "option not found",
        "error while loading shared libraries",
        "failed to load",
        "could not load",
        "bad cpu type",
        "exec format",
        "access is denied",
        "access denied",
        "not a valid win32 application",
        "entry point",
    ]
    .iter()
    .any(|marker| detail.contains(marker))
}

// AI-FUNC-SUMMARY: Formats a failed ffprobe exit with path and concise diagnostics; returns a terminal process error; side effects: none.
fn probe_failure_message(action: &str, path: &Path, status: ExitStatus, stderr: &[u8]) -> String {
    let detail = String::from_utf8_lossy(stderr);
    let detail = detail.lines().find(|line| !line.trim().is_empty());
    match detail {
        Some(detail) => format!(
            "{action} {} with status {status}: {}",
            path.to_string_lossy(),
            detail.trim()
        ),
        None => format!("{action} {} with status {status}", path.to_string_lossy()),
    }
}

// AI-FUNC-SUMMARY: Parses compact ffprobe stream records, retaining actual indexes and chapter-carrier metadata; returns unique indexed streams in probe order; side effects: none.
fn parse_stream_probe(text: &str) -> Vec<StreamInfo> {
    let mut streams = Vec::new();
    let mut seen_indexes = Vec::new();
    for line in text.lines() {
        let mut index = None;
        let mut codec_type = None;
        let mut codec_name = None;
        let mut codec_tag_string = None;
        let mut track_id = None;
        let mut handler_name = None;
        for (key, value) in parse_compact_fields(line) {
            match key.as_str() {
                "index" => index = value.parse::<usize>().ok(),
                "codec_type" => codec_type = Some(value),
                "codec_name" => codec_name = Some(value),
                "codec_tag_string" => codec_tag_string = known_probe_value(&value),
                "id" => track_id = parse_track_id(&value),
                "tag:handler_name" | "TAG:handler_name" => handler_name = known_probe_value(&value),
                _ => {}
            }
        }

        if let (Some(index), Some(codec_type)) = (index, codec_type) {
            if seen_indexes.contains(&index) {
                continue;
            }
            seen_indexes.push(index);

            streams.push(StreamInfo {
                index,
                codec_type,
                codec_name: codec_name.unwrap_or_else(|| "unknown".to_string()),
                codec_tag_string,
                track_id,
                handler_name,
            });
        }
    }
    streams
}

// AI-FUNC-SUMMARY: Converts a non-empty ffprobe field into optional metadata; returns none for empty, N/A, or unknown values; side effects: none.
fn known_probe_value(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()
        && !value.eq_ignore_ascii_case("N/A")
        && !value.eq_ignore_ascii_case("unknown"))
    .then(|| value.to_string())
}

// AI-FUNC-SUMMARY: Parses an ffprobe track ID in decimal or 0x-prefixed hexadecimal form; returns a positive 32-bit ID or none; side effects: none.
fn parse_track_id(value: &str) -> Option<u32> {
    let value = value.trim();
    let parsed = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
        .map_or_else(|| value.parse::<u32>(), |hex| u32::from_str_radix(hex, 16))
        .ok()?;
    (parsed != 0).then_some(parsed)
}

// AI-FUNC-SUMMARY: Probes source chapter start/end/title metadata; returns parsed chapters or a tool/process error; side effects: runs ffprobe.
fn probe_chapters(ffprobe: &str, path: &Path) -> Result<Vec<ChapterInfo>, String> {
    let output = process_command(ffprobe)
        .args([
            "-v",
            "error",
            "-show_entries",
            "chapter=start_time,end_time:chapter_tags=title",
            "-of",
            "compact=p=0:nk=0",
        ])
        .arg(path)
        .output()
        .map_err(|err| {
            format!(
                "failed to inspect chapters for {}: {err}",
                path.to_string_lossy()
            )
        })?;
    if !output.status.success() {
        return Err(probe_failure_message(
            "ffprobe failed to inspect chapters for",
            path,
            output.status,
            &output.stderr,
        ));
    }
    parse_chapter_probe(&String::from_utf8_lossy(&output.stdout)).map_err(|err| {
        format!(
            "invalid chapter metadata for {}: {err}",
            path.to_string_lossy()
        )
    })
}

// AI-FUNC-SUMMARY: Parses compact ffprobe chapter records; returns complete finite timestamp/title records or a structural error; side effects: none.
fn parse_chapter_probe(text: &str) -> Result<Vec<ChapterInfo>, String> {
    let mut chapters = Vec::new();
    for (line_index, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let mut start = None;
        let mut end = None;
        let mut title = String::new();
        for (key, value) in parse_compact_fields(line) {
            match key.as_str() {
                "start_time" => start = value.parse::<f64>().ok().filter(|v| v.is_finite()),
                "end_time" => end = value.parse::<f64>().ok().filter(|v| v.is_finite()),
                "tag:title" | "TAG:title" => title = value,
                _ => {}
            }
        }
        let start = start
            .ok_or_else(|| format!("chapter record {} has no finite start_time", line_index + 1))?;
        let end =
            end.ok_or_else(|| format!("chapter record {} has no finite end_time", line_index + 1))?;
        if end < start {
            return Err(format!(
                "chapter record {} ends before it starts",
                line_index + 1
            ));
        }
        chapters.push(ChapterInfo {
            start_seconds: start,
            end_seconds: end,
            title,
        });
    }
    Ok(chapters)
}

// AI-FUNC-SUMMARY: Splits one FFprobe compact record at unescaped separators and decodes C-style field values; returns key/value fields without losing escaped title or handler characters; side effects: none.
fn parse_compact_fields(line: &str) -> Vec<(String, String)> {
    split_compact_unescaped(line, '|')
        .into_iter()
        .filter_map(|field| {
            let parts = split_compact_unescaped(&field, '=');
            let (key, values) = parts.split_first()?;
            (!values.is_empty()).then(|| {
                (
                    decode_compact_escapes(key),
                    decode_compact_escapes(&values.join("=")),
                )
            })
        })
        .collect()
}

// AI-FUNC-SUMMARY: Splits FFprobe compact text on unescaped delimiters while retaining escape sequences for later decoding; returns encoded segments; side effects: none.
fn split_compact_unescaped(text: &str, separator: char) -> Vec<String> {
    let mut fields = vec![String::new()];
    let mut chars = text.chars();
    while let Some(ch) = chars.next() {
        if ch == separator {
            fields.push(String::new());
            continue;
        }
        fields.last_mut().expect("field exists").push(ch);
        if ch == '\\'
            && let Some(escaped) = chars.next()
        {
            fields.last_mut().expect("field exists").push(escaped);
        }
    }
    fields
}

// AI-FUNC-SUMMARY: Decodes common FFprobe compact C escape sequences after structural splitting; returns the represented field text; side effects: none.
fn decode_compact_escapes(text: &str) -> String {
    let mut decoded = String::new();
    let mut chars = text.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            decoded.push(ch);
            continue;
        }
        let Some(escaped) = chars.next() else {
            decoded.push('\\');
            break;
        };
        decoded.push(match escaped {
            'n' => '\n',
            'r' => '\r',
            't' => '\t',
            'f' => '\u{000c}',
            'b' => '\u{0008}',
            other => other,
        });
    }
    decoded
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TranscodePlanKind {
    Exact,
    Adapted {
        output_pix_fmt: &'static str,
        description: &'static str,
    },
}

/// Colour details the output is allowed to lose for one attempt.
///
/// Nothing is ever exempt under strict preservation. Under flexible
/// preservation a field is exempted only when the chosen codec genuinely cannot
/// record it, and every exemption is reported to the user as a change.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct MetadataExemptions {
    chroma_location: bool,
    color_primaries: bool,
    color_transfer: bool,
    color_space: bool,
    color_range: bool,
}

impl MetadataExemptions {}

#[derive(Debug, Clone, Copy)]
struct TranscodePlan<'a> {
    encoder: &'a Encoder,
    kind: TranscodePlanKind,
    /// Quality setting on this encoder's own scale.
    quality: EncoderQuality,
    /// Colour details this attempt may drop.
    exempt: MetadataExemptions,
}

#[derive(Debug)]
struct TranscodeAttemptError {
    message: String,
    retryable: bool,
}

impl TranscodeAttemptError {
    // AI-FUNC-SUMMARY: Creates a retryable encoder-attempt failure; returns typed failure; side effects: none.
    fn retryable(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            retryable: true,
        }
    }

    // AI-FUNC-SUMMARY: Creates a terminal encoder-attempt failure; returns typed failure; side effects: none.
    fn terminal(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            retryable: false,
        }
    }
}

impl TranscodePlanKind {
    // AI-FUNC-SUMMARY: Returns the stable event label for a transcode plan; returns exact or adapted; side effects: none.
    fn label(&self) -> &'static str {
        match self {
            TranscodePlanKind::Exact => "exact",
            TranscodePlanKind::Adapted { .. } => "adapted",
        }
    }

    // AI-FUNC-SUMMARY: Returns the planned output pixel format when adaptation is requested; returns none for exact plans; side effects: none.
    fn output_pix_fmt(&self) -> Option<&'static str> {
        match self {
            TranscodePlanKind::Exact => None,
            TranscodePlanKind::Adapted { output_pix_fmt, .. } => Some(output_pix_fmt),
        }
    }

    // AI-FUNC-SUMMARY: Returns a user-facing plan description; returns exact-preservation or adaptation detail; side effects: none.
    fn description(&self) -> &'static str {
        match self {
            TranscodePlanKind::Exact => "preserve source video properties",
            TranscodePlanKind::Adapted { description, .. } => description,
        }
    }
}

// AI-FUNC-SUMMARY:
// Purpose: Builds the ordered list of encode attempts for one file.
// Inputs: Runtime-usable encoders, the encoder preference, preservation policy, target codec, and source video properties.
// Returns: Exact plans first, then any plan that changes something recoverable, so the tool always tries the faithful option before the compromise.
// Side effects: None.
// Notes: An explicit encoder choice never switches to a different encoder, though it may retry with an adapted pixel format.
fn transcode_plans<'a>(
    selection: &'a EncoderSelection,
    preference: EncoderPreference,
    preservation: Preservation,
    codec: TargetCodec,
    video: &VideoInfo,
    item: &WorkItem,
) -> Vec<TranscodePlan<'a>> {
    let exemptions = if preservation.allows_deviations() {
        metadata_exemptions_for(codec, video)
    } else {
        MetadataExemptions::default()
    };
    let adaptation = closest_hardware_pixel_format(video).map(|(output_pix_fmt, description)| {
        TranscodePlanKind::Adapted {
            output_pix_fmt,
            description,
        }
    });
    // A codec that accepts only one pixel format has to convert every source,
    // so its adaptation is part of the plan rather than a fallback.
    let forced_format = codec_forced_pixel_format(codec, video);

    let plan = |encoder: &'a Encoder, kind: TranscodePlanKind| TranscodePlan {
        encoder,
        kind,
        quality: default_encoder_quality(item, encoder, codec),
        exempt: exemptions,
    };

    if selection.explicit {
        let encoder = selection.preferred();
        let first_kind = forced_format.unwrap_or(TranscodePlanKind::Exact);
        let mut plans = vec![plan(encoder, first_kind)];
        if forced_format.is_none()
            && encoder.is_gpu()
            && matches!(preference, EncoderPreference::Gpu)
            && let Some(adapted) = adaptation
        {
            plans.push(plan(encoder, adapted));
        }
        return plans;
    }

    let gpu_encoders = selection
        .candidates
        .iter()
        .filter(|encoder| encoder.is_gpu())
        .collect::<Vec<_>>();
    let cpu_encoders = selection
        .candidates
        .iter()
        .filter(|encoder| !encoder.is_gpu())
        .collect::<Vec<_>>();

    // Processor-only runs never touch a graphics card, even if one would work.
    let gpu_encoders = if preference == EncoderPreference::Cpu {
        Vec::new()
    } else {
        gpu_encoders
    };

    if let Some(forced) = forced_format {
        // Only the encoders that can produce this codec are in the selection,
        // and they all need the same conversion.
        return gpu_encoders
            .iter()
            .chain(cpu_encoders.iter())
            .map(|encoder| plan(encoder, forced))
            .collect();
    }

    let exact_gpu = gpu_encoders
        .iter()
        .map(|encoder| plan(encoder, TranscodePlanKind::Exact))
        .collect::<Vec<_>>();
    let exact_cpu = cpu_encoders
        .iter()
        .map(|encoder| plan(encoder, TranscodePlanKind::Exact))
        .collect::<Vec<_>>();
    let adapted_gpu = adaptation
        .map(|kind| {
            gpu_encoders
                .iter()
                .map(|encoder| plan(encoder, kind))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let mut plans = exact_gpu;
    match preference {
        // Put an adapted graphics-card attempt ahead of the processor, because
        // the user asked for speed over an exact pixel format.
        EncoderPreference::Gpu => {
            plans.extend(adapted_gpu);
            plans.extend(exact_cpu);
        }
        EncoderPreference::Auto | EncoderPreference::Cpu => {
            plans.extend(exact_cpu);
            // Only a flexible run may fall back to changing the pixel format.
            if preservation.allows_deviations() {
                plans.extend(adapted_gpu);
            }
        }
    }
    plans
}

// AI-FUNC-SUMMARY: Reports the conversion a codec forces on every source it cannot take directly; returns an adapted plan kind or none; side effects: none.
fn codec_forced_pixel_format(codec: TargetCodec, video: &VideoInfo) -> Option<TranscodePlanKind> {
    if codec::codec_accepts_source(codec, video) {
        return None;
    }

    match codec {
        TargetCodec::Vvc => Some(TranscodePlanKind::Adapted {
            output_pix_fmt: "yuv420p10le",
            description: "convert to 10-bit 4:2:0, the only format H.266 encoding accepts",
        }),
        TargetCodec::Av1 | TargetCodec::Hevc => None,
    }
}

// AI-FUNC-SUMMARY: Works out which colour details the chosen codec cannot record for this source; returns the exemptions a flexible run may accept; side effects: none.
fn metadata_exemptions_for(codec: TargetCodec, video: &VideoInfo) -> MetadataExemptions {
    let Some(filter) = codec::metadata_filter(codec) else {
        // A codec with no metadata filter cannot state any of it.
        return MetadataExemptions {
            chroma_location: true,
            color_primaries: true,
            color_transfer: true,
            color_space: true,
            color_range: true,
        };
    };

    let chroma_location = filter.chroma.is_none()
        || video
            .chroma_location
            .as_deref()
            .is_some_and(|value| codec::chroma_location_value(codec, value).is_none());
    let color_range = video
        .color_range
        .as_deref()
        .is_some_and(|value| codec::color_range_value(codec, value).is_none());
    let color_primaries = video
        .color_primaries
        .as_deref()
        .is_some_and(|value| av1_color_primaries_value(value).is_none());
    let color_transfer = video
        .color_transfer
        .as_deref()
        .is_some_and(|value| av1_transfer_characteristics_value(value).is_none());
    let color_space = video
        .color_space
        .as_deref()
        .is_some_and(|value| av1_matrix_coefficients_value(value).is_none());

    MetadataExemptions {
        chroma_location,
        color_primaries,
        color_transfer,
        color_space,
        color_range,
    }
}

// AI-FUNC-SUMMARY: Describes each accepted colour exemption for the user; returns one sentence per change; side effects: none.
fn exemption_descriptions(
    exempt: &MetadataExemptions,
    codec: TargetCodec,
) -> Vec<(String, String)> {
    let codec_name = codec.label();
    let mut changes = Vec::new();
    if exempt.chroma_location {
        changes.push((
            "chroma_location".to_string(),
            format!("{codec_name} cannot record where colour samples sit, so that note is lost. Picture data is unchanged."),
        ));
    }
    if exempt.color_primaries {
        changes.push((
            "color_primaries".to_string(),
            format!("{codec_name} cannot record this colour space name, so players fall back to the usual one."),
        ));
    }
    if exempt.color_transfer {
        changes.push((
            "color_transfer".to_string(),
            format!("{codec_name} cannot record this brightness curve, so players fall back to the usual one."),
        ));
    }
    if exempt.color_space {
        changes.push((
            "color_space".to_string(),
            format!("{codec_name} cannot record this colour matrix, so players fall back to the usual one."),
        ));
    }
    if exempt.color_range {
        changes.push((
            "color_range".to_string(),
            format!("{codec_name} cannot record the black-and-white level range, so players assume the usual one."),
        ));
    }
    changes
}

// AI-FUNC-SUMMARY: Chooses the closest safe initial hardware pixel-format target; returns target plus change description or none when adaptation is unsafe/unnecessary; side effects: none.
fn closest_hardware_pixel_format(video: &VideoInfo) -> Option<(&'static str, &'static str)> {
    let source = normalized_pix_fmt(video.pix_fmt.as_deref()?);
    if !source.starts_with("yuv") || pixel_format_has_alpha(source) {
        return None;
    }

    let target = if video.bits_per_raw_sample.unwrap_or(8) > 8
        || pixel_format_bit_depth(source).unwrap_or(8) > 8
        || matches!(
            video.color_transfer.as_deref(),
            Some("smpte2084" | "arib-std-b67")
        ) {
        "yuv420p10le"
    } else {
        "yuv420p"
    };
    if source == target {
        return None;
    }

    let description = if target == "yuv420p10le" {
        "adapt chroma/bit depth to 4:2:0 10-bit for GPU encoding"
    } else {
        "adapt chroma to 4:2:0 8-bit for GPU encoding"
    };
    Some((target, description))
}

// AI-FUNC-SUMMARY: Extracts an explicit YUV planar bit depth such as 9, 10, 12, 14, or 16 from an FFmpeg pixel-format name; returns none for implicit 8-bit names; side effects: none.
fn pixel_format_bit_depth(pix_fmt: &str) -> Option<u8> {
    let (_, suffix) = pix_fmt.rsplit_once('p')?;
    let digits = suffix
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>();
    (!digits.is_empty()).then(|| digits.parse().ok()).flatten()
}

// AI-FUNC-SUMMARY: Detects alpha-bearing pixel formats that must not be adapted automatically; returns true for known YUV/RGB alpha formats; side effects: none.
fn pixel_format_has_alpha(pix_fmt: &str) -> bool {
    pix_fmt.starts_with("yuva")
        || pix_fmt.starts_with("gbrap")
        || matches!(
            pix_fmt,
            "rgba" | "bgra" | "argb" | "abgr" | "ya8" | "ya16le" | "ya16be"
        )
}

// AI-FUNC-SUMMARY:
// Purpose: Produces and commits one validated output while allowing safe exact/adapted encoder fallback.
// Inputs: Run options, one work item, the runtime-usable encoder selection, and progress UI.
// Returns: Conversion sizes or a terminal/aggregated user-facing error.
// Side effects: May inspect cached outputs, run encoder attempts, remove failed temps, and commit one validated output.
// Notes: Cache lookup and final commit occur outside the retry loop; explicit encoder selections never fall back.
fn transcode_item(
    options: &RunOptions,
    item: &WorkItem,
    selection: &EncoderSelection,
    ui: &mut ProgressUi,
) -> Result<ConversionResult, String> {
    let metadata_filter = metadata_bsf_arg(
        options.target_codec,
        &item.video,
        &MetadataExemptions::default(),
    );
    if metadata_filter.is_some() {
        ensure_metadata_bsf(&options.ffmpeg, options.target_codec)?;
    }

    if let Some(cache_path) =
        prepare_cached_temp_output(options, item, metadata_filter.as_deref(), ui)?
    {
        let mut cached_item = item.clone();
        cached_item.temp_path = cache_path;
        // A reused temp still has to earn its place, so it is measured and can
        // still end up in review rather than replacing the original outright.
        let outcome = assess_quality(options, &cached_item, &[], ui);
        return finish_conversion(options, &cached_item, outcome, ui);
    }

    let plans = transcode_plans(
        selection,
        options.encoder_preference,
        options.preservation,
        options.target_codec,
        &item.video,
        item,
    );
    let total_attempts = plans.len();
    let mut failures = Vec::new();
    let mut fallback_reason = None;

    for (index, plan) in plans.iter().enumerate() {
        let mut plan = *plan;
        // Tune the quality setting on samples before committing to a full
        // encode. The result replaces the estimate for this attempt only.
        if options.quality_mode == QualityMode::Search && ui.quality_available {
            plan.quality = search_quality_for(options, item, &plan, ui);
        }
        let plan = &plan;

        ui.start_attempt(
            item,
            plan,
            index + 1,
            total_attempts,
            fallback_reason.as_deref(),
        );
        match execute_transcode_attempt(options, item, plan, metadata_filter.as_deref(), ui) {
            Ok(()) => {
                let deviations = plan_deviations(options, plan, item);
                let outcome = assess_quality(options, item, &deviations, ui);
                return finish_conversion(options, item, outcome, ui);
            }
            Err(err) => {
                if let Err(cleanup_err) = remove_failed_attempt_output(&item.temp_path) {
                    return Err(format!("{}; {}", err.message, cleanup_err.message));
                }

                failures.push(format!("{}: {}", plan.encoder.name, err.message));
                let has_next = index + 1 < total_attempts;
                let explicit_stays_on_encoder = selection.explicit
                    && has_next
                    && plans[index + 1].encoder.name == plan.encoder.name;
                if (selection.explicit && !explicit_stays_on_encoder) || !err.retryable || !has_next
                {
                    return Err(failures.join("; "));
                }

                ui.log_warning(format!(
                    "Warning: encoder {} could not complete this plan; trying the next candidate: {}",
                    plan.encoder.name, err.message
                ));
                fallback_reason = Some(err.message);
            }
        }
    }

    Err("no usable encoder plan is available".to_string())
}

/// What the tool decided about one finished conversion.
struct ConversionOutcome {
    /// Measured quality in hundredths, when it could be measured.
    score: Option<u32>,
    /// Recoverable differences from the source, as (kind, sentence) pairs.
    deviations: Vec<(String, String)>,
    /// True when the result must be kept beside its original for the user.
    needs_review: bool,
}

// AI-FUNC-SUMMARY:
// Purpose: Tunes the quality setting for one file by encoding and measuring short samples.
// Inputs: Run options, the work item, the plan whose encoder is being tuned, and the progress UI.
// Returns: The tuned setting, or the starting estimate when sampling is not possible.
// Side effects: Runs several short ffmpeg encodes and comparisons, and writes then deletes temporary files.
// Notes: A search that cannot run is never fatal; the estimate is a working setting on its own.
fn search_quality_for(
    options: &RunOptions,
    item: &WorkItem,
    plan: &TranscodePlan<'_>,
    ui: &mut ProgressUi,
) -> EncoderQuality {
    let fallback = plan.quality;
    let Some(sample_plan) = search::sample_plan(item.video.duration_seconds) else {
        // Short files are not worth sampling: the trials would cost more than
        // simply encoding the file once.
        return fallback;
    };

    let work_dir = options
        .tmp_dir
        .as_deref()
        .map(Path::to_path_buf)
        .or_else(|| item.input_path.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."));
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();

    ui.render_stage("Choosing quality settings");
    let samples = match search::extract_samples(
        &options.ffmpeg,
        &item.input_path,
        &item.video,
        sample_plan,
        &work_dir,
        stamp,
    ) {
        Ok(samples) => samples,
        Err(err) => {
            ui.log_warning(format!(
                "Warning: could not tune quality for this file: {err}"
            ));
            return fallback;
        }
    };

    let kind = plan.encoder.kind;
    let ladder = matches!(codec::quality_style(kind), codec::QualityStyle::Bitrate).then(|| {
        search::hardware_bitrate_ladder(estimate_target_bitrate(item, options.target_codec).0)
    });
    let range = codec::quality_range(kind);
    let mut tried: Vec<(f64, u32)> = Vec::new();

    for iteration in 0..search::MAX_ITERATIONS {
        // Hardware encoders walk a fixed ladder of bitrates; everything else
        // interpolates on its own quality scale.
        let candidate = match &ladder {
            Some(ladder) => ladder.get(iteration).copied(),
            None => search::next_quality(&tried, options.quality_target, range),
        };
        let Some(candidate) = candidate else {
            break;
        };

        let quality = search::quality_from_value(kind, candidate);
        let label = quality.label(kind);
        match search::measure_trial(
            &options.ffmpeg,
            samples.paths(),
            &plan.encoder.name,
            kind,
            candidate,
            options.target_codec,
            &item.video,
        ) {
            Ok(score) => {
                ui.emit_quality_search(iteration + 1, &plan.encoder.name, &label, Some(score));
                tried.push((candidate, score));
                // A hardware ladder can stop as soon as one rung clears the
                // target, because the rungs only go up from there.
                if ladder.is_some() && score >= options.quality_target {
                    break;
                }
            }
            Err(err) => {
                ui.log_warning(format!("Warning: a trial encode failed: {err}"));
                ui.emit_quality_search(iteration + 1, &plan.encoder.name, &label, None);
                break;
            }
        }
    }

    match search::best_quality(&tried, options.quality_target) {
        Some((value, _score, reached)) => {
            if !reached {
                ui.log_warning(
                    "Warning: this file could not reach the quality target; using the best setting found."
                        .to_string(),
                );
            }
            search::quality_from_value(kind, value)
        }
        None => fallback,
    }
}
// AI-FUNC-SUMMARY: Lists the recoverable differences one encode plan introduces; returns a kind and sentence for each; side effects: none.
fn plan_deviations(
    options: &RunOptions,
    plan: &TranscodePlan<'_>,
    item: &WorkItem,
) -> Vec<(String, String)> {
    let mut deviations = exemption_descriptions(&plan.exempt, options.target_codec);

    if let Some(target) = plan.kind.output_pix_fmt() {
        let source = item.video.pix_fmt.as_deref().unwrap_or("the source format");
        deviations.push((
            "pixel_format".to_string(),
            format!(
                "Colour detail was converted from {source} to {target}. Most viewers will not see a difference."
            ),
        ));
    }

    deviations
}

// AI-FUNC-SUMMARY: Measures a finished conversion and decides whether it needs a human decision; returns the outcome; side effects: may run a long ffmpeg comparison and emit quality events.
fn assess_quality(
    options: &RunOptions,
    item: &WorkItem,
    deviations: &[(String, String)],
    ui: &mut ProgressUi,
) -> ConversionOutcome {
    let mut deviations = deviations.to_vec();
    // A conversion that changed something is always measured, because the user
    // needs a number to judge it by.
    let must_measure = !deviations.is_empty();
    let wants_measure = options.quality_check != QualityCheck::Off;

    if !must_measure && !wants_measure {
        return ConversionOutcome {
            score: None,
            deviations,
            needs_review: false,
        };
    }

    if !ui.quality_available {
        // The reason was already reported once at the start of the run.
        return ConversionOutcome {
            score: None,
            deviations,
            needs_review: must_measure,
        };
    }

    ui.render_stage("Measuring quality");
    let vmaf_options = VmafOptions {
        subsample: options.quality_check.subsample(),
        threads: options.quality_threads,
    };
    let measured = vmaf::measure_vmaf(
        &options.ffmpeg,
        &item.temp_path,
        &item.input_path,
        &item.video,
        vmaf_options,
    );

    match measured {
        Ok(score) => {
            let caveat = vmaf::accuracy_caveat(&item.video).map(str::to_string);
            let passed = score.hundredths >= options.review_threshold();
            ui.emit_quality_measured(
                score.hundredths,
                score.subsample,
                options.quality_target,
                passed,
                caveat.clone(),
            );
            if !passed {
                deviations.push((
                    "quality".to_string(),
                    format!(
                        "The converted file scored {} out of 100, below the {} you asked for.",
                        format_quality(score.hundredths),
                        format_quality(options.quality_target)
                    ),
                ));
            }
            ConversionOutcome {
                score: Some(score.hundredths),
                needs_review: !passed || !deviations.is_empty(),
                deviations,
            }
        }
        Err(err) => {
            ui.log_warning(format!("Warning: could not measure quality: {err}"));
            ConversionOutcome {
                score: None,
                needs_review: must_measure,
                deviations,
            }
        }
    }
}

// AI-FUNC-SUMMARY: Commits a finished conversion, either replacing the original or keeping both for review; returns the conversion sizes or a terminal error; side effects: moves files and may write a review record.
fn finish_conversion(
    options: &RunOptions,
    item: &WorkItem,
    outcome: ConversionOutcome,
    ui: &mut ProgressUi,
) -> Result<ConversionResult, String> {
    if !outcome.needs_review {
        return commit_validated_output(item, options.keep_original, ui);
    }

    commit_for_review(options, item, outcome, ui)
}

// AI-FUNC-SUMMARY: Moves a finished conversion beside its untouched original and records why it needs a decision; returns the conversion sizes or a terminal error; side effects: moves the temp file and writes a review record.
fn commit_for_review(
    options: &RunOptions,
    item: &WorkItem,
    outcome: ConversionOutcome,
    ui: &mut ProgressUi,
) -> Result<ConversionResult, String> {
    let source_bytes = fs::metadata(&item.input_path).map(|meta| meta.len()).ok();
    let output_bytes = fs::metadata(&item.temp_path).map(|meta| meta.len()).ok();

    let review_path = review::review_output_path(&item.output_path);
    if review_path.exists() {
        return Err(format!(
            "a previous review file is already waiting at {}",
            review_path.to_string_lossy()
        ));
    }

    move_validated_output_with_ui(&item.temp_path, &review_path, ui)?;

    let review_item = ReviewItem {
        original_path: item.input_path.clone(),
        review_path: review_path.clone(),
        sidecar_path: review::review_sidecar_path(&review_path),
        final_path: item.output_path.clone(),
        original_bytes: source_bytes.unwrap_or(0),
        review_bytes: output_bytes.unwrap_or(0),
        encoder: ui.last_encoder.clone(),
        codec: options.target_codec.as_str().to_string(),
        quality: ui.last_quality.clone(),
        score: outcome.score,
        target: options.quality_target,
        deviations: outcome
            .deviations
            .iter()
            .map(|(kind, detail)| review::Deviation::new(kind.clone(), detail.clone()))
            .collect(),
    };

    review::write_sidecar(&review_item)?;
    ui.emit_review_pending(&review_item);

    Ok(ConversionResult {
        source_bytes: source_bytes.unwrap_or(0),
        output_bytes: output_bytes.unwrap_or(0),
    })
}
// AI-FUNC-SUMMARY: Executes one exact or explicitly adapted encoder attempt without committing it; returns success or classified failure; side effects: removes an old temp, runs ffmpeg, and validates or repairs the generated output.
fn execute_transcode_attempt(
    options: &RunOptions,
    item: &WorkItem,
    plan: &TranscodePlan<'_>,
    metadata_filter: Option<&str>,
    ui: &mut ProgressUi,
) -> Result<(), TranscodeAttemptError> {
    remove_failed_attempt_output(&item.temp_path)?;

    let args = build_ffmpeg_args_for_plan(item, plan, options.target_codec);
    let run = run_ffmpeg_with_progress(&options.ffmpeg, &args, item, plan.encoder, ui)
        .map_err(TranscodeAttemptError::terminal)?;
    if !run.status.success() {
        let message = if run.stderr.trim().is_empty() {
            format!("ffmpeg exited with status {}", run.status)
        } else {
            first_error_line(&run.stderr)
        };
        return if ffmpeg_failure_is_encoder_retryable(&run.stderr) {
            Err(TranscodeAttemptError::retryable(message))
        } else {
            Err(TranscodeAttemptError::terminal(message))
        };
    }

    validate_or_repair_output(
        options,
        item,
        plan,
        options.target_codec,
        metadata_filter,
        ui,
    )
    .map_err(|err| {
        if validation_failure_is_retryable(&err) {
            TranscodeAttemptError::retryable(err)
        } else {
            TranscodeAttemptError::terminal(err)
        }
    })
}

// AI-FUNC-SUMMARY: Removes a failed-attempt temp output before reuse; returns success or terminal disk error; side effects: may delete one MyVidComp temporary file.
fn remove_failed_attempt_output(path: &Path) -> Result<(), TranscodeAttemptError> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(TranscodeAttemptError::terminal(format!(
            "failed to remove temporary output {}: {err}",
            path.to_string_lossy()
        ))),
    }
}

// AI-FUNC-SUMMARY: Reads sizes and commits one validated output; returns conversion sizes or terminal error; side effects: may rename, copy, or delete source/output files through commit policy.
fn commit_validated_output(
    item: &WorkItem,
    keep_original: bool,
    ui: &mut ProgressUi,
) -> Result<ConversionResult, String> {
    let result = ConversionResult {
        source_bytes: file_size(&item.input_path)?,
        output_bytes: file_size(&item.temp_path)?,
    };
    commit_output(item, keep_original, ui)?;
    Ok(result)
}

// AI-FUNC-SUMMARY: Classifies an ffmpeg failure as a safe encoder fallback case; returns true only for known device/encoder/format capability diagnostics; side effects: none.
fn ffmpeg_failure_is_encoder_retryable(stderr: &str) -> bool {
    let detail = stderr.to_ascii_lowercase();
    let terminal_markers = [
        "no space left",
        "permission denied",
        "read-only file system",
        "input/output error",
        "invalid data found when processing input",
        "moov atom not found",
        "error while decoding",
        "error opening output",
    ];
    if terminal_markers
        .iter()
        .any(|marker| detail.contains(marker))
    {
        return false;
    }

    let retryable_markers = [
        "no capable devices found",
        "no nvenc capable devices",
        "cannot load nvcuda",
        "device does not support",
        "codec not supported",
        "unsupported pixel format",
        "pixel format is not supported",
        "unsupported profile",
        "unsupported dimensions",
        "invalid frame dimensions",
        "error while opening encoder",
        "failed to open encoder",
        "encoder initialization failed",
        "initialize encoder failed",
        "impossible to convert between the formats",
        "error reinitializing filters",
        // Hardware encoder wrappers that give up part-way through.
        "operation not permitted",
        "generic error in an external library",
    ];
    if retryable_markers
        .iter()
        .any(|marker| detail.contains(marker))
    {
        return true;
    }

    // ffmpeg can die without explaining itself, usually when a hardware
    // wrapper crashes. Treating that as terminal throws away the file for
    // something the next encoder may well handle, so retry instead.
    !detail
        .lines()
        .any(|line| line.contains("error") || line.contains("failed") || line.contains("invalid"))
}

// AI-FUNC-SUMMARY: Classifies strict output validation failures for encoder fallback; returns false for tool/process/repair infrastructure failures; side effects: none.
fn validation_failure_is_retryable(error: &str) -> bool {
    !error.contains("failed to run ffprobe")
        && !error.contains("failed to inspect streams")
        && !error.contains("ffprobe failed")
        && !error.contains("metadata repair also failed")
}

// AI-FUNC-SUMMARY: Validates validate or repair output conditions; returns success, failure, or test assertion result; side effects: may inspect files or emit test failures.
fn validate_or_repair_output(
    options: &RunOptions,
    item: &WorkItem,
    plan: &TranscodePlan<'_>,
    codec: TargetCodec,
    metadata_filter: Option<&str>,
    ui: &mut ProgressUi,
) -> Result<(), String> {
    match validate_output_for_plan(&options.ffprobe, &item.temp_path, item, plan, codec) {
        Ok(()) => Ok(()),
        Err(first_err) => {
            let Some(filter) = metadata_filter else {
                return Err(first_err);
            };

            if !validation_error_may_need_remux_repair(&first_err) {
                return Err(first_err);
            }

            repair_av1_metadata(&options.ffmpeg, &options.ffprobe, item, filter, ui).map_err(
                |repair_err| format!("{first_err}; metadata repair also failed: {repair_err}"),
            )?;
            validate_output_for_plan(&options.ffprobe, &item.temp_path, item, plan, codec)
                .map_err(|second_err| format!("{first_err}; after metadata repair: {second_err}"))
        }
    }
}

// AI-FUNC-SUMMARY:
// Purpose: Finds a previously generated MyVidComp temp that validates under the current exact or hardware-adapted policy.
// Inputs: Runtime options, source work item, optional metadata repair filter, and progress UI.
// Returns: A reusable cache path, none when caches are invalid, or a terminal probe/repair infrastructure error.
// Side effects: Runs probes, may metadata-repair or delete unusable MyVidComp temps, and emits progress/log events.
fn prepare_cached_temp_output(
    options: &RunOptions,
    item: &WorkItem,
    metadata_filter: Option<&str>,
    ui: &mut ProgressUi,
) -> Result<Option<PathBuf>, String> {
    let cached_paths = cached_temp_output_paths(&item.input_path, options.tmp_dir.as_deref());
    for cache_path in cached_paths {
        if cache_path == item.temp_path {
            continue;
        }

        let mut cached_item = item.clone();
        cached_item.temp_path = cache_path.clone();
        ui.render_stage(&format!(
            "checking cached temp {}",
            cache_path.to_string_lossy()
        ));

        match validate_cached_output(options, &cache_path, &cached_item) {
            Ok(()) => {
                ui.log_info(format!(
                    "Info: reusing validated cached temp {}",
                    cache_path.to_string_lossy()
                ));
                return Ok(Some(cache_path));
            }
            Err(err) => {
                if !validation_failure_is_retryable(&err) {
                    return Err(err);
                }
                if !validation_error_may_need_remux_repair(&err) {
                    remove_unusable_temp_output(&cache_path, &err, ui);
                    ui.log_info(format!(
                        "Info: cached temp {} is not reusable: {err}",
                        cache_path.to_string_lossy()
                    ));
                    continue;
                }
                ui.log_info(format!(
                    "Info: cached temp {} needs repair before reuse: {err}",
                    cache_path.to_string_lossy()
                ));
            }
        }

        if let Some(filter) = metadata_filter
            && let Err(err) =
                repair_av1_metadata(&options.ffmpeg, &options.ffprobe, &cached_item, filter, ui)
        {
            return Err(format!(
                "cached temp {} metadata repair failed: {err}",
                cache_path.to_string_lossy()
            ));
        }

        match validate_cached_output(options, &cache_path, &cached_item) {
            Ok(()) => {
                ui.log_info(format!(
                    "Info: reusing validated cached temp {}",
                    cache_path.to_string_lossy()
                ));
                return Ok(Some(cache_path));
            }
            Err(err) => {
                if !validation_failure_is_retryable(&err) {
                    return Err(err);
                }
                remove_unusable_temp_output(&cache_path, &err, ui);
                ui.log_info(format!(
                    "Info: cached temp {} is not reusable: {err}",
                    cache_path.to_string_lossy()
                ));
            }
        }
    }

    Ok(None)
}

// AI-FUNC-SUMMARY: Validates a cached output against exact preservation or the current hardware-mode adaptation; returns success for either allowed plan; side effects: runs ffprobe.
fn validate_cached_output(
    options: &RunOptions,
    path: &Path,
    item: &WorkItem,
) -> Result<(), String> {
    match validate_output(&options.ffprobe, path, item) {
        Ok(()) => Ok(()),
        Err(exact_err)
            if options.encoder_preference == EncoderPreference::Gpu
                && validation_failure_is_retryable(&exact_err) =>
        {
            let Some((output_pix_fmt, description)) = closest_hardware_pixel_format(&item.video)
            else {
                return Err(exact_err);
            };
            // A cached temp may have been produced by an earlier adapted
            // attempt, so accept it against that target rather than discarding
            // work that is still valid.
            let encoder = Encoder {
                name: String::new(),
                kind: EncoderKind::Hardware,
            };
            let plan = TranscodePlan {
                encoder: &encoder,
                kind: TranscodePlanKind::Adapted {
                    output_pix_fmt,
                    description,
                },
                quality: EncoderQuality::Constant(0.0),
                exempt: MetadataExemptions::default(),
            };
            validate_output_for_plan(&options.ffprobe, path, item, &plan, options.target_codec)
        }
        Err(err) => Err(err),
    }
}

// AI-FUNC-SUMMARY: Checks whether a validation failure may be fixed by a no-reencode remux or AV1 metadata repair; returns true for stream-count and color/chroma metadata failures; side effects: none.
fn validation_error_may_need_remux_repair(error: &str) -> bool {
    error.contains("stream count") || error.contains("color ") || error.contains("chroma ")
}

// AI-FUNC-SUMMARY: Checks whether a validation failure means a temp output cannot be safely reused; returns true for unreadable or structurally wrong outputs; side effects: none.
fn validation_error_indicates_unusable_temp(error: &str) -> bool {
    error.contains("has no readable video stream")
        || error.contains("output codec is")
        || error.contains("output duration is not valid")
}

// AI-FUNC-SUMMARY: Deletes a temp output after an unrecoverable validation failure; returns nothing; side effects: may remove a MyVidComp temporary file and write cleanup warnings/events.
fn remove_unusable_temp_output(path: &Path, validation_error: &str, ui: &mut ProgressUi) {
    if !validation_error_indicates_unusable_temp(validation_error)
        || !is_temp_output_path(path)
        || !path.exists()
    {
        return;
    }

    match fs::remove_file(path) {
        Ok(()) => ui.log_info(format!(
            "Info: removed unusable temp output {}",
            path.to_string_lossy()
        )),
        Err(err) => ui.log_warning(format!(
            "Warning: failed to remove unusable temp output {}: {err}",
            path.to_string_lossy()
        )),
    }
}

// AI-FUNC-SUMMARY: Checks whether a path looks like a temporary conversion output owned by this tool; returns true for current .myvidcomp-*.tmp.mp4/.mkv names and legacy .pvac-* names; side effects: none.
fn is_temp_output_path(path: &Path) -> bool {
    path.file_name()
        .and_then(OsStr::to_str)
        .is_some_and(|name| {
            (name.starts_with(TEMP_PREFIX) || name.starts_with(LEGACY_TEMP_PREFIX))
                && (name.ends_with(".tmp.mp4") || name.ends_with(".tmp.mkv"))
        })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ConversionResult {
    source_bytes: u64,
    output_bytes: u64,
}

// AI-FUNC-SUMMARY: Provides file size behavior; returns the declared result; side effects: see implementation.
fn file_size(path: &Path) -> Result<u64, String> {
    fs::metadata(path)
        .map(|metadata| metadata.len())
        .map_err(|err| {
            format!(
                "failed to read file size for {}: {err}",
                path.to_string_lossy()
            )
        })
}

#[cfg(test)]
// AI-FUNC-SUMMARY: Builds the ffmpeg arguments for a plain exact AV1 attempt, for tests; returns command arguments; side effects: none.
fn build_ffmpeg_args(item: &WorkItem, encoder: &Encoder) -> Vec<String> {
    let plan = TranscodePlan {
        encoder,
        kind: TranscodePlanKind::Exact,
        quality: default_encoder_quality(item, encoder, TargetCodec::Av1),
        exempt: MetadataExemptions::default(),
    };
    build_ffmpeg_args_for_plan(item, &plan, TargetCodec::Av1)
}

// AI-FUNC-SUMMARY: Builds ffmpeg arguments for one encode attempt; returns command arguments; side effects: none.
fn build_ffmpeg_args_for_plan(
    item: &WorkItem,
    plan: &TranscodePlan<'_>,
    codec: TargetCodec,
) -> Vec<String> {
    let encoder = plan.encoder;
    let plan_kind = plan.kind;
    // Decoding is deliberately left to the processor. Asking ffmpeg for
    // automatic hardware decoding crashed it outright on roughly one run in six
    // during testing, losing the whole file's work, and the encode dominates the
    // time anyway. Hardware acceleration for the encode side is unaffected.
    let mut args = vec![
        "-hide_banner".to_string(),
        "-nostdin".to_string(),
        "-y".to_string(),
        "-stats_period".to_string(),
        "1".to_string(),
        "-i".to_string(),
        item.input_path.to_string_lossy().into_owned(),
    ];

    args.extend(stream_map_args(&item.mapped_stream_indexes));
    args.extend([
        "-map_metadata".to_string(),
        "0".to_string(),
        "-map_chapters".to_string(),
        item.chapter_policy.map_value().to_string(),
        "-c".to_string(),
        "copy".to_string(),
        "-c:v:0".to_string(),
        encoder.name.clone(),
    ]);

    args.extend(encoder_quality_args(encoder, &plan.quality));

    // Apple players only decode HEVC in MP4 when it carries the hvc1 tag, and
    // FFmpeg writes the other one by default.
    if item.output_container == OutputContainer::Mp4
        && let Some(tag) = codec.mp4_tag()
    {
        args.extend(["-tag:v".to_string(), tag.to_string()]);
    }

    if let Some(frame_rate) = output_frame_rate_arg(&item.video) {
        args.extend(["-r:v:0".to_string(), frame_rate.to_string()]);
    }

    if let Some(pix_fmt) = plan_kind
        .output_pix_fmt()
        .or_else(|| output_pix_fmt_arg(&item.video))
    {
        args.extend(["-pix_fmt:v:0".to_string(), pix_fmt.to_string()]);
    }

    args.extend(color_metadata_args(&item.video));
    args.extend(aspect_metadata_args(&item.video));
    if let Some(filter) = metadata_bsf_arg(codec, &item.video, &plan.exempt) {
        args.extend(["-bsf:v:0".to_string(), filter]);
    }

    if item.output_container == OutputContainer::Mkv {
        args.extend([
            "-f".to_string(),
            item.output_container.ffmpeg_format().to_string(),
            "-fps_mode:v:0".to_string(),
            "cfr".to_string(),
            "-progress".to_string(),
            "pipe:1".to_string(),
            item.temp_path.to_string_lossy().into_owned(),
        ]);
    } else {
        args.extend([
            "-movflags".to_string(),
            "+faststart".to_string(),
            "-fps_mode:v:0".to_string(),
            "cfr".to_string(),
            "-progress".to_string(),
            "pipe:1".to_string(),
            item.temp_path.to_string_lossy().into_owned(),
        ]);
    }

    args
}

// AI-FUNC-SUMMARY: Checks that this ffmpeg build carries the bitstream filter one codec needs for colour metadata; returns success or a user-facing error; side effects: runs ffmpeg once to describe the filter.
fn ensure_metadata_bsf(ffmpeg: &str, codec: TargetCodec) -> Result<(), String> {
    let Some(filter) = codec::metadata_filter(codec) else {
        return Ok(());
    };

    let output = process_command(ffmpeg)
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-h",
            &format!("bsf={}", filter.name),
        ])
        .output()
        .map_err(|err| {
            format!(
                "failed to inspect the ffmpeg {} bitstream filter: {err}",
                filter.name
            )
        })?;

    let detail = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    if output.status.success() && detail.contains(&format!("Bit stream filter {}", filter.name)) {
        Ok(())
    } else {
        Err(format!(
            "this ffmpeg build has no {} bitstream filter, which is needed to keep the source colour metadata: {}",
            filter.name,
            first_error_line(&detail)
        ))
    }
}

// AI-FUNC-SUMMARY: Performs repair av1 metadata operation; returns operation status or result; side effects: may spawn processes, move files, or write progress.
fn repair_av1_metadata(
    ffmpeg: &str,
    ffprobe: &str,
    item: &WorkItem,
    filter: &str,
    ui: &mut ProgressUi,
) -> Result<(), String> {
    ui.render_stage("repairing AV1 metadata");
    let repair_path = metadata_repair_temp_path(&item.temp_path);
    let backup_path = metadata_backup_temp_path(&item.temp_path);
    let repair_stream_indexes = probe_streams(ffprobe, &item.temp_path)?
        .into_iter()
        .map(|stream| stream.index)
        .collect::<Vec<_>>();
    if repair_stream_indexes.len() != item.stream_signature.len() {
        return Err(format!(
            "cannot repair output with unexpected stream count {} (expected {})",
            repair_stream_indexes.len(),
            item.stream_signature.len()
        ));
    }
    let args = build_av1_metadata_repair_args(
        &item.temp_path,
        &repair_path,
        &item.video,
        filter,
        &repair_stream_indexes,
        &item.chapter_policy,
        item.output_container,
    );
    let output = process_command(ffmpeg)
        .args(&args)
        .output()
        .map_err(|err| format!("failed to start ffmpeg AV1 metadata repair: {err}"))?;

    if !output.status.success() {
        let _ = fs::remove_file(&repair_path);
        return Err(format!(
            "failed to repair AV1 metadata for {}: {}",
            item.temp_path.to_string_lossy(),
            ffmpeg_failure_detail(&output)
        ));
    }

    replace_with_repaired_output(&item.temp_path, &repair_path, &backup_path)
}

// AI-FUNC-SUMMARY: Builds or derives build av1 metadata repair args data; returns the computed value; side effects: none.
fn build_av1_metadata_repair_args(
    source: &Path,
    destination: &Path,
    video: &VideoInfo,
    filter: &str,
    stream_indexes: &[usize],
    chapter_policy: &ChapterPolicy,
    output_container: OutputContainer,
) -> Vec<String> {
    let mut args = vec![
        "-hide_banner".to_string(),
        "-nostdin".to_string(),
        "-n".to_string(),
        "-i".to_string(),
        source.to_string_lossy().into_owned(),
    ];

    args.extend(stream_map_args(stream_indexes));
    args.extend([
        "-map_metadata".to_string(),
        "0".to_string(),
        "-map_chapters".to_string(),
        chapter_policy.map_value().to_string(),
        "-c".to_string(),
        "copy".to_string(),
    ]);

    args.extend(color_metadata_args(video));
    args.extend(["-bsf:v:0".to_string(), filter.to_string()]);

    if output_container == OutputContainer::Mkv {
        args.extend([
            "-f".to_string(),
            output_container.ffmpeg_format().to_string(),
            destination.to_string_lossy().into_owned(),
        ]);
    } else {
        args.extend([
            "-movflags".to_string(),
            "+faststart+write_colr".to_string(),
            destination.to_string_lossy().into_owned(),
        ]);
    }
    args
}

// AI-FUNC-SUMMARY: Builds explicit FFmpeg mappings from actual probed stream indexes; returns one -map pair per index and never emits a broad map; side effects: none.
fn stream_map_args(stream_indexes: &[usize]) -> Vec<String> {
    let mut args = Vec::with_capacity(stream_indexes.len() * 2);
    for index in stream_indexes {
        args.extend(["-map".to_string(), format!("0:{index}")]);
    }
    args
}

// AI-FUNC-SUMMARY: Performs replace with repaired output operation; returns operation status or result; side effects: may spawn processes, move files, or write progress.
fn replace_with_repaired_output(
    original: &Path,
    repaired: &Path,
    backup: &Path,
) -> Result<(), String> {
    fs::rename(original, backup).map_err(|err| {
        let _ = fs::remove_file(repaired);
        format!(
            "failed to stage original temp output {} for metadata repair: {err}",
            original.to_string_lossy()
        )
    })?;

    if let Err(err) = fs::rename(repaired, original) {
        let _ = fs::rename(backup, original);
        let _ = fs::remove_file(repaired);
        return Err(format!(
            "failed to replace temp output {} with metadata-repaired output: {err}",
            original.to_string_lossy()
        ));
    }

    let _ = fs::remove_file(backup);
    Ok(())
}

// AI-FUNC-SUMMARY: Provides ffmpeg failure detail behavior; returns the declared result; side effects: see implementation.
fn ffmpeg_failure_detail(output: &std::process::Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !stderr.trim().is_empty() {
        return first_error_line(&stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.trim().is_empty() {
        return first_error_line(&stdout);
    }

    format!("ffmpeg exited with status {}", output.status)
}

// AI-FUNC-SUMMARY: Provides output frame rate arg behavior; returns the declared result; side effects: see implementation.
fn output_frame_rate_arg(video: &VideoInfo) -> Option<&str> {
    video
        .nominal_frame_rate
        .as_deref()
        .or(video.avg_frame_rate.as_deref())
}

// AI-FUNC-SUMMARY: Provides output pix fmt arg behavior; returns the declared result; side effects: see implementation.
fn output_pix_fmt_arg(video: &VideoInfo) -> Option<&str> {
    video.pix_fmt.as_deref().map(normalized_pix_fmt)
}

// AI-FUNC-SUMMARY: Builds or derives normalized pix fmt data; returns the computed value; side effects: none.
fn normalized_pix_fmt(pix_fmt: &str) -> &str {
    match pix_fmt {
        "yuvj420p" => "yuv420p",
        "yuvj422p" => "yuv422p",
        "yuvj444p" => "yuv444p",
        _ => pix_fmt,
    }
}

// AI-FUNC-SUMMARY: Provides color metadata args behavior; returns the declared result; side effects: see implementation.
fn color_metadata_args(video: &VideoInfo) -> Vec<String> {
    let mut args = Vec::new();
    push_optional_arg(&mut args, "-color_range:v:0", video.color_range.as_deref());
    push_optional_arg(&mut args, "-colorspace:v:0", video.color_space.as_deref());
    push_optional_arg(&mut args, "-color_trc:v:0", video.color_transfer.as_deref());
    push_optional_arg(
        &mut args,
        "-color_primaries:v:0",
        video.color_primaries.as_deref(),
    );
    push_optional_arg(
        &mut args,
        "-chroma_sample_location:v:0",
        video.chroma_location.as_deref(),
    );
    args
}

// AI-FUNC-SUMMARY: Builds the bitstream-filter argument that carries the source colour metadata into the encoded stream; returns the argument or none when there is nothing to carry; side effects: none.
fn metadata_bsf_arg(
    codec: TargetCodec,
    video: &VideoInfo,
    exempt: &MetadataExemptions,
) -> Option<String> {
    // Each codec names these options differently, and H.266 has no filter that
    // can state them at all.
    let filter = codec::metadata_filter(codec)?;
    let mut options = Vec::new();

    if !exempt.color_primaries
        && let Some(value) = video
            .color_primaries
            .as_deref()
            .and_then(av1_color_primaries_value)
    {
        options.push(format!("{}={value}", filter.color_primaries));
    }

    if !exempt.color_transfer
        && let Some(value) = video
            .color_transfer
            .as_deref()
            .and_then(av1_transfer_characteristics_value)
    {
        options.push(format!("{}={value}", filter.transfer));
    }

    if !exempt.color_space
        && let Some(value) = video
            .color_space
            .as_deref()
            .and_then(av1_matrix_coefficients_value)
    {
        options.push(format!("{}={value}", filter.matrix));
    }

    if !exempt.color_range
        && let Some(value) = video
            .color_range
            .as_deref()
            .and_then(|value| codec::color_range_value(codec, value))
    {
        options.push(format!("{}={value}", filter.range));
    }

    if !exempt.chroma_location
        && let Some(option) = filter.chroma
        && let Some(value) = video
            .chroma_location
            .as_deref()
            .and_then(|value| codec::chroma_location_value(codec, value))
    {
        options.push(format!("{option}={value}"));
    }

    (!options.is_empty()).then(|| format!("{}={}", filter.name, options.join(":")))
}

// AI-FUNC-SUMMARY: Reports the first colour detail the chosen codec cannot record; returns a skip reason or none when everything can be preserved; side effects: none.
fn unsupported_metadata_reason(codec: TargetCodec, video: &VideoInfo) -> Option<String> {
    let name = codec.label();

    if codec::metadata_filter(codec).is_none() {
        let has_metadata = video.color_primaries.is_some()
            || video.color_transfer.is_some()
            || video.color_space.is_some()
            || video.color_range.is_some()
            || video.chroma_location.is_some();
        return has_metadata.then(|| format!("{name} cannot record this video's colour metadata"));
    }

    if let Some(value) = video.color_primaries.as_deref()
        && av1_color_primaries_value(value).is_none()
    {
        return Some(format!("{name} cannot record the colour space {value}"));
    }

    if let Some(value) = video.color_transfer.as_deref()
        && av1_transfer_characteristics_value(value).is_none()
    {
        return Some(format!("{name} cannot record the brightness curve {value}"));
    }

    if let Some(value) = video.color_space.as_deref()
        && av1_matrix_coefficients_value(value).is_none()
    {
        return Some(format!("{name} cannot record the colour matrix {value}"));
    }

    if let Some(value) = video.color_range.as_deref()
        && codec::color_range_value(codec, value).is_none()
    {
        return Some(format!("{name} cannot record the colour range {value}"));
    }

    if let Some(value) = video.chroma_location.as_deref()
        && codec::chroma_location_value(codec, value).is_none()
    {
        return Some(format!("{name} cannot record the chroma position {value}"));
    }

    None
}

// AI-FUNC-SUMMARY: Provides av1 color primaries value behavior; returns the declared result; side effects: see implementation.
fn av1_color_primaries_value(value: &str) -> Option<&'static str> {
    let value = metadata_key(value);
    match value.as_str() {
        "bt709" => Some("1"),
        "bt470m" => Some("4"),
        "bt470bg" => Some("5"),
        "smpte170m" => Some("6"),
        "smpte240m" => Some("7"),
        "film" => Some("8"),
        "bt2020" => Some("9"),
        "smpte428" | "smpte428-1" => Some("10"),
        "smpte431" => Some("11"),
        "smpte432" => Some("12"),
        "ebu3213" | "jedec-p22" => Some("22"),
        _ => None,
    }
}

// AI-FUNC-SUMMARY: Provides av1 transfer characteristics value behavior; returns the declared result; side effects: see implementation.
fn av1_transfer_characteristics_value(value: &str) -> Option<&'static str> {
    let value = metadata_key(value);
    match value.as_str() {
        "bt709" => Some("1"),
        "bt470m" | "gamma22" => Some("4"),
        "bt470bg" | "gamma28" => Some("5"),
        "smpte170m" => Some("6"),
        "smpte240m" => Some("7"),
        "linear" => Some("8"),
        "log" | "log100" => Some("9"),
        "log-sqrt" | "log316" => Some("10"),
        "iec61966-2-4" => Some("11"),
        "bt1361e" | "bt1361" => Some("12"),
        "iec61966-2-1" | "srgb" => Some("13"),
        "bt2020-10" => Some("14"),
        "bt2020-12" => Some("15"),
        "smpte2084" => Some("16"),
        "smpte428" | "smpte428-1" => Some("17"),
        "arib-std-b67" | "hlg" => Some("18"),
        _ => None,
    }
}

// AI-FUNC-SUMMARY: Provides av1 matrix coefficients value behavior; returns the declared result; side effects: see implementation.
fn av1_matrix_coefficients_value(value: &str) -> Option<&'static str> {
    let value = metadata_key(value);
    match value.as_str() {
        "gbr" | "rgb" => Some("0"),
        "bt709" => Some("1"),
        "fcc" => Some("4"),
        "bt470bg" => Some("5"),
        "smpte170m" => Some("6"),
        "smpte240m" => Some("7"),
        "ycgco" => Some("8"),
        "bt2020nc" | "bt2020-nc" | "bt2020ncl" | "bt2020-ncl" => Some("9"),
        "bt2020c" | "bt2020-c" | "bt2020cl" | "bt2020-cl" => Some("10"),
        "smpte2085" => Some("11"),
        "chroma-derived-nc" | "chromaderivednc" => Some("12"),
        "chroma-derived-c" | "chromaderivedc" => Some("13"),
        "ictcp" => Some("14"),
        _ => None,
    }
}

// AI-FUNC-SUMMARY: Provides metadata key behavior; returns the declared result; side effects: see implementation.
fn metadata_key(value: &str) -> String {
    value.trim().to_ascii_lowercase().replace('_', "-")
}

// AI-FUNC-SUMMARY: Provides aspect metadata args behavior; returns the declared result; side effects: see implementation.
fn aspect_metadata_args(video: &VideoInfo) -> Vec<String> {
    let mut args = Vec::new();
    push_optional_arg(&mut args, "-aspect", video.display_aspect_ratio.as_deref());
    args
}

// AI-FUNC-SUMMARY: Provides push optional arg behavior; returns the declared result; side effects: see implementation.
fn push_optional_arg(args: &mut Vec<String>, name: &str, value: Option<&str>) {
    if let Some(value) = value {
        args.extend([name.to_string(), value.to_string()]);
    }
}

/// A concrete quality setting for one encoder, already converted onto that
/// encoder's own scale.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EncoderQuality {
    /// A constant-quality value. Lower is better quality.
    Constant(f64),
    /// A target bitrate in bits per second, for encoders with no usable
    /// constant-quality mode.
    Bitrate(u64),
}

impl EncoderQuality {
    // AI-FUNC-SUMMARY: Describes a quality setting for logs and events; returns a short label; side effects: none.
    fn label(&self, kind: EncoderKind) -> String {
        match self {
            EncoderQuality::Constant(value) => {
                let name = match codec::quality_style(kind) {
                    codec::QualityStyle::Quantizer => "qp",
                    _ => "crf",
                };
                if (value.fract()).abs() < f64::EPSILON {
                    format!("{name}={value:.0}")
                } else {
                    format!("{name}={value:.1}")
                }
            }
            EncoderQuality::Bitrate(bits) => Bitrate(*bits).to_string(),
        }
    }
}

// AI-FUNC-SUMMARY: Chooses the starting quality setting for one file and encoder; returns the setting on that encoder's own scale; side effects: none.
fn default_encoder_quality(
    item: &WorkItem,
    encoder: &Encoder,
    codec: TargetCodec,
) -> EncoderQuality {
    match codec::quality_style(encoder.kind) {
        codec::QualityStyle::Bitrate => {
            EncoderQuality::Bitrate(estimate_target_bitrate(item, codec).0)
        }
        _ => EncoderQuality::Constant(codec::convert_quality(item.estimated_crf, encoder.kind)),
    }
}

// AI-FUNC-SUMMARY: Formats a constant-quality value the way its encoder expects it; returns the argument text; side effects: none.
fn quality_value_arg(value: f64, kind: EncoderKind) -> String {
    match kind {
        // x265 is the only encoder here that reads fractional CRF values.
        EncoderKind::X265 => format!("{value:.1}"),
        _ => format!("{:.0}", value.round()),
    }
}

// AI-FUNC-SUMMARY: Builds the quality arguments for one encoder at one setting; returns the argument list; side effects: none.
fn encoder_quality_args(encoder: &Encoder, quality: &EncoderQuality) -> Vec<String> {
    let kind = encoder.kind;
    match (kind, quality) {
        (_, EncoderQuality::Bitrate(bits)) => {
            let mut args = vec!["-b:v".to_string(), Bitrate(*bits).to_string()];
            if kind == EncoderKind::MediaCodec {
                // Android encoders default to a rate control that ignores the
                // requested bitrate on some devices.
                args.extend(["-bitrate_mode".to_string(), "1".to_string()]);
            }
            args
        }
        (EncoderKind::Vulkan, EncoderQuality::Constant(value)) => vec![
            "-crf".to_string(),
            quality_value_arg(*value, kind),
            "-b:v".to_string(),
            "0".to_string(),
        ],
        (EncoderKind::SvtAv1, EncoderQuality::Constant(value)) => vec![
            "-crf".to_string(),
            quality_value_arg(*value, kind),
            "-preset".to_string(),
            "6".to_string(),
        ],
        (EncoderKind::LibAom, EncoderQuality::Constant(value)) => vec![
            "-crf".to_string(),
            quality_value_arg(*value, kind),
            "-b:v".to_string(),
            "0".to_string(),
            "-cpu-used".to_string(),
            "4".to_string(),
        ],
        (EncoderKind::Rav1e, EncoderQuality::Constant(value)) => vec![
            "-qp".to_string(),
            quality_value_arg(*value, kind),
            "-speed".to_string(),
            "6".to_string(),
        ],
        (EncoderKind::X265, EncoderQuality::Constant(value)) => vec![
            "-crf".to_string(),
            quality_value_arg(*value, kind),
            "-preset".to_string(),
            "medium".to_string(),
        ],
        (EncoderKind::Vvenc, EncoderQuality::Constant(value)) => vec![
            "-qp".to_string(),
            quality_value_arg(*value, kind),
            "-preset".to_string(),
            // Even the fastest VVenC preset is slow; anything higher is not
            // usable for batch work.
            "faster".to_string(),
        ],
        (EncoderKind::Hardware | EncoderKind::MediaCodec, EncoderQuality::Constant(value)) => {
            vec!["-cq".to_string(), quality_value_arg(*value, kind)]
        }
    }
}

struct FfmpegRunOutput {
    status: ExitStatus,
    stderr: String,
}

// AI-FUNC-SUMMARY: Runs ffmpeg while reporting progress; returns exit status plus captured stderr or process-management error; side effects: starts ffmpeg and emits progress events.
fn run_ffmpeg_with_progress(
    ffmpeg: &str,
    args: &[String],
    item: &WorkItem,
    encoder: &Encoder,
    ui: &mut ProgressUi,
) -> Result<FfmpegRunOutput, String> {
    let mut child = process_command(ffmpeg)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| format!("failed to start ffmpeg: {err}"))?;

    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "failed to capture ffmpeg stderr".to_string())?;
    let stderr_handle = thread::spawn(move || read_to_string(stderr));

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "failed to capture ffmpeg progress".to_string())?;
    let reader = BufReader::new(stdout);
    let mut snapshot = ProgressSnapshot::default();

    for line in reader.lines() {
        let line = line.map_err(|err| format!("failed reading ffmpeg progress: {err}"))?;
        if let Some(update) = parse_progress_line(&line) {
            snapshot.apply(update);
            ui.render_file_progress(item, encoder, &snapshot);
        }
    }

    let status = child
        .wait()
        .map_err(|err| format!("failed waiting for ffmpeg: {err}"))?;
    let stderr_text = stderr_handle
        .join()
        .map_err(|_| "failed to join ffmpeg stderr reader".to_string())?
        .unwrap_or_default();

    Ok(FfmpegRunOutput {
        status,
        stderr: stderr_text,
    })
}

// AI-FUNC-SUMMARY: Provides read to string behavior; returns the declared result; side effects: see implementation.
fn read_to_string<R: Read>(mut reader: R) -> io::Result<String> {
    let mut text = String::new();
    reader.read_to_string(&mut text)?;
    Ok(text)
}

// AI-FUNC-SUMMARY: Validates validate output conditions; returns success, failure, or test assertion result; side effects: may inspect files or emit test failures.
fn validate_output(ffprobe: &str, path: &Path, item: &WorkItem) -> Result<(), String> {
    validate_output_against(
        ffprobe,
        path,
        item,
        &item.video,
        TargetCodec::Av1,
        &MetadataExemptions::default(),
    )
}

// AI-FUNC-SUMMARY: Validates output against an exact or adapted plan; returns success or validation error; side effects: runs ffprobe.
fn validate_output_for_plan(
    ffprobe: &str,
    path: &Path,
    item: &WorkItem,
    plan: &TranscodePlan<'_>,
    target_codec: TargetCodec,
) -> Result<(), String> {
    let mut expected = item.video.clone();
    if let Some(pix_fmt) = plan.kind.output_pix_fmt() {
        expected.pix_fmt = Some(pix_fmt.to_string());
    }
    validate_output_against(ffprobe, path, item, &expected, target_codec, &plan.exempt)
}

// AI-FUNC-SUMMARY: Validates probed output against expected primary-video properties and source stream layout; returns success or validation error; side effects: runs ffprobe.
fn validate_output_against(
    ffprobe: &str,
    path: &Path,
    item: &WorkItem,
    expected: &VideoInfo,
    codec: TargetCodec,
    exempt: &MetadataExemptions,
) -> Result<(), String> {
    let source = expected;
    let Some(info) = probe_video(ffprobe, path)? else {
        return Err(format!(
            "output has no readable video stream: {}",
            path.to_string_lossy()
        ));
    };

    if !info.codec_name.eq_ignore_ascii_case(codec.ffprobe_name()) {
        return Err(format!(
            "the converted file is {}, not the {} that was asked for: {}",
            info.codec_name,
            codec.ffprobe_name(),
            path.to_string_lossy()
        ));
    }

    if info.duration_seconds <= 0.0 {
        return Err(format!(
            "output duration is not valid: {}",
            path.to_string_lossy()
        ));
    }

    // Without this, a truncated encode that stopped after a few seconds passes
    // every other check, because resolution, frame rate and stream layout all
    // still match.
    if let Some(err) = duration_validation_error(source, &info, path) {
        return Err(err);
    }

    if info.width != source.width || info.height != source.height {
        return Err(format!(
            "output resolution changed from {}x{} to {}x{}: {}",
            source.width,
            source.height,
            info.width,
            info.height,
            path.to_string_lossy()
        ));
    }

    if let Some(err) = frame_rate_validation_error(source, &info, path) {
        return Err(err);
    }

    if let Some(err) = color_metadata_validation_error(source, &info, path, exempt) {
        return Err(err);
    }

    if let Some(err) = pixel_format_validation_error(source, &info, path) {
        return Err(err);
    }

    if let Some(err) = display_metadata_validation_error(source, &info, path) {
        return Err(err);
    }

    validate_stream_signature(ffprobe, path, &item.stream_signature, codec)?;
    validate_chapter_policy(ffprobe, path, &item.chapter_policy)?;

    Ok(())
}

// AI-FUNC-SUMMARY: Validates output chapters only for confirmed chapter-carrier sources; returns semantic timestamp/title mismatch detail or success; side effects: runs ffprobe when validation is required.
fn validate_chapter_policy(
    ffprobe: &str,
    path: &Path,
    policy: &ChapterPolicy,
) -> Result<(), String> {
    let expected = match policy {
        ChapterPolicy::Ordinary => return Ok(()),
        ChapterPolicy::ConfirmedEmpty => &[][..],
        ChapterPolicy::ConfirmedMeaningful(chapters) => chapters.as_slice(),
    };
    let output = probe_chapters(ffprobe, path)?;
    chapter_validation_error(expected, &output, path).map_or(Ok(()), Err)
}

// AI-FUNC-SUMMARY: Compares expected and output chapter count, timestamps, and exact titles; returns detailed mismatch with practical timestamp tolerance or none; side effects: none.
fn chapter_validation_error(
    expected: &[ChapterInfo],
    output: &[ChapterInfo],
    path: &Path,
) -> Option<String> {
    const CHAPTER_TIMESTAMP_TOLERANCE: f64 = 0.05;
    if expected.len() != output.len() {
        return Some(format!(
            "output chapter count changed from {} to {}: {}",
            expected.len(),
            output.len(),
            path.to_string_lossy()
        ));
    }
    for (index, (expected, output)) in expected.iter().zip(output).enumerate() {
        if (expected.start_seconds - output.start_seconds).abs() > CHAPTER_TIMESTAMP_TOLERANCE
            || (expected.end_seconds - output.end_seconds).abs() > CHAPTER_TIMESTAMP_TOLERANCE
        {
            return Some(format!(
                "output chapter #{index} timestamps changed from {:.6}-{:.6} to {:.6}-{:.6}: {}",
                expected.start_seconds,
                expected.end_seconds,
                output.start_seconds,
                output.end_seconds,
                path.to_string_lossy()
            ));
        }
        if expected.title != output.title {
            return Some(format!(
                "output chapter #{index} title changed from {:?} to {:?}: {}",
                expected.title,
                output.title,
                path.to_string_lossy()
            ));
        }
    }
    None
}

// AI-FUNC-SUMMARY: Provides pixel format validation error behavior; returns the declared result; side effects: see implementation.
fn pixel_format_validation_error(
    source: &VideoInfo,
    output: &VideoInfo,
    path: &Path,
) -> Option<String> {
    let source_pix_fmt = source.pix_fmt.as_deref()?;
    let Some(output_pix_fmt) = output.pix_fmt.as_deref() else {
        return Some(format!(
            "output pixel format is missing, expected {}: {}",
            normalized_pix_fmt(source_pix_fmt),
            path.to_string_lossy()
        ));
    };

    let source_pix_fmt = normalized_pix_fmt(source_pix_fmt);
    let output_pix_fmt = normalized_pix_fmt(output_pix_fmt);
    if source_pix_fmt != output_pix_fmt {
        return Some(format!(
            "output pixel format changed from {source_pix_fmt} to {output_pix_fmt}: {}",
            path.to_string_lossy()
        ));
    }

    None
}

// AI-FUNC-SUMMARY: Validates validate stream signature conditions; returns success, failure, or test assertion result; side effects: may inspect files or emit test failures.
fn validate_stream_signature(
    ffprobe: &str,
    path: &Path,
    source: &[StreamSignature],
    codec: TargetCodec,
) -> Result<(), String> {
    let output = probe_streams(ffprobe, path)?;
    if let Some(err) = stream_signature_validation_error(source, &output, path, codec) {
        return Err(err);
    }

    Ok(())
}

// AI-FUNC-SUMMARY: Provides stream signature validation error behavior; returns the declared result; side effects: see implementation.
fn stream_signature_validation_error(
    source: &[StreamSignature],
    output: &[StreamInfo],
    path: &Path,
    codec: TargetCodec,
) -> Option<String> {
    if output.len() != source.len() {
        return Some(format!(
            "output stream count changed from {} to {}: {}",
            source.len(),
            output.len(),
            path.to_string_lossy()
        ));
    }

    let mut seen_first_video = false;
    for (index, (source_stream, output_stream)) in source.iter().zip(output.iter()).enumerate() {
        if source_stream.codec_type != output_stream.codec_type {
            return Some(format!(
                "output stream #{index} type changed from {} to {}: {}",
                source_stream.codec_type,
                output_stream.codec_type,
                path.to_string_lossy()
            ));
        }

        if source_stream.codec_type == "video" && !seen_first_video {
            seen_first_video = true;
            if !output_stream
                .codec_name
                .eq_ignore_ascii_case(codec.ffprobe_name())
            {
                return Some(format!(
                    "the converted video stream is {}, not the {} that was asked for: {}",
                    output_stream.codec_name,
                    codec.ffprobe_name(),
                    path.to_string_lossy()
                ));
            }
            continue;
        }

        if source_stream.codec_name != output_stream.codec_name {
            return Some(format!(
                "output stream #{index} codec changed from {} to {}: {}",
                source_stream.codec_name,
                output_stream.codec_name,
                path.to_string_lossy()
            ));
        }
    }

    None
}

// AI-FUNC-SUMMARY: Provides display metadata validation error behavior; returns the declared result; side effects: see implementation.
fn display_metadata_validation_error(
    source: &VideoInfo,
    output: &VideoInfo,
    path: &Path,
) -> Option<String> {
    let checks = [
        (
            "sample aspect ratio",
            source.sample_aspect_ratio.as_deref(),
            output.sample_aspect_ratio.as_deref(),
        ),
        (
            "display aspect ratio",
            source.display_aspect_ratio.as_deref(),
            output.display_aspect_ratio.as_deref(),
        ),
        (
            "field order",
            source.field_order.as_deref(),
            output.field_order.as_deref(),
        ),
    ];

    for (label, source_value, output_value) in checks {
        let Some(source_value) = source_value else {
            continue;
        };

        match output_value {
            Some(output_value) if display_metadata_matches(label, source_value, output_value) => {}
            Some(output_value) => {
                return Some(format!(
                    "output {label} changed from {source_value} to {output_value}: {}",
                    path.to_string_lossy()
                ));
            }
            None => {
                return Some(format!(
                    "output {label} is missing, expected {source_value}: {}",
                    path.to_string_lossy()
                ));
            }
        }
    }

    None
}

// AI-FUNC-SUMMARY: Compares display metadata with rational tolerance for aspect ratios; returns true when values are equivalent enough for validation; side effects: none.
fn display_metadata_matches(label: &str, source_value: &str, output_value: &str) -> bool {
    if label == "sample aspect ratio" || label == "display aspect ratio" {
        return rational_metadata_matches(source_value, output_value);
    }

    source_value.eq_ignore_ascii_case(output_value)
}

// AI-FUNC-SUMMARY: Compares rational metadata values with a small tolerance for container rewrites; returns true when ratios are visually equivalent; side effects: none.
fn rational_metadata_matches(source_value: &str, output_value: &str) -> bool {
    const RATIO_TOLERANCE: f64 = 0.001;
    match (parse_ratio(source_value), parse_ratio(output_value)) {
        (Some(source), Some(output)) => (source - output).abs() <= RATIO_TOLERANCE,
        _ => source_value.eq_ignore_ascii_case(output_value),
    }
}

// AI-FUNC-SUMMARY: Compares the converted duration against the source; returns an error when the output is noticeably shorter or longer; side effects: none.
fn duration_validation_error(
    source: &VideoInfo,
    output: &VideoInfo,
    path: &Path,
) -> Option<String> {
    if source.duration_seconds <= 0.0 {
        return None;
    }

    // Container rounding and a trailing partial frame can move the reported
    // duration slightly, so allow the larger of half a second and one percent.
    let tolerance = (source.duration_seconds * 0.01).max(0.5);
    let difference = (output.duration_seconds - source.duration_seconds).abs();
    (difference > tolerance).then(|| {
        format!(
            "the converted file is {:.1}s long but the original is {:.1}s: {}",
            output.duration_seconds,
            source.duration_seconds,
            path.to_string_lossy()
        )
    })
}
// AI-FUNC-SUMMARY: Provides color metadata validation error behavior; returns the declared result; side effects: see implementation.
fn color_metadata_validation_error(
    source: &VideoInfo,
    output: &VideoInfo,
    path: &Path,
    exempt: &MetadataExemptions,
) -> Option<String> {
    let checks = [
        (
            "color range",
            source.color_range.as_deref(),
            output.color_range.as_deref(),
            exempt.color_range,
        ),
        (
            "color space",
            source.color_space.as_deref(),
            output.color_space.as_deref(),
            exempt.color_space,
        ),
        (
            "color transfer",
            source.color_transfer.as_deref(),
            output.color_transfer.as_deref(),
            exempt.color_transfer,
        ),
        (
            "color primaries",
            source.color_primaries.as_deref(),
            output.color_primaries.as_deref(),
            exempt.color_primaries,
        ),
        (
            "chroma location",
            source.chroma_location.as_deref(),
            output.chroma_location.as_deref(),
            exempt.chroma_location,
        ),
    ];

    for (label, source_value, output_value, is_exempt) in checks {
        // A detail this codec cannot record was already reported to the
        // user as a change, so its absence here is expected.
        if is_exempt {
            continue;
        }
        let Some(source_value) = source_value else {
            continue;
        };

        match output_value {
            Some(output_value) if color_metadata_matches(source_value, output_value) => {}
            Some(output_value) => {
                return Some(format!(
                    "output {label} changed from {source_value} to {output_value}: {}",
                    path.to_string_lossy()
                ));
            }
            None => {
                if missing_chroma_location_report_is_acceptable(label, output, path) {
                    continue;
                }
                return Some(format!(
                    "output {label} is missing, expected {source_value}: {}",
                    path.to_string_lossy()
                ));
            }
        }
    }

    None
}

// AI-FUNC-SUMMARY: Checks color metadata matches predicate; returns a boolean; side effects: none.
fn color_metadata_matches(source: &str, output: &str) -> bool {
    source.eq_ignore_ascii_case(output)
}

// AI-FUNC-SUMMARY: Checks missing chroma location report is acceptable predicate; returns a boolean; side effects: none.
fn missing_chroma_location_report_is_acceptable(
    label: &str,
    output: &VideoInfo,
    path: &Path,
) -> bool {
    label == "chroma location"
        && output.chroma_location.is_none()
        && output.codec_name.eq_ignore_ascii_case("av1")
        && path
            .extension()
            .and_then(OsStr::to_str)
            .is_some_and(|extension| extension.eq_ignore_ascii_case("mp4"))
}

// AI-FUNC-SUMMARY: Provides frame rate validation error behavior; returns the declared result; side effects: see implementation.
fn frame_rate_validation_error(
    source: &VideoInfo,
    output: &VideoInfo,
    path: &Path,
) -> Option<String> {
    if frame_rates_match(source, output) {
        return None;
    }

    Some(format!(
        "output frame rate changed from {} to {}: {}",
        frame_rate_label(source),
        frame_rate_label(output),
        path.to_string_lossy()
    ))
}

// AI-FUNC-SUMMARY: Provides frame rates match behavior; returns the declared result; side effects: see implementation.
fn frame_rates_match(source: &VideoInfo, output: &VideoInfo) -> bool {
    let pairs = [
        (source.nominal_fps, output.nominal_fps),
        (source.fps, output.fps),
        (source.nominal_fps, output.fps),
        (source.fps, output.nominal_fps),
    ];

    let mut comparable = false;
    for (source_fps, output_fps) in pairs {
        if source_fps > 0.0 && output_fps > 0.0 {
            comparable = true;
            if fps_matches(source_fps, output_fps) {
                return true;
            }
        }
    }

    !comparable
}

// AI-FUNC-SUMMARY: Provides frame rate label behavior; returns the declared result; side effects: see implementation.
fn frame_rate_label(info: &VideoInfo) -> String {
    match (info.fps > 0.0, info.nominal_fps > 0.0) {
        (true, true) => format!(
            "avg {:.3} fps / nominal {:.3} fps",
            info.fps, info.nominal_fps
        ),
        (true, false) => format!("avg {:.3} fps", info.fps),
        (false, true) => format!("nominal {:.3} fps", info.nominal_fps),
        (false, false) => "unknown fps".to_string(),
    }
}

// AI-FUNC-SUMMARY: Checks fps matches predicate; returns a boolean; side effects: none.
fn fps_matches(source: f64, output: f64) -> bool {
    (source - output).abs() <= 0.02
}

// AI-FUNC-SUMMARY: Performs commit output operation; returns operation status or result; side effects: may spawn processes, move files, or write progress.
fn commit_output(item: &WorkItem, keep_original: bool, ui: &mut ProgressUi) -> Result<(), String> {
    if keep_original {
        let old_path = item
            .old_path
            .as_ref()
            .ok_or_else(|| "missing old path".to_string())?;
        fs::rename(&item.input_path, old_path).map_err(|err| {
            format!(
                "failed to rename original {} to {}: {err}",
                item.input_path.to_string_lossy(),
                old_path.to_string_lossy()
            )
        })?;

        if let Err(err) = move_validated_output_with_ui(&item.temp_path, &item.output_path, ui) {
            let _ = fs::rename(old_path, &item.input_path);
            return Err(format!(
                "failed to move converted file to {}: {err}",
                item.output_path.to_string_lossy()
            ));
        }
        return Ok(());
    }

    if item.output_path == item.input_path {
        fs::rename(&item.input_path, &item.recovery_path).map_err(|err| {
            format!(
                "failed to create recovery copy {}: {err}",
                item.recovery_path.to_string_lossy()
            )
        })?;

        if let Err(err) = move_validated_output_with_ui(&item.temp_path, &item.output_path, ui) {
            let _ = fs::rename(&item.recovery_path, &item.input_path);
            return Err(format!(
                "failed to replace {}: {err}",
                item.output_path.to_string_lossy()
            ));
        }

        let _ = fs::remove_file(&item.recovery_path);
        return Ok(());
    }

    move_validated_output_with_ui(&item.temp_path, &item.output_path, ui).map_err(|err| {
        format!(
            "failed to move converted file to {}: {err}",
            item.output_path.to_string_lossy()
        )
    })?;
    fs::remove_file(&item.input_path).map_err(|err| {
        format!(
            "converted file was written, but failed to remove original {}: {err}",
            item.input_path.to_string_lossy()
        )
    })?;

    Ok(())
}

#[cfg(test)]
// AI-FUNC-SUMMARY: Performs move validated output operation; returns operation status or result; side effects: may spawn processes, move files, or write progress.
fn move_validated_output(source: &Path, destination: &Path) -> Result<(), String> {
    move_validated_output_with_progress(source, destination, |_, _| {})
}

// AI-FUNC-SUMMARY: Performs move validated output with ui operation; returns operation status or result; side effects: may spawn processes, move files, or write progress.
fn move_validated_output_with_ui(
    source: &Path,
    destination: &Path,
    ui: &mut ProgressUi,
) -> Result<(), String> {
    let started_at = Instant::now();
    move_validated_output_with_progress(source, destination, |copied, total| {
        ui.render_copy_progress(copied, total, started_at);
    })
}

// AI-FUNC-SUMMARY: Performs move validated output with progress operation; returns operation status or result; side effects: may spawn processes, move files, or write progress.
fn move_validated_output_with_progress<F>(
    source: &Path,
    destination: &Path,
    mut on_progress: F,
) -> Result<(), String>
where
    F: FnMut(u64, u64),
{
    match fs::rename(source, destination) {
        Ok(()) => Ok(()),
        Err(rename_err) => {
            let staging = final_commit_temp_path(destination);
            copy_file_to_new_path_with_progress(source, &staging, &mut on_progress).map_err(
                |copy_err| {
                    format!(
                        "rename failed ({rename_err}); copy fallback to {} failed: {copy_err}",
                        staging.to_string_lossy()
                    )
                },
            )?;

            if destination.exists() {
                let _ = fs::remove_file(&staging);
                return Err(format!(
                    "rename failed ({rename_err}); destination appeared during copy fallback: {}",
                    destination.to_string_lossy()
                ));
            }

            if let Err(err) = fs::rename(&staging, destination) {
                let _ = fs::remove_file(&staging);
                return Err(format!(
                    "rename failed ({rename_err}); copied output could not be finalized at {}: {err}",
                    destination.to_string_lossy()
                ));
            }

            let _ = fs::remove_file(source);
            Ok(())
        }
    }
}

#[cfg(test)]
// AI-FUNC-SUMMARY: Performs copy file to new path operation; returns operation status or result; side effects: may spawn processes, move files, or write progress.
fn copy_file_to_new_path(source: &Path, destination: &Path) -> io::Result<u64> {
    copy_file_to_new_path_with_progress(source, destination, |_, _| {})
}

// AI-FUNC-SUMMARY: Performs copy file to new path with progress operation; returns operation status or result; side effects: may spawn processes, move files, or write progress.
fn copy_file_to_new_path_with_progress<F>(
    source: &Path,
    destination: &Path,
    mut on_progress: F,
) -> io::Result<u64>
where
    F: FnMut(u64, u64),
{
    const COPY_BUFFER_SIZE: usize = 1024 * 1024;

    let total = fs::metadata(source)?.len();
    let mut input = fs::File::open(source)?;
    let mut output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)?;
    let mut buffer = vec![0u8; COPY_BUFFER_SIZE];
    let mut copied = 0u64;

    on_progress(copied, total);

    loop {
        let read = input.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        output.write_all(&buffer[..read])?;
        copied += read as u64;
        on_progress(copied, total);
    }

    output.sync_all()?;
    Ok(copied)
}

// AI-FUNC-SUMMARY: Provides final commit temp path behavior; returns the declared result; side effects: see implementation.
fn final_commit_temp_path(destination: &Path) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let stem = destination
        .file_stem()
        .and_then(OsStr::to_str)
        .unwrap_or("video");
    let ext = destination
        .extension()
        .and_then(OsStr::to_str)
        .unwrap_or("mp4");
    let file_name = format!("{TEMP_PREFIX}commit-{stem}-{stamp}.tmp.{ext}");
    destination
        .parent()
        .map(|dir| dir.join(file_name.clone()))
        .unwrap_or_else(|| PathBuf::from(file_name))
}

// AI-FUNC-SUMMARY: Provides metadata repair temp path behavior; returns the declared result; side effects: see implementation.
fn metadata_repair_temp_path(path: &Path) -> PathBuf {
    with_added_suffix(path, ".repairing.mp4")
}

// AI-FUNC-SUMMARY: Provides metadata backup temp path behavior; returns the declared result; side effects: see implementation.
fn metadata_backup_temp_path(path: &Path) -> PathBuf {
    with_added_suffix(path, ".before-repair.mp4")
}

#[derive(Debug, Default, Clone)]
struct ProgressSnapshot {
    out_time_seconds: Option<f64>,
    speed: Option<String>,
    done: bool,
}

impl ProgressSnapshot {
    // AI-FUNC-SUMMARY: Provides apply behavior; returns the declared result; side effects: see implementation.
    fn apply(&mut self, update: ProgressUpdate) {
        match update {
            ProgressUpdate::OutTime(seconds) => self.out_time_seconds = Some(seconds),
            ProgressUpdate::Speed(speed) => self.speed = Some(speed),
            ProgressUpdate::Done => self.done = true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum ProgressUpdate {
    OutTime(f64),
    Speed(String),
    Done,
}

// AI-FUNC-SUMMARY: Parses progress line input; returns parsed values or errors; side effects: none.
fn parse_progress_line(line: &str) -> Option<ProgressUpdate> {
    let (key, value) = line.split_once('=')?;
    match key {
        "out_time_us" | "out_time_ms" => value
            .parse::<f64>()
            .ok()
            .map(|raw| ProgressUpdate::OutTime(raw / 1_000_000.0)),
        "out_time" => parse_hms(value).map(ProgressUpdate::OutTime),
        "speed" => Some(ProgressUpdate::Speed(value.to_string())),
        "progress" if value == "end" => Some(ProgressUpdate::Done),
        _ => None,
    }
}

// AI-FUNC-SUMMARY: Parses hms input; returns parsed values or errors; side effects: none.
fn parse_hms(value: &str) -> Option<f64> {
    let mut parts = value.split(':');
    let hours = parts.next()?.parse::<f64>().ok()?;
    let minutes = parts.next()?.parse::<f64>().ok()?;
    let seconds = parts.next()?.parse::<f64>().ok()?;
    Some(hours * 3600.0 + minutes * 60.0 + seconds)
}

struct ProgressUi<'a> {
    total_files: Option<usize>,
    current_index: usize,
    started_at: Instant,
    file_started_at: Instant,
    prompt_active: Arc<AtomicBool>,
    events: &'a mut dyn EventSink,
    terminal: bool,
    /// The codec this run produces, so labels can name the right quality knob.
    codec: TargetCodec,
    /// Whether this ffmpeg build can measure quality at all.
    quality_available: bool,
    /// The encoder and quality setting of the attempt in progress, kept
    /// so a review record can say how the file was produced.
    last_encoder: String,
    last_quality: String,
}

struct GracefulExit {
    prompt_active: Arc<AtomicBool>,
}

impl GracefulExit {
    // AI-FUNC-SUMMARY:
    // Purpose: Starts the optional stdin listener for graceful shutdown requests.
    // Inputs: Shared cancellation token.
    // Returns: A shared graceful-exit flag handle.
    // Side effects: Spawns a background stdin reader on interactive terminals and writes prompt text to stderr.
    // Notes: The listener only requests a stop after confirmation; it never interrupts the active ffmpeg process.
    fn start(token: CancellationToken) -> Self {
        let prompt_active = Arc::new(AtomicBool::new(false));
        if io::stdin().is_terminal() {
            eprintln!("Type q then Enter to request a graceful stop after the current file.");
            let listener_token = token.clone();
            let listener_prompt_active = Arc::clone(&prompt_active);
            let _ = thread::spawn(move || {
                graceful_exit_input_loop(listener_token, listener_prompt_active)
            });
        }

        Self { prompt_active }
    }

    // AI-FUNC-SUMMARY: Clones the prompt-active flag for progress rendering coordination; returns shared flag handle; side effects: none.
    fn prompt_active_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.prompt_active)
    }

    // AI-FUNC-SUMMARY: Waits for an active quit confirmation prompt to finish; returns when UI output is safe; side effects: may sleep briefly.
    fn wait_until_prompt_inactive(&self) {
        while self.prompt_active.load(Ordering::Relaxed) {
            thread::sleep(Duration::from_millis(50));
        }
    }
}

// AI-FUNC-SUMMARY:
// Purpose: Reads stdin commands and records a confirmed graceful-exit request.
// Inputs: Shared cancellation token and prompt-active flag.
// Returns: None.
// Side effects: Blocks on stdin in a background thread, writes confirmation prompts to stderr, and updates shared flags.
// Notes: Confirmation accepts y/yes; EOF or stdin errors stop the listener without affecting conversion.
fn graceful_exit_input_loop(token: CancellationToken, prompt_active: Arc<AtomicBool>) {
    let stdin = io::stdin();
    let mut stdin = stdin.lock();
    let mut line = String::new();

    loop {
        line.clear();
        match stdin.read_line(&mut line) {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }

        if token.is_stop_requested() {
            break;
        }

        if !line.trim().eq_ignore_ascii_case("q") {
            continue;
        }

        prompt_active.store(true, Ordering::Relaxed);
        eprint!("\nStop after the current file? Type y then Enter to confirm: ");
        let _ = io::stderr().flush();

        line.clear();
        let read_result = stdin.read_line(&mut line);
        prompt_active.store(false, Ordering::Relaxed);
        match read_result {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }

        let answer = line.trim();
        if answer.eq_ignore_ascii_case("y") || answer.eq_ignore_ascii_case("yes") {
            token.request_stop();
            eprintln!("Graceful exit requested; MyVidComp will stop after the current file.");
            break;
        }

        eprintln!("Graceful exit cancelled.");
    }
}

impl<'a> ProgressUi<'a> {
    // AI-FUNC-SUMMARY: Constructs the progress UI state; returns a new instance; side effects: none.
    fn new(
        total_files: Option<usize>,
        prompt_active: Arc<AtomicBool>,
        events: &'a mut dyn EventSink,
        terminal: bool,
    ) -> Self {
        let now = Instant::now();
        Self {
            total_files,
            current_index: 0,
            started_at: now,
            file_started_at: now,
            prompt_active,
            events,
            terminal,
            codec: TargetCodec::Av1,
            quality_available: false,
            last_encoder: String::new(),
            last_quality: String::new(),
        }
    }

    // AI-FUNC-SUMMARY: Records the codec this run produces; returns none; side effects: updates progress state.
    fn set_codec(&mut self, codec: TargetCodec) {
        self.codec = codec;
    }
    // AI-FUNC-SUMMARY: Records whether quality can be measured in this run; returns none; side effects: updates progress state.
    fn set_quality_available(&mut self, available: bool) {
        self.quality_available = available;
    }

    // AI-FUNC-SUMMARY: Reports a finished quality measurement; returns none; side effects: emits an event and prints a terminal line.
    fn emit_quality_measured(
        &mut self,
        score: u32,
        subsample: u32,
        target: u32,
        passed: bool,
        caveat: Option<String>,
    ) {
        self.events.on_event(Event::QualityMeasured {
            index: self.current_index,
            score,
            subsample,
            target,
            passed,
            caveat: caveat.clone(),
        });
        if !self.terminal {
            return;
        }

        let note = caveat
            .map(|caveat| format!(" ({caveat})"))
            .unwrap_or_default();
        eprintln!(
            "  quality {} of 100, target {}{note}",
            format_quality(score),
            format_quality(target)
        );
    }

    // AI-FUNC-SUMMARY: Reports one step of a quality search; returns none; side effects: emits an event and prints a terminal line.
    fn emit_quality_search(
        &mut self,
        iteration: usize,
        encoder: &str,
        quality: &str,
        score: Option<u32>,
    ) {
        self.events.on_event(Event::QualitySearch {
            index: self.current_index,
            iteration,
            encoder: encoder.to_string(),
            quality: quality.to_string(),
            score,
        });
        if !self.terminal {
            return;
        }

        let score = score
            .map(format_quality)
            .unwrap_or_else(|| "not measured".to_string());
        eprintln!("  trying {quality}: quality {score}");
    }

    // AI-FUNC-SUMMARY: Reports that a conversion is waiting for a keep-or-discard decision; returns none; side effects: emits an event and prints a terminal summary.
    fn emit_review_pending(&mut self, item: &ReviewItem) {
        self.events.on_event(Event::ReviewPending {
            index: self.current_index,
            original_path: item.original_path.clone(),
            review_path: item.review_path.clone(),
            sidecar_path: item.sidecar_path.clone(),
            score: item.score,
            original_bytes: item.original_bytes,
            review_bytes: item.review_bytes,
            deviations: item
                .deviations
                .iter()
                .map(|deviation| format!("{}: {}", deviation.kind, deviation.detail))
                .collect(),
        });
        if !self.terminal {
            return;
        }

        eprintln!("  kept both files for you to compare:");
        eprintln!("    {}", review::review_summary_line(item));
        for deviation in &item.deviations {
            eprintln!("    - {}", deviation.detail);
        }
    }

    // AI-FUNC-SUMMARY: Reports a capability this ffmpeg build lacks; returns none; side effects: emits an event and prints a warning.
    fn emit_capability_missing(&mut self, capability: &str, detail: &str) {
        self.events.on_event(Event::CapabilityMissing {
            capability: capability.to_string(),
            detail: detail.to_string(),
        });
        if self.terminal {
            eprintln!("Note: {detail}");
        }
    }

    // AI-FUNC-SUMMARY: Checks whether quit confirmation currently owns the terminal; returns true while progress rendering should pause; side effects: none.
    fn prompt_active(&self) -> bool {
        self.prompt_active.load(Ordering::Relaxed)
    }

    // AI-FUNC-SUMMARY: Waits until quit confirmation no longer owns the terminal; returns once structured UI output can safely print; side effects: may sleep briefly.
    fn wait_until_prompt_inactive(&self) {
        while self.prompt_active() {
            thread::sleep(Duration::from_millis(50));
        }
    }

    // AI-FUNC-SUMMARY: Provides start file behavior; returns the declared result; side effects: see implementation.
    fn start_file(&mut self, index: usize, item: &WorkItem, encoder: &Encoder) {
        self.wait_until_prompt_inactive();
        self.current_index = index;
        self.file_started_at = Instant::now();
        let quality = item.quality_label(encoder, TargetCodec::Av1);
        self.events.on_event(Event::FileStarted {
            index: self.current_index,
            total: self.total_files,
            input_path: item.input_path.clone(),
            output_path: item.output_path.clone(),
            encoder: encoder.name.clone(),
            quality: quality.clone(),
        });
        if !self.terminal {
            return;
        }
        eprintln!(
            "[{}/{}] {} -> {} using {} (quality {})",
            self.current_index,
            self.total_label(),
            item.input_path.to_string_lossy(),
            item.output_path.to_string_lossy(),
            encoder.name,
            quality
        );
    }

    // AI-FUNC-SUMMARY: Starts one encoder attempt for the active file; returns none; side effects: resets timing, emits current-encoder and attempt events, and may print fallback context.
    fn start_attempt(
        &mut self,
        item: &WorkItem,
        plan: &TranscodePlan<'_>,
        attempt: usize,
        total_attempts: usize,
        fallback_reason: Option<&str>,
    ) {
        let encoder = plan.encoder;
        let plan_kind = plan.kind;
        self.wait_until_prompt_inactive();
        self.file_started_at = Instant::now();
        let quality = plan.quality.label(encoder.kind);
        self.last_encoder = encoder.name.clone();
        self.last_quality = quality.clone();
        self.events.on_event(Event::EncoderSelected {
            encoder: encoder.name.clone(),
        });
        self.events.on_event(Event::FileAttempt {
            index: self.current_index,
            attempt,
            total_attempts,
            encoder: encoder.name.clone(),
            quality: quality.clone(),
            plan: plan_kind.label().to_string(),
            target_pix_fmt: plan_kind.output_pix_fmt().map(str::to_string),
            fallback_reason: fallback_reason.map(str::to_string),
        });
        if !self.terminal {
            return;
        }

        if let Some(reason) = fallback_reason {
            eprintln!(
                "[{}/{}] retrying {} with {} (attempt {attempt}/{total_attempts}, quality {quality}, plan: {}) after: {reason}",
                self.current_index,
                self.total_label(),
                item.input_path.to_string_lossy(),
                encoder.name,
                plan_kind.description(),
            );
        }
    }

    // AI-FUNC-SUMMARY: Provides render file progress behavior; returns the declared result; side effects: see implementation.
    fn render_file_progress(
        &mut self,
        item: &WorkItem,
        encoder: &Encoder,
        snapshot: &ProgressSnapshot,
    ) {
        if self.prompt_active() {
            return;
        }

        let percent = snapshot
            .out_time_seconds
            .zip(nonzero(item.video.duration_seconds))
            .map(|(out, duration)| (out / duration * 100.0).clamp(0.0, 100.0))
            .unwrap_or(0.0);
        let eta = estimate_eta(percent, self.file_started_at.elapsed());
        let speed = snapshot.speed.clone();
        self.events.on_event(Event::FileProgress {
            index: self.current_index,
            percent,
            speed: speed.clone(),
            eta_seconds: eta.as_secs(),
        });
        if !self.terminal {
            return;
        }

        let bar = progress_bar(percent);
        let speed = speed.as_deref().unwrap_or("?");

        eprint!(
            "\r[{}/{}] {} {:>5.1}% speed={} eta={} encoder={} quality={}    ",
            self.current_index,
            self.total_label(),
            bar,
            percent,
            speed,
            format_duration(eta),
            encoder.name,
            item.quality_label(encoder, TargetCodec::Av1)
        );
        let _ = io::stderr().flush();
    }

    // AI-FUNC-SUMMARY: Provides render copy progress behavior; returns the declared result; side effects: see implementation.
    fn render_copy_progress(&mut self, copied: u64, total: u64, started_at: Instant) {
        if self.prompt_active() {
            return;
        }

        let percent = if total > 0 {
            copied as f64 / total as f64 * 100.0
        } else {
            100.0
        }
        .clamp(0.0, 100.0);
        let bar = progress_bar(percent);
        let elapsed = started_at.elapsed();
        let speed = copy_speed_label(copied, elapsed);
        let eta = estimate_eta(percent, elapsed);
        self.events.on_event(Event::CopyProgress {
            index: self.current_index,
            percent,
            copied_bytes: copied,
            total_bytes: total,
            eta_seconds: eta.as_secs(),
        });
        if !self.terminal {
            return;
        }

        eprint!(
            "\r[{}/{}] {} {:>5.1}% copy={}/{} speed={} eta={}    ",
            self.current_index,
            self.total_label(),
            bar,
            percent,
            format_bytes(copied),
            format_bytes(total),
            speed,
            format_duration(eta),
        );
        let _ = io::stderr().flush();
    }

    // AI-FUNC-SUMMARY: Provides render stage behavior; returns the declared result; side effects: see implementation.
    fn render_stage(&mut self, stage: &str) {
        if self.prompt_active() {
            return;
        }

        self.events.on_event(Event::Stage {
            index: self.current_index,
            stage: stage.to_string(),
        });
        if !self.terminal {
            return;
        }

        eprintln!(
            "\n[{}/{}] {}{}",
            self.current_index,
            self.total_label(),
            stage,
            " ".repeat(40)
        );
        let _ = io::stderr().flush();
    }

    // AI-FUNC-SUMMARY: Provides finish file behavior; returns the declared result; side effects: see implementation.
    fn finish_file(&mut self, status: &str) {
        self.wait_until_prompt_inactive();
        if !self.terminal {
            return;
        }
        eprintln!(
            "\r[{}/{}] {} in {} after {} total{}",
            self.current_index,
            self.total_label(),
            status,
            format_duration(self.file_started_at.elapsed()),
            format_duration(self.started_at.elapsed()),
            " ".repeat(20)
        );
    }

    // AI-FUNC-SUMMARY: Emits a file-skipped event; returns none; side effects: sends a structured event.
    fn emit_file_skipped(&mut self, path: &Path, reason: &str) {
        self.events.on_event(Event::FileSkipped {
            path: path.to_path_buf(),
            reason: reason.to_string(),
        });
    }

    // AI-FUNC-SUMMARY: Emits a file-finished event; returns none; side effects: sends a structured event.
    fn emit_file_finished(
        &mut self,
        input_path: &Path,
        status: FileStatus,
        message: &str,
        result: Option<ConversionResult>,
    ) {
        let (source_bytes, output_bytes) = result
            .map(|result| (Some(result.source_bytes), Some(result.output_bytes)))
            .unwrap_or((None, None));
        self.events.on_event(Event::FileFinished {
            index: self.current_index,
            input_path: input_path.to_path_buf(),
            status,
            message: message.to_string(),
            source_bytes,
            output_bytes,
        });
    }

    // AI-FUNC-SUMMARY: Emits a stop-requested event; returns none; side effects: sends a structured event.
    fn emit_stop_requested(&mut self) {
        self.events.on_event(Event::StopRequested);
    }

    // AI-FUNC-SUMMARY: Emits the final summary event; returns none; side effects: sends a structured event.
    fn emit_summary(&mut self, summary: &RunSummary) {
        self.events.on_event(Event::Summary {
            summary: summary.clone(),
        });
    }

    // AI-FUNC-SUMMARY: Emits a log event without terminal rendering; returns none; side effects: sends a structured event.
    fn emit_log(&mut self, level: LogLevel, message: String) {
        self.events.on_event(Event::Log { level, message });
    }

    // AI-FUNC-SUMMARY: Emits an informational log and optional terminal line; returns none; side effects: may write stderr and sends a structured event.
    fn log_info(&mut self, message: String) {
        self.log(LogLevel::Info, message);
    }

    // AI-FUNC-SUMMARY: Emits a warning log and optional terminal line; returns none; side effects: may write stderr and sends a structured event.
    fn log_warning(&mut self, message: String) {
        self.log(LogLevel::Warning, message);
    }

    // AI-FUNC-SUMMARY: Emits an error log and optional terminal line; returns none; side effects: may write stderr and sends a structured event.
    fn log_error(&mut self, message: String) {
        self.log(LogLevel::Error, message);
    }

    // AI-FUNC-SUMMARY: Emits a log message to terminal mode and the structured event sink; returns none; side effects: may write stderr and sends a structured event.
    fn log(&mut self, level: LogLevel, message: String) {
        if self.terminal {
            eprintln!("{message}");
        }
        self.emit_log(level, message);
    }

    // AI-FUNC-SUMMARY: Provides total label behavior; returns the declared result; side effects: see implementation.
    fn total_label(&self) -> String {
        self.total_files
            .map(|total| total.to_string())
            .unwrap_or_else(|| "?".to_string())
    }
}

impl WorkItem {
    // AI-FUNC-SUMMARY: Provides quality label behavior; returns the declared result; side effects: see implementation.
    fn quality_label(&self, encoder: &Encoder, codec: TargetCodec) -> String {
        default_encoder_quality(self, encoder, codec).label(encoder.kind)
    }
}

// AI-FUNC-SUMMARY: Formats one conversion's before/after byte counts; returns a concise size change label; side effects: none.
fn conversion_size_label(result: ConversionResult) -> String {
    format!(
        "{} -> {} ({})",
        format_bytes(result.source_bytes),
        format_bytes(result.output_bytes),
        size_change_label(result.source_bytes, result.output_bytes)
    )
}

// AI-FUNC-SUMMARY: Provides print summary behavior; returns the declared result; side effects: see implementation.
fn print_summary(summary: &RunSummary) {
    println!(
        "Done. converted={} skipped={} failed={}",
        summary.converted, summary.skipped, summary.failed
    );
    if summary.converted > 0 {
        println!(
            "Size: {} -> {} ({})",
            format_bytes(summary.source_bytes),
            format_bytes(summary.output_bytes),
            size_change_label(summary.source_bytes, summary.output_bytes)
        );
    }
}

// AI-FUNC-SUMMARY: Provides print dry run behavior; returns the declared result; side effects: see implementation.
fn print_dry_run(candidates: &[CandidateItem], skipped: usize, encoder: &Encoder, count: isize) {
    println!("Dry run. Encoder: {}", encoder.name);
    if count >= 0 {
        println!("Conversion limit: {count}");
    }
    println!("Candidate video files: {}", candidates.len());
    for candidate in candidates {
        println!(
            "  - {} -> {} (probe at conversion time)",
            candidate.input_path.to_string_lossy(),
            candidate.output_path().to_string_lossy()
        );
    }
    println!("Skipped videos: {skipped}");
}

const MAX_BMFF_BOXES: usize = 100_000;

#[derive(Debug, Clone, Copy)]
struct BmffBox {
    kind: [u8; 4],
    payload_start: u64,
    end: u64,
}

// AI-FUNC-SUMMARY:
// Purpose: Determines mapped streams and chapter behavior without guessing that arbitrary text/data tracks are disposable.
// Inputs: Source path, probed streams/chapters, and source duration metadata.
// Returns: Filtered mapped streams plus ordinary, empty-carrier, or meaningful-carrier chapter policy.
// Side effects: Reads bounded ISO BMFF atom headers and small chapter-reference metadata from the source file when a carrier candidate exists.
// Notes: Only data/bin_data streams tagged with FourCC text, a numeric track ID, and an explicit tref/chap target are removed.
fn source_stream_policy(
    path: &Path,
    streams: Vec<StreamInfo>,
    chapters: Vec<ChapterInfo>,
    video: &VideoInfo,
) -> Result<(Vec<StreamInfo>, ChapterPolicy), String> {
    let candidates = streams
        .iter()
        .filter(|stream| is_chapter_carrier_candidate(stream))
        .collect::<Vec<_>>();
    if candidates.is_empty() {
        return Ok((streams, ChapterPolicy::Ordinary));
    }

    let targets = parse_bmff_chapter_targets(path).map_err(|err| {
        format!(
            "cannot safely classify possible chapter carrier: malformed or ambiguous ISO BMFF atoms: {err}"
        )
    })?;
    let confirmed = candidates
        .iter()
        .filter(|stream| stream.track_id.is_some_and(|id| targets.contains(&id)))
        .collect::<Vec<_>>();
    if confirmed.len() > 1 {
        return Err(format!(
            "cannot safely classify chapter carrier: {} text data tracks are targeted by tref/chap",
            confirmed.len()
        ));
    }
    let Some(carrier) = confirmed.first() else {
        return Ok((streams, ChapterPolicy::Ordinary));
    };
    let carrier_index = carrier.index;
    let filtered = streams
        .into_iter()
        .filter(|stream| stream.index != carrier_index)
        .collect();
    let chapter_policy = if chapters_are_placeholder_or_empty(&chapters, video.duration_seconds) {
        ChapterPolicy::ConfirmedEmpty
    } else {
        ChapterPolicy::ConfirmedMeaningful(chapters)
    };
    Ok((filtered, chapter_policy))
}

// AI-FUNC-SUMMARY: Checks whether a stream has all ffprobe-side traits required of a possible QuickTime chapter carrier; returns true only for data/bin_data FourCC text streams with a numeric track ID; side effects: none.
fn is_chapter_carrier_candidate(stream: &StreamInfo) -> bool {
    stream.codec_type == "data"
        && stream.codec_name == "bin_data"
        && stream
            .codec_tag_string
            .as_deref()
            .is_some_and(|tag| tag.eq_ignore_ascii_case("text"))
        && stream.track_id.is_some()
}

// AI-FUNC-SUMMARY: Classifies absent chapters or one unnamed full-duration chapter as an empty placeholder; returns true within practical muxer timestamp tolerance; side effects: none.
fn chapters_are_placeholder_or_empty(chapters: &[ChapterInfo], duration_seconds: f64) -> bool {
    if chapters.is_empty() {
        return true;
    }
    if chapters.len() != 1 || duration_seconds <= 0.0 {
        return false;
    }
    let chapter = &chapters[0];
    let end_tolerance = (duration_seconds * 0.001).max(0.1);
    chapter.title.trim().is_empty()
        && chapter.start_seconds.abs() <= 0.05
        && (chapter.end_seconds - duration_seconds).abs() <= end_tolerance
}

// AI-FUNC-SUMMARY:
// Purpose: Reads ISO BMFF structure to collect track IDs explicitly targeted by tref/chap references.
// Inputs: Path to a possible ISO BMFF source.
// Returns: Unique referenced track IDs, or a bounded structural/read error.
// Side effects: Opens and seeks through the source; reads box headers and small tkhd/chap payload fields only, never mdat payload.
fn parse_bmff_chapter_targets(path: &Path) -> Result<HashSet<u32>, String> {
    let mut file = fs::File::open(path)
        .map_err(|err| format!("failed to open {}: {err}", path.to_string_lossy()))?;
    let length = file
        .metadata()
        .map_err(|err| format!("failed to stat {}: {err}", path.to_string_lossy()))?
        .len();
    let mut budget = MAX_BMFF_BOXES;
    let top = bmff_children(&mut file, 0, length, &mut budget)?;
    let moov = top
        .iter()
        .filter(|atom| atom.kind == *b"moov")
        .collect::<Vec<_>>();
    if moov.len() != 1 {
        return Err(format!(
            "expected exactly one moov box, found {}",
            moov.len()
        ));
    }
    let mut targets = HashSet::new();
    for trak in bmff_children(&mut file, moov[0].payload_start, moov[0].end, &mut budget)?
        .into_iter()
        .filter(|atom| atom.kind == *b"trak")
    {
        let children = bmff_children(&mut file, trak.payload_start, trak.end, &mut budget)?;
        let tkhd_count = children.iter().filter(|atom| atom.kind == *b"tkhd").count();
        if tkhd_count > 1 {
            return Err("trak contains multiple tkhd boxes".to_string());
        }
        if let Some(tkhd) = children.iter().find(|atom| atom.kind == *b"tkhd") {
            let _ = read_tkhd_track_id(&mut file, *tkhd)?;
        }
        for tref in children.iter().filter(|atom| atom.kind == *b"tref") {
            for chap in bmff_children(&mut file, tref.payload_start, tref.end, &mut budget)?
                .into_iter()
                .filter(|atom| atom.kind == *b"chap")
            {
                let payload_len = chap.end - chap.payload_start;
                if payload_len == 0 || payload_len % 4 != 0 || payload_len > 4096 {
                    return Err(format!("invalid tref/chap payload length {payload_len}"));
                }
                file.seek(SeekFrom::Start(chap.payload_start))
                    .map_err(|err| format!("failed to seek tref/chap: {err}"))?;
                for _ in 0..payload_len / 4 {
                    let id = read_be_u32(&mut file)?;
                    if id == 0 {
                        return Err("tref/chap contains zero track ID".to_string());
                    }
                    targets.insert(id);
                }
            }
        }
    }
    Ok(targets)
}

// AI-FUNC-SUMMARY: Enumerates direct child boxes inside a bounded parent range; returns validated headers or a structural/read error; side effects: seeks and reads box headers only.
fn bmff_children(
    reader: &mut (impl Read + Seek),
    start: u64,
    end: u64,
    budget: &mut usize,
) -> Result<Vec<BmffBox>, String> {
    let mut offset = start;
    let mut boxes = Vec::new();
    while offset < end {
        if *budget == 0 {
            return Err(format!("box count exceeds {MAX_BMFF_BOXES}"));
        }
        *budget -= 1;
        if end - offset < 8 {
            return Err(format!("truncated box header at offset {offset}"));
        }
        reader
            .seek(SeekFrom::Start(offset))
            .map_err(|err| format!("failed to seek box at {offset}: {err}"))?;
        let size32 = read_be_u32(reader)? as u64;
        let mut kind = [0_u8; 4];
        reader
            .read_exact(&mut kind)
            .map_err(|err| format!("failed to read box type at {offset}: {err}"))?;
        let (header_size, box_end) = match size32 {
            0 => (8, end),
            1 => {
                let size64 = read_be_u64(reader)?;
                if size64 < 16 {
                    return Err(format!("invalid extended box size {size64} at {offset}"));
                }
                let box_end = offset
                    .checked_add(size64)
                    .ok_or_else(|| format!("box size overflow at {offset}"))?;
                (16, box_end)
            }
            size if size >= 8 => {
                let box_end = offset
                    .checked_add(size)
                    .ok_or_else(|| format!("box size overflow at {offset}"))?;
                (8, box_end)
            }
            size => return Err(format!("invalid box size {size} at {offset}")),
        };
        if box_end > end {
            return Err(format!("box at {offset} exceeds parent bound {end}"));
        }
        boxes.push(BmffBox {
            kind,
            payload_start: offset + header_size,
            end: box_end,
        });
        offset = box_end;
    }
    Ok(boxes)
}

// AI-FUNC-SUMMARY: Reads a version 0 or 1 tkhd track ID within its validated box bounds; returns a positive track ID or a structural/read error; side effects: seeks and reads at most 24 payload bytes.
fn read_tkhd_track_id(reader: &mut (impl Read + Seek), atom: BmffBox) -> Result<u32, String> {
    reader
        .seek(SeekFrom::Start(atom.payload_start))
        .map_err(|err| format!("failed to seek tkhd: {err}"))?;
    let mut version = [0_u8; 1];
    reader
        .read_exact(&mut version)
        .map_err(|err| format!("failed to read tkhd version: {err}"))?;
    let id_offset = match version[0] {
        0 => 12,
        1 => 20,
        version => return Err(format!("unsupported tkhd version {version}")),
    };
    let id_position = atom
        .payload_start
        .checked_add(id_offset)
        .ok_or_else(|| "tkhd offset overflow".to_string())?;
    if id_position + 4 > atom.end {
        return Err("truncated tkhd track ID".to_string());
    }
    reader
        .seek(SeekFrom::Start(id_position))
        .map_err(|err| format!("failed to seek tkhd track ID: {err}"))?;
    let id = read_be_u32(reader)?;
    (id != 0)
        .then_some(id)
        .ok_or_else(|| "tkhd contains zero track ID".to_string())
}

// AI-FUNC-SUMMARY: Reads one big-endian u32 from a bounded parser position; returns the value or a concise read error; side effects: advances the reader by four bytes.
fn read_be_u32(reader: &mut impl Read) -> Result<u32, String> {
    let mut bytes = [0_u8; 4];
    reader
        .read_exact(&mut bytes)
        .map_err(|err| format!("failed to read 32-bit atom field: {err}"))?;
    Ok(u32::from_be_bytes(bytes))
}

// AI-FUNC-SUMMARY: Reads one big-endian u64 from a bounded parser position; returns the value or a concise read error; side effects: advances the reader by eight bytes.
fn read_be_u64(reader: &mut impl Read) -> Result<u64, String> {
    let mut bytes = [0_u8; 8];
    reader
        .read_exact(&mut bytes)
        .map_err(|err| format!("failed to read 64-bit atom field: {err}"))?;
    Ok(u64::from_be_bytes(bytes))
}

// AI-FUNC-SUMMARY: Selects MP4 or conservative Matroska fallback after carrier filtering; returns a container or detailed unsupported-stream reason; side effects: none.
fn select_output_container(
    output_format: OutputFormat,
    streams: &[StreamInfo],
) -> Result<OutputContainer, String> {
    let Some(mp4_reason) = mp4_incompatible_stream_reason(streams) else {
        return Ok(OutputContainer::Mp4);
    };
    if output_format == OutputFormat::Mp4 {
        return Err(mp4_reason);
    }
    if let Some(reason) = matroska_incompatible_stream_reason(streams) {
        return Err(format!(
            "{mp4_reason}; MKV fallback is also unsafe: {reason}"
        ));
    }
    Ok(OutputContainer::Mkv)
}

// AI-FUNC-SUMMARY: Finds a mapped non-primary stream type not conservatively copy-compatible with Matroska; returns actual index/type/codec detail or none; side effects: none.
fn matroska_incompatible_stream_reason(streams: &[StreamInfo]) -> Option<String> {
    let mut seen_primary_video = false;
    for stream in streams {
        if stream.codec_type == "video" && !seen_primary_video {
            seen_primary_video = true;
            continue;
        }
        if !matches!(stream.codec_type.as_str(), "video" | "audio" | "subtitle") {
            return Some(format!(
                "stream #{} ({}) codec '{}' is not conservatively Matroska-copy-compatible",
                stream.index, stream.codec_type, stream.codec_name
            ));
        }
    }
    None
}

// AI-FUNC-SUMMARY: Finds the first non-primary stream that cannot be copied into an MP4 container; returns a detail message or none; side effects: none.
fn mp4_incompatible_stream_reason(streams: &[StreamInfo]) -> Option<String> {
    let mut seen_primary_video = false;
    for stream in streams {
        if stream.codec_type == "video" && !seen_primary_video {
            seen_primary_video = true;
            continue;
        }

        if !is_mp4_copy_compatible_stream(stream) {
            let handler = stream
                .handler_name
                .as_deref()
                .map(|name| format!(", handler '{name}'"))
                .unwrap_or_default();
            return Some(format!(
                "stream #{} ({}) codec '{}'{} is not MP4-copy-compatible",
                stream.index, stream.codec_type, stream.codec_name, handler
            ));
        }
    }
    None
}

// AI-FUNC-SUMMARY: Checks whether a non-primary stream can be copied into the MP4 target; returns true when the stream type/codec is supported; side effects: none.
fn is_mp4_copy_compatible_stream(stream: &StreamInfo) -> bool {
    match stream.codec_type.as_str() {
        "video" => is_mp4_video_codec(&stream.codec_name),
        "audio" => is_mp4_audio_codec(&stream.codec_name),
        "subtitle" => is_mp4_subtitle_codec(&stream.codec_name),
        _ => false,
    }
}

// AI-FUNC-SUMMARY: Checks whether a copied non-primary video codec is MP4-compatible; returns a boolean; side effects: none.
fn is_mp4_video_codec(codec: &str) -> bool {
    matches!(
        codec,
        "av1" | "h264" | "hevc" | "mpeg4" | "mjpeg" | "jpeg2000"
    )
}

// AI-FUNC-SUMMARY: Checks is mp4 audio codec predicate; returns a boolean; side effects: none.
fn is_mp4_audio_codec(codec: &str) -> bool {
    matches!(
        codec,
        "aac" | "mp3" | "alac" | "ac3" | "eac3" | "flac" | "opus"
    )
}

// AI-FUNC-SUMMARY: Checks whether a copied subtitle codec is MP4-compatible; returns a boolean; side effects: none.
fn is_mp4_subtitle_codec(codec: &str) -> bool {
    matches!(codec, "mov_text")
}

// AI-FUNC-SUMMARY: Returns a skip reason when the source is explicitly interlaced; returns none for progressive or unknown field order; side effects: none.
fn interlaced_source_reason(video: &VideoInfo) -> Option<String> {
    match video.field_order.as_deref() {
        None | Some("progressive") => None,
        Some(field_order) => Some(format!(
            "interlaced source (field_order={field_order}); MyVidComp does not deinterlace"
        )),
    }
}

// AI-FUNC-SUMMARY: Builds or derives estimate av1 crf data; returns the computed value; side effects: none.
fn estimate_av1_crf(video: &VideoInfo) -> u8 {
    let base = if let Some(tag) = &video.encoder_tag {
        if let Some(crf) = extract_named_number(tag, "crf") {
            map_source_quantizer_to_av1(video, crf)
        } else if let Some(qp) = extract_named_number(tag, "qp") {
            map_source_quantizer_to_av1(video, qp)
        } else {
            estimate_av1_crf_from_bitrate(video)
        }
    } else {
        estimate_av1_crf_from_bitrate(video)
    };

    apply_quality_preservation_adjustment(video, base)
}

// AI-FUNC-SUMMARY: Builds or derives estimate av1 crf from bitrate data; returns the computed value; side effects: none.
fn estimate_av1_crf_from_bitrate(video: &VideoInfo) -> u8 {
    let Some(bit_rate) = video.bit_rate else {
        return 24;
    };
    let pixels = u64::from(video.width).saturating_mul(u64::from(video.height));
    if pixels == 0 || video.fps <= 0.0 {
        return 24;
    }

    let bpppf = bit_rate as f64 / (pixels as f64 * video.fps);
    let normalized = bpppf * codec_efficiency_factor(&video.codec_name);
    let depth_bonus = if video.bits_per_raw_sample.unwrap_or(8) >= 10 {
        0.012
    } else {
        0.0
    };
    let score = normalized + depth_bonus;

    if score >= 0.18 {
        18
    } else if score >= 0.12 {
        20
    } else if score >= 0.08 {
        23
    } else if score >= 0.05 {
        26
    } else if score >= 0.03 {
        30
    } else {
        34
    }
}

// AI-FUNC-SUMMARY: Provides apply quality preservation adjustment behavior; returns the declared result; side effects: see implementation.
fn apply_quality_preservation_adjustment(video: &VideoInfo, crf: u8) -> u8 {
    let mut adjustment = 0u8;

    if video
        .profile
        .as_deref()
        .is_some_and(|profile| contains_any_ignore_case(profile, &["high", "main 10", "main10"]))
    {
        adjustment += 1;
    }

    if video
        .pix_fmt
        .as_deref()
        .is_some_and(|pix_fmt| contains_any_ignore_case(pix_fmt, &["10", "12", "422", "444"]))
    {
        adjustment += 1;
    }

    if video.color_transfer.as_deref().is_some_and(|transfer| {
        contains_any_ignore_case(transfer, &["smpte2084", "arib-std-b67", "iec61966"])
    }) {
        adjustment += 2;
    }

    if video.color_space.is_some() && video.color_primaries.is_some() {
        adjustment += 1;
    }

    crf.saturating_sub(adjustment).clamp(12, 36)
}

// AI-FUNC-SUMMARY: Provides contains any ignore case behavior; returns the declared result; side effects: see implementation.
fn contains_any_ignore_case(text: &str, needles: &[&str]) -> bool {
    let lower = text.to_ascii_lowercase();
    needles
        .iter()
        .any(|needle| lower.contains(&needle.to_ascii_lowercase()))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SourceCodecFamily {
    Av1,
    H265,
    H264,
    Vp9,
    Vp8,
    ProRes,
    Mpeg4Part2,
    Mpeg2,
    Mpeg1,
    H263,
    Theora,
    Vc1,
    Mjpeg,
    Dv,
    Other,
}

// AI-FUNC-SUMMARY: Provides source codec family behavior; returns the declared result; side effects: see implementation.
fn source_codec_family(codec: &str) -> SourceCodecFamily {
    let codec = codec.to_ascii_lowercase();
    if contains_any_ignore_case(&codec, &["av1", "av01"]) {
        SourceCodecFamily::Av1
    } else if contains_any_ignore_case(&codec, &["hevc", "h265", "h.265", "hvc1", "hev1"]) {
        SourceCodecFamily::H265
    } else if contains_any_ignore_case(&codec, &["h264", "h.264", "avc", "avc1"]) {
        SourceCodecFamily::H264
    } else if contains_any_ignore_case(&codec, &["vp9", "vp09"]) {
        SourceCodecFamily::Vp9
    } else if contains_any_ignore_case(&codec, &["vp8", "vp08"]) {
        SourceCodecFamily::Vp8
    } else if contains_any_ignore_case(&codec, &["prores", "apch", "apcn", "apcs", "ap4h"]) {
        SourceCodecFamily::ProRes
    } else if contains_any_ignore_case(&codec, &["mpeg4", "mp4v", "xvid", "divx", "msmpeg4"]) {
        SourceCodecFamily::Mpeg4Part2
    } else if contains_any_ignore_case(&codec, &["mpeg2", "mpg2"]) {
        SourceCodecFamily::Mpeg2
    } else if contains_any_ignore_case(&codec, &["mpeg1", "mpg1"]) {
        SourceCodecFamily::Mpeg1
    } else if contains_any_ignore_case(&codec, &["h263", "h.263"]) {
        SourceCodecFamily::H263
    } else if contains_any_ignore_case(&codec, &["theora"]) {
        SourceCodecFamily::Theora
    } else if contains_any_ignore_case(&codec, &["vc1", "vc-1", "wmv3", "wmv2", "wmv1"]) {
        SourceCodecFamily::Vc1
    } else if contains_any_ignore_case(&codec, &["mjpeg", "mjpegb", "jpeg"]) {
        SourceCodecFamily::Mjpeg
    } else if contains_any_ignore_case(&codec, &["dvvideo", "dvc", "dv "]) {
        SourceCodecFamily::Dv
    } else {
        SourceCodecFamily::Other
    }
}

// AI-FUNC-SUMMARY: Builds or derives map source quantizer to av1 data; returns the computed value; side effects: none.
fn map_source_quantizer_to_av1(video: &VideoInfo, quantizer: f64) -> u8 {
    let crf = match source_codec_family(&video.codec_name) {
        SourceCodecFamily::H265 | SourceCodecFamily::Vp9 => quantizer - 2.0,
        SourceCodecFamily::Vp8 | SourceCodecFamily::Mpeg4Part2 => quantizer - 4.0,
        SourceCodecFamily::Mpeg1 | SourceCodecFamily::Mpeg2 | SourceCodecFamily::H263 => {
            quantizer - 6.0
        }
        _ => quantizer,
    };
    crf.round().clamp(12.0, 36.0) as u8
}

// AI-FUNC-SUMMARY: Provides codec efficiency factor behavior; returns the declared result; side effects: see implementation.
fn codec_efficiency_factor(codec: &str) -> f64 {
    match source_codec_family(codec) {
        SourceCodecFamily::H265 => 1.45,
        SourceCodecFamily::Vp9 => 1.35,
        SourceCodecFamily::H264 | SourceCodecFamily::Av1 | SourceCodecFamily::Other => 1.0,
        SourceCodecFamily::Vc1 => 0.9,
        SourceCodecFamily::Vp8 => 0.85,
        SourceCodecFamily::Mpeg4Part2 | SourceCodecFamily::Theora => 0.75,
        SourceCodecFamily::Mpeg1 | SourceCodecFamily::Mpeg2 => 0.60,
        SourceCodecFamily::H263 => 0.55,
        SourceCodecFamily::Mjpeg | SourceCodecFamily::Dv => 0.45,
        SourceCodecFamily::ProRes => 2.2,
    }
}

// AI-FUNC-SUMMARY: Builds or derives estimate target bitrate data; returns the computed value; side effects: none.
fn estimate_target_bitrate(item: &WorkItem, codec: TargetCodec) -> Bitrate {
    let source = item.video.bit_rate.unwrap_or_else(|| {
        let pixels = u64::from(item.video.width).saturating_mul(u64::from(item.video.height));
        let fps = if item.video.fps > 0.0 {
            item.video.fps
        } else {
            30.0
        };
        (pixels as f64 * fps * 0.08) as u64
    });

    // Two adjustments: how much more efficient the target codec is than the
    // source, and how much bitrate this particular codec needs to match AV1.
    let ratio =
        hardware_bitrate_ratio(&item.video.codec_name) * codec::codec_bitrate_multiplier(codec);
    let target = (source as f64 * ratio) as u64;
    Bitrate(target.clamp(600_000, source.max(600_000)))
}

// AI-FUNC-SUMMARY: Provides hardware bitrate ratio behavior; returns the declared result; side effects: see implementation.
fn hardware_bitrate_ratio(codec: &str) -> f64 {
    match source_codec_family(codec) {
        SourceCodecFamily::H265 | SourceCodecFamily::Vp9 => 0.85,
        SourceCodecFamily::H264 | SourceCodecFamily::Vc1 | SourceCodecFamily::Other => 0.70,
        SourceCodecFamily::Vp8 | SourceCodecFamily::Mpeg4Part2 | SourceCodecFamily::Theora => 0.60,
        SourceCodecFamily::Mpeg1 | SourceCodecFamily::Mpeg2 | SourceCodecFamily::H263 => 0.50,
        SourceCodecFamily::Mjpeg | SourceCodecFamily::Dv | SourceCodecFamily::ProRes => 0.35,
        SourceCodecFamily::Av1 => 1.0,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Bitrate(u64);

impl std::fmt::Display for Bitrate {
    // AI-FUNC-SUMMARY: Provides fmt behavior; returns the declared result; side effects: see implementation.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}k", self.0.div_ceil(1000))
    }
}

// AI-FUNC-SUMMARY: Provides extract named number behavior; returns the declared result; side effects: see implementation.
fn extract_named_number(text: &str, name: &str) -> Option<f64> {
    let lower = text.to_ascii_lowercase();
    let name = name.to_ascii_lowercase();
    let start = lower.find(&name)?;
    let tail = &lower[start + name.len()..];
    let number_start = tail.char_indices().find(|(_, ch)| ch.is_ascii_digit())?.0;
    let number_tail = &tail[number_start..];
    let number_end = number_tail
        .char_indices()
        .find(|(_, ch)| !ch.is_ascii_digit() && *ch != '.')
        .map(|(index, _)| index)
        .unwrap_or(number_tail.len());
    number_tail[..number_end].parse::<f64>().ok()
}

// AI-FUNC-SUMMARY: Parses ratio input; returns parsed values or errors; side effects: none.
fn parse_ratio(value: &str) -> Option<f64> {
    let (left, right) = value.split_once('/').or_else(|| value.split_once(':'))?;
    let numerator = left.parse::<f64>().ok()?;
    let denominator = right.parse::<f64>().ok()?;
    if denominator == 0.0 {
        None
    } else {
        Some(numerator / denominator)
    }
}

// AI-FUNC-SUMMARY: Parses frame rate input; returns parsed values or errors; side effects: none.
fn parse_frame_rate(value: &str) -> Option<(String, f64)> {
    let fps = parse_ratio(value)?;
    if fps > 0.0 {
        Some((value.to_string(), fps))
    } else {
        None
    }
}

// AI-FUNC-SUMMARY: Provides progress bar behavior; returns the declared result; side effects: see implementation.
fn progress_bar(percent: f64) -> String {
    let width = 24;
    let filled = ((percent / 100.0) * width as f64).round() as usize;
    let filled = filled.min(width);
    format!(
        "[{}{}]",
        "#".repeat(filled),
        "-".repeat(width.saturating_sub(filled))
    )
}

// AI-FUNC-SUMMARY: Builds or derives estimate eta data; returns the computed value; side effects: none.
fn estimate_eta(percent: f64, elapsed: Duration) -> Duration {
    if percent <= 0.0 {
        return Duration::ZERO;
    }
    let total = elapsed.as_secs_f64() / (percent / 100.0);
    let remaining = (total - elapsed.as_secs_f64()).max(0.0);
    Duration::from_secs_f64(remaining)
}

// AI-FUNC-SUMMARY: Builds or derives format duration data; returns the computed value; side effects: none.
fn format_duration(duration: Duration) -> String {
    let seconds = duration.as_secs();
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let seconds = seconds % 60;
    if hours > 0 {
        format!("{hours:02}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes:02}:{seconds:02}")
    }
}

// AI-FUNC-SUMMARY: Builds or derives format bytes data; returns the computed value; side effects: none.
fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];

    let mut value = bytes as f64;
    let mut unit_index = 0;
    while value >= 1024.0 && unit_index + 1 < UNITS.len() {
        value /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{bytes} B")
    } else if value >= 100.0 {
        format!("{value:.0} {}", UNITS[unit_index])
    } else if value >= 10.0 {
        format!("{value:.1} {}", UNITS[unit_index])
    } else {
        format!("{value:.2} {}", UNITS[unit_index])
    }
}

// AI-FUNC-SUMMARY: Provides size change label behavior; returns the declared result; side effects: see implementation.
fn size_change_label(source_bytes: u64, output_bytes: u64) -> String {
    if source_bytes == 0 {
        return "source size was 0 B".to_string();
    }

    if output_bytes < source_bytes {
        let saved = source_bytes - output_bytes;
        let percent = saved as f64 / source_bytes as f64 * 100.0;
        format!("saved {}, {percent:.1}% smaller", format_bytes(saved))
    } else if output_bytes > source_bytes {
        let added = output_bytes - source_bytes;
        let percent = added as f64 / source_bytes as f64 * 100.0;
        format!("grew by {}, {percent:.1}% larger", format_bytes(added))
    } else {
        "unchanged".to_string()
    }
}

// AI-FUNC-SUMMARY: Performs copy speed label operation; returns operation status or result; side effects: may spawn processes, move files, or write progress.
fn copy_speed_label(copied: u64, elapsed: Duration) -> String {
    let seconds = elapsed.as_secs_f64();
    if seconds <= 0.0 {
        return "?".to_string();
    }
    format!("{}/s", format_bytes((copied as f64 / seconds) as u64))
}

// AI-FUNC-SUMMARY: Provides nonzero behavior; returns the declared result; side effects: see implementation.
fn nonzero(value: f64) -> Option<f64> {
    if value > 0.0 { Some(value) } else { None }
}

// AI-FUNC-SUMMARY: Provides with added suffix behavior; returns the declared result; side effects: see implementation.
fn with_added_suffix(path: &Path, suffix: &str) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or("file")
        .to_string();
    path.with_file_name(format!("{file_name}{suffix}"))
}

// AI-FUNC-SUMMARY: Checks has added suffix predicate; returns a boolean; side effects: none.
fn has_added_suffix(path: &Path, suffix: &str) -> bool {
    path.file_name()
        .and_then(OsStr::to_str)
        .is_some_and(|name| name.ends_with(suffix))
}

// AI-FUNC-SUMMARY: Provides temp output path behavior; returns a timestamped MyVidComp temp path with the correct container extension; side effects: none.
fn temp_output_path(input: &Path, tmp_dir: Option<&Path>, container: OutputContainer) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let stem = input.file_stem().and_then(OsStr::to_str).unwrap_or("video");
    let file_name = format!("{TEMP_PREFIX}{stem}-{stamp}.tmp.{}", container.extension());
    tmp_dir
        .map(|dir| dir.join(file_name.clone()))
        .unwrap_or_else(|| input.with_file_name(file_name))
}

// AI-FUNC-SUMMARY: Provides cached temp output paths behavior; returns matching MyVidComp temp files for both containers sorted newest-first; side effects: reads directory entries.
fn cached_temp_output_paths(input: &Path, tmp_dir: Option<&Path>) -> Vec<PathBuf> {
    let dir = tmp_dir
        .map(Path::to_path_buf)
        .or_else(|| input.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."));
    let stem = input.file_stem().and_then(OsStr::to_str).unwrap_or("video");
    let prefixes = [
        format!("{TEMP_PREFIX}{stem}-"),
        format!("{LEGACY_TEMP_PREFIX}{stem}-"),
    ];

    let Ok(entries) = fs::read_dir(&dir) else {
        return Vec::new();
    };

    let mut paths = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            let name = path.file_name()?.to_str()?;
            (prefixes
                .iter()
                .any(|prefix| name.starts_with(prefix.as_str()))
                && (name.ends_with(".tmp.mp4") || name.ends_with(".tmp.mkv")))
            .then_some(path)
        })
        .collect::<Vec<_>>();

    paths.sort_by_key(|path| std::cmp::Reverse(path.file_name().map(OsStr::to_os_string)));
    paths
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    // AI-FUNC-SUMMARY: Parses parses cli defaults input; returns parsed values or errors; side effects: none.
    fn parses_cli_defaults() {
        let root = make_temp_dir("cli-defaults");
        let cli = Cli::parse([root.to_string_lossy().to_string()]).unwrap();

        assert_eq!(cli.count, -1);
        assert!(cli.keep_original);
        assert!(!cli.dry_run);
        assert!(!cli.version);
        assert_eq!(cli.ffmpeg, platform_binary_name("ffmpeg"));
        assert_eq!(cli.ffprobe, platform_binary_name("ffprobe"));
        assert_eq!(cli.tmp_dir, None);
        assert_eq!(cli.encoder, None);
        assert_eq!(cli.encoder_preference, EncoderPreference::Auto);

        remove_temp_dir(root);
    }

    #[test]
    // AI-FUNC-SUMMARY: Parses parses cli options input; returns parsed values or errors; side effects: none.
    fn parses_cli_options() {
        let root = make_temp_dir("cli-options");
        let cli = Cli::parse([
            root.to_string_lossy().to_string(),
            "--count".to_string(),
            "3".to_string(),
            "--no-keep-original".to_string(),
            "--dry-run".to_string(),
            "--ffmpeg".to_string(),
            "/opt/ffmpeg".to_string(),
            "--ffprobe".to_string(),
            "/opt/ffprobe".to_string(),
            "--tmp-dir".to_string(),
            root.join("tmp").to_string_lossy().to_string(),
            "--encoder".to_string(),
            "nvenc".to_string(),
        ])
        .unwrap();

        assert_eq!(cli.count, 3);
        assert!(!cli.keep_original);
        assert!(cli.dry_run);
        assert_eq!(cli.ffmpeg, "/opt/ffmpeg");
        assert_eq!(cli.ffprobe, "/opt/ffprobe");
        assert_eq!(cli.tmp_dir, Some(root.join("tmp")));
        assert_eq!(cli.encoder, Some("nvenc".to_string()));

        remove_temp_dir(root);
    }

    #[test]
    // AI-FUNC-SUMMARY: Parses parses version without target folder input; returns parsed values or errors; side effects: none.
    fn parses_version_without_target_folder() {
        let cli = Cli::parse(["--version".to_string()]).unwrap();

        assert!(cli.version);
        assert!(cli.target_folder.as_os_str().is_empty());
        assert!(version_label().starts_with("myvidcomp "));
        assert!(version_label().contains(env!("CARGO_PKG_VERSION")));
    }

    #[test]
    // AI-FUNC-SUMMARY: Provides reports info when count exceeds eligible videos behavior; returns the declared result; side effects: see implementation.
    fn reports_info_when_count_exceeds_eligible_videos() {
        assert_eq!(
            count_limit_info(10, 3),
            Some(
                "Info: requested 10 conversion(s), only 3 eligible video(s) need conversion."
                    .to_string()
            )
        );
        assert_eq!(count_limit_info(3, 3), None);
        assert_eq!(count_limit_info(-1, 3), None);
    }

    #[test]
    // AI-FUNC-SUMMARY: Provides reports info when count exceeds candidates in dry run behavior; returns the declared result; side effects: see implementation.
    fn reports_info_when_count_exceeds_candidates_in_dry_run() {
        assert_eq!(
            count_candidate_info(10, 3),
            Some(
                "Info: requested 10 conversion(s), only 3 candidate video file(s) were found before probing."
                    .to_string()
            )
        );
        assert_eq!(count_candidate_info(3, 3), None);
        assert_eq!(count_candidate_info(-1, 3), None);
    }

    #[test]
    // AI-FUNC-SUMMARY: Discovers or probes reads config when no cli args data; returns collected metadata; side effects: may read filesystem or subprocess output.
    fn reads_config_when_no_cli_args() {
        let root = make_temp_dir("config-default");
        let videos = root.join("videos");
        fs::create_dir(&videos).unwrap();
        let config_path = root.join("config.yaml");
        fs::write(
            &config_path,
            format!(
                r#"
target_folder: "{}"
count: 2
keep_original: false
dry_run: true
ffmpeg: "/tools/ffmpeg"
ffprobe: "/tools/ffprobe"
tmp_dir: "{}"
encoder: av1_qsv
"#,
                videos.to_string_lossy(),
                root.join("tmp").to_string_lossy()
            ),
        )
        .unwrap();

        let cli =
            Cli::parse_with_default_config(std::iter::empty::<String>(), &config_path).unwrap();

        assert_eq!(cli.target_folder, videos);
        assert_eq!(cli.count, 2);
        assert!(!cli.keep_original);
        assert!(cli.dry_run);
        assert_eq!(cli.ffmpeg, "/tools/ffmpeg");
        assert_eq!(cli.ffprobe, "/tools/ffprobe");
        assert_eq!(cli.tmp_dir, Some(root.join("tmp")));
        assert_eq!(cli.encoder, Some("av1_qsv".to_string()));

        remove_temp_dir(root);
    }

    #[test]
    // AI-FUNC-SUMMARY: Provides cli values override config values behavior; returns the declared result; side effects: see implementation.
    fn cli_values_override_config_values() {
        let root = make_temp_dir("config-override");
        let config_target = root.join("from-config");
        let cli_target = root.join("from-cli");
        fs::create_dir(&config_target).unwrap();
        fs::create_dir(&cli_target).unwrap();
        let config_path = root.join("config.yaml");
        fs::write(
            &config_path,
            format!(
                r#"
target_folder: "{}"
count: 9
keep_original: false
dry_run: true
ffmpeg: "/config/ffmpeg"
ffprobe: "/config/ffprobe"
tmp_dir: "{}"
encoder: av1_amf
"#,
                config_target.to_string_lossy(),
                root.join("config-tmp").to_string_lossy()
            ),
        )
        .unwrap();

        let cli = Cli::parse_with_default_config(
            [
                cli_target.to_string_lossy().to_string(),
                "--count".to_string(),
                "1".to_string(),
                "--keep-original".to_string(),
                "--no-dry-run".to_string(),
                "--ffmpeg".to_string(),
                "/cli/ffmpeg".to_string(),
                "--tmp-dir".to_string(),
                root.join("cli-tmp").to_string_lossy().to_string(),
                "--encoder".to_string(),
                "auto".to_string(),
            ],
            &config_path,
        )
        .unwrap();

        assert_eq!(cli.target_folder, cli_target);
        assert_eq!(cli.count, 1);
        assert!(cli.keep_original);
        assert!(!cli.dry_run);
        assert_eq!(cli.ffmpeg, "/cli/ffmpeg");
        assert_eq!(cli.ffprobe, "/config/ffprobe");
        assert_eq!(cli.tmp_dir, Some(root.join("cli-tmp")));
        assert_eq!(cli.encoder, None);

        remove_temp_dir(root);
    }

    #[test]
    // AI-FUNC-SUMMARY: Provides no args require target in config behavior; returns the declared result; side effects: see implementation.
    fn no_args_require_target_in_config() {
        let root = make_temp_dir("config-missing");
        let config_path = root.join("config.yaml");

        let err =
            Cli::parse_with_default_config(std::iter::empty::<String>(), &config_path).unwrap_err();

        assert!(err.contains("failed to read config"));
        assert!(err.contains("config.yaml"));

        remove_temp_dir(root);
    }

    #[test]
    // AI-FUNC-SUMMARY: Parses parses flat yaml config comments and quotes input; returns parsed values or errors; side effects: none.
    fn parses_flat_yaml_config_comments_and_quotes() {
        let parsed = parse_config_yaml(
            r#"
---
target: "/Volumes/video # archive"
count: -1
keep_original: yes
dry_run: off # preview disabled
ffmpeg: 'C:\Tools\ffmpeg.exe'
ffprobe: C:\Tools\ffprobe.exe
tmp_dir: C:\Temp\myvidcomp
encoder: amf
"#,
        )
        .unwrap();

        assert_eq!(
            parsed.target_folder,
            Some(PathBuf::from("/Volumes/video # archive"))
        );
        assert_eq!(parsed.count, Some(-1));
        assert_eq!(parsed.keep_original, Some(true));
        assert_eq!(parsed.dry_run, Some(false));
        assert_eq!(parsed.ffmpeg, Some("C:\\Tools\\ffmpeg.exe".to_string()));
        assert_eq!(parsed.ffprobe, Some("C:\\Tools\\ffprobe.exe".to_string()));
        assert_eq!(parsed.tmp_dir, Some(PathBuf::from("C:\\Temp\\myvidcomp")));
        assert_eq!(parsed.encoder, Some("amf".to_string()));
    }

    #[test]
    // AI-FUNC-SUMMARY: Provides empty config encoder keeps auto detection behavior; returns the declared result; side effects: see implementation.
    fn empty_config_encoder_keeps_auto_detection() {
        let parsed = parse_config_yaml(
            r#"
target_folder: "/videos"
encoder:
"#,
        )
        .unwrap();

        assert_eq!(parsed.encoder, None);
        assert_eq!(normalize_encoder_choice("default"), None);
        assert_eq!(normalize_encoder_choice("auto"), None);
        assert_eq!(normalize_encoder_choice("qsv"), Some("qsv".to_string()));
        assert_eq!(normalize_encoder_choice("MF"), Some("mf".to_string()));
    }

    #[test]
    // AI-FUNC-SUMMARY: Parses parses windows unc config paths input; returns parsed values or errors; side effects: none.
    fn parses_windows_unc_config_paths() {
        let parsed = parse_config_yaml(
            r#"
target_folder: "\\server\share\videos"
tmp_dir: '\\server\scratch\myvidcomp'
"#,
        )
        .unwrap();

        assert_eq!(
            parsed.target_folder,
            Some(PathBuf::from("\\\\server\\share\\videos"))
        );
        assert_eq!(
            parsed.tmp_dir,
            Some(PathBuf::from("\\\\server\\scratch\\myvidcomp"))
        );
    }

    #[test]
    // AI-FUNC-SUMMARY: Parses parses yaml escaped windows unc config paths input; returns parsed values or errors; side effects: none.
    fn parses_yaml_escaped_windows_unc_config_paths() {
        let parsed = parse_config_yaml(
            r#"
target_folder: "\\\\server\\share\\videos"
"#,
        )
        .unwrap();

        assert_eq!(
            parsed.target_folder,
            Some(PathBuf::from("\\\\server\\share\\videos"))
        );
    }

    #[test]
    #[cfg(not(windows))]
    // AI-FUNC-SUMMARY: Provides uses unix runtime binary names behavior; returns the declared result; side effects: see implementation.
    fn uses_unix_runtime_binary_names() {
        assert_eq!(platform_binary_name("ffmpeg"), "ffmpeg");
        assert_eq!(platform_binary_name("ffprobe"), "ffprobe");
        assert_eq!(bundled_binary_names("ffmpeg"), ["ffmpeg"]);
    }

    #[test]
    #[cfg(windows)]
    // AI-FUNC-SUMMARY: Provides uses windows runtime binary names behavior; returns the declared result; side effects: see implementation.
    fn uses_windows_runtime_binary_names() {
        assert_eq!(platform_binary_name("ffmpeg"), "ffmpeg.exe");
        assert_eq!(platform_binary_name("ffprobe"), "ffprobe.exe");
        assert_eq!(bundled_binary_names("ffmpeg"), ["ffmpeg.exe", "ffmpeg"]);
    }

    #[test]
    // AI-FUNC-SUMMARY: Discovers or probes discovers level order files before descending data; returns collected metadata; side effects: may read filesystem or subprocess output.
    fn discovers_level_order_files_before_descending() {
        let root = make_temp_dir("discover");
        fs::create_dir(root.join("AA")).unwrap();
        fs::create_dir(root.join("BB")).unwrap();
        fs::create_dir(root.join("CC")).unwrap();
        fs::create_dir(root.join("CC").join("A1")).unwrap();
        fs::write(root.join("A.mp4"), b"").unwrap();
        fs::write(root.join("B.mp4"), b"").unwrap();
        fs::write(root.join("C.mp4"), b"").unwrap();
        fs::write(root.join("AA").join("11.mp4"), b"").unwrap();
        fs::write(root.join("BB").join("12.mp4"), b"").unwrap();
        fs::write(root.join("CC").join("A1").join("11.mp4"), b"").unwrap();

        let files = discover_files(&root).unwrap();
        let names: Vec<String> = files
            .iter()
            .map(|path| {
                path.strip_prefix(&root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect();

        assert_eq!(
            names,
            [
                "A.mp4",
                "B.mp4",
                "C.mp4",
                "AA/11.mp4",
                "BB/12.mp4",
                "CC/A1/11.mp4"
            ]
        );

        remove_temp_dir(root);
    }

    #[test]
    // AI-FUNC-SUMMARY: Builds or derives builds output and old paths data; returns the computed value; side effects: none.
    fn builds_output_and_old_paths() {
        let root = make_temp_dir("paths");
        let input = root.join("clip.mkv");
        fs::write(&input, b"").unwrap();
        let video = sample_video("h264", Some(8_000_000));
        let streams = vec![sample_stream(0, "video", "h264")];

        let item = WorkItem::new(
            input.clone(),
            video,
            streams,
            Vec::new(),
            ItemPolicy {
                keep_original: true,
                tmp_dir: None,
                output_format: OutputFormat::Mp4,
                target_codec: TargetCodec::Av1,
                preservation: Preservation::Strict,
            },
        )
        .unwrap();

        assert_eq!(item.output_path, root.join("clip.mp4"));
        assert_eq!(item.old_path, Some(root.join("clip.mkv.old")));

        remove_temp_dir(root);
    }

    #[test]
    // AI-FUNC-SUMMARY: Provides uses configured tmp dir for temp output behavior; returns the declared result; side effects: see implementation.
    fn uses_configured_tmp_dir_for_temp_output() {
        let root = make_temp_dir("tmp-dir");
        let tmp = root.join("local-tmp");
        fs::create_dir(&tmp).unwrap();
        let input = root.join("clip.mkv");
        fs::write(&input, b"").unwrap();
        let video = sample_video("h264", Some(8_000_000));
        let streams = vec![sample_stream(0, "video", "h264")];

        let item = WorkItem::new(
            input,
            video,
            streams,
            Vec::new(),
            ItemPolicy {
                keep_original: true,
                tmp_dir: Some(&tmp),
                output_format: OutputFormat::Mp4,
                target_codec: TargetCodec::Av1,
                preservation: Preservation::Strict,
            },
        )
        .unwrap();

        assert!(item.temp_path.starts_with(&tmp));
        assert_eq!(
            item.temp_path.extension().and_then(OsStr::to_str),
            Some("mp4")
        );

        remove_temp_dir(root);
    }

    #[test]
    // AI-FUNC-SUMMARY: Discovers or probes finds cached temp outputs for same input stem data; returns collected metadata; side effects: may read filesystem or subprocess output.
    fn finds_cached_temp_outputs_for_same_input_stem() {
        let root = make_temp_dir("cached-temp");
        let tmp = root.join("local-tmp");
        fs::create_dir(&tmp).unwrap();
        let input = root.join("clip.mkv");
        let older = tmp.join(".myvidcomp-clip-100.tmp.mp4");
        let newer = tmp.join(".myvidcomp-clip-200.tmp.mp4");
        let other = tmp.join(".myvidcomp-other-300.tmp.mp4");
        let repair = tmp.join(".myvidcomp-clip-200.tmp.mp4.repairing.mp4");
        fs::write(&older, b"older").unwrap();
        fs::write(&newer, b"newer").unwrap();
        fs::write(&other, b"other").unwrap();
        fs::write(&repair, b"repair").unwrap();

        let cached = cached_temp_output_paths(&input, Some(&tmp));

        assert_eq!(cached, vec![newer, older]);
        remove_temp_dir(root);
    }

    #[test]
    // AI-FUNC-SUMMARY: Provides final commit temp path stays next to destination behavior; returns the declared result; side effects: see implementation.
    fn final_commit_temp_path_stays_next_to_destination() {
        let destination = Path::new("/videos/clip.mp4");
        let temp = final_commit_temp_path(destination);

        assert_eq!(temp.parent(), Some(Path::new("/videos")));
        assert!(
            temp.file_name()
                .and_then(OsStr::to_str)
                .is_some_and(|name| name.starts_with(".myvidcomp-commit-clip-"))
        );
    }

    #[test]
    // AI-FUNC-SUMMARY: Performs move validated output moves source to destination operation; returns operation status or result; side effects: may spawn processes, move files, or write progress.
    fn move_validated_output_moves_source_to_destination() {
        let root = make_temp_dir("move-output");
        let source = root.join("source.tmp.mp4");
        let destination = root.join("clip.mp4");
        fs::write(&source, b"converted").unwrap();

        move_validated_output(&source, &destination).unwrap();

        assert!(!source.exists());
        assert_eq!(fs::read(&destination).unwrap(), b"converted");

        remove_temp_dir(root);
    }

    #[test]
    // AI-FUNC-SUMMARY: Performs copy file to new path refuses to overwrite operation; returns operation status or result; side effects: may spawn processes, move files, or write progress.
    fn copy_file_to_new_path_refuses_to_overwrite() {
        let root = make_temp_dir("copy-new");
        let source = root.join("source.tmp.mp4");
        let destination = root.join("existing.mp4");
        fs::write(&source, b"converted").unwrap();
        fs::write(&destination, b"existing").unwrap();

        assert!(copy_file_to_new_path(&source, &destination).is_err());
        assert_eq!(fs::read(&destination).unwrap(), b"existing");

        remove_temp_dir(root);
    }

    #[test]
    // AI-FUNC-SUMMARY: Performs copy file to new path reports progress operation; returns operation status or result; side effects: may spawn processes, move files, or write progress.
    fn copy_file_to_new_path_reports_progress() {
        let root = make_temp_dir("copy-progress");
        let source = root.join("source.tmp.mp4");
        let destination = root.join("destination.tmp.mp4");
        fs::write(&source, vec![7u8; 2 * 1024 * 1024 + 17]).unwrap();
        let mut updates = Vec::new();

        let bytes = copy_file_to_new_path_with_progress(&source, &destination, |copied, total| {
            updates.push((copied, total));
        })
        .unwrap();

        assert_eq!(bytes, 2 * 1024 * 1024 + 17);
        assert_eq!(fs::metadata(&destination).unwrap().len(), bytes);
        assert_eq!(updates.first(), Some(&(0, bytes)));
        assert_eq!(updates.last(), Some(&(bytes, bytes)));
        assert!(updates.len() >= 3);

        remove_temp_dir(root);
    }

    #[test]
    // AI-FUNC-SUMMARY: Parses parses progress lines input; returns parsed values or errors; side effects: none.
    fn parses_progress_lines() {
        assert_eq!(
            parse_progress_line("out_time_us=2500000"),
            Some(ProgressUpdate::OutTime(2.5))
        );
        assert_eq!(
            parse_progress_line("out_time=00:01:05.500000"),
            Some(ProgressUpdate::OutTime(65.5))
        );
        assert_eq!(
            parse_progress_line("speed=1.25x"),
            Some(ProgressUpdate::Speed("1.25x".to_string()))
        );
        assert_eq!(
            parse_progress_line("progress=end"),
            Some(ProgressUpdate::Done)
        );
    }

    #[test]
    // AI-FUNC-SUMMARY: Validates progress UI observes quit-prompt ownership flag; returns test assertion result; side effects: none.
    fn progress_ui_observes_prompt_active_flag() {
        let prompt_active = Arc::new(AtomicBool::new(false));
        let mut events = NoopEventSink;
        let ui = ProgressUi::new(None, Arc::clone(&prompt_active), &mut events, false);

        assert!(!ui.prompt_active());
        prompt_active.store(true, Ordering::Relaxed);
        assert!(ui.prompt_active());
    }

    #[test]
    // AI-FUNC-SUMMARY: Validates cloned cancellation tokens share stop state; returns test assertion result; side effects: none.
    fn cancellation_token_shares_stop_request() {
        let token = CancellationToken::new();
        let clone = token.clone();

        assert!(!token.is_stop_requested());
        clone.request_stop();

        assert!(token.is_stop_requested());
    }

    #[test]
    // AI-FUNC-SUMMARY: Validates closure event sinks receive structured events; returns test assertion result; side effects: mutates a local vector.
    fn closure_event_sink_receives_events() {
        let mut events = Vec::new();
        let mut sink = |event| events.push(event);

        EventSink::on_event(&mut sink, Event::StopRequested);

        assert_eq!(events, vec![Event::StopRequested]);
    }

    #[test]
    // AI-FUNC-SUMMARY: Validates FFI event JSON escaping; returns test assertion result; side effects: none.
    fn serializes_events_as_json_for_ffi() {
        let event = Event::Log {
            level: LogLevel::Warning,
            message: "quote \" slash \\ newline\n".to_string(),
        };

        assert_eq!(
            event_json(&event),
            "{\"type\":\"log\",\"level\":\"warning\",\"message\":\"quote \\\" slash \\\\ newline\\n\"}"
        );
    }

    #[test]
    // AI-FUNC-SUMMARY: Verifies the settings event carries every option a front end shows; returns test assertion result; side effects: none.
    fn serializes_settings_event_as_json() {
        let event = Event::SettingsSelected {
            codec: TargetCodec::Hevc,
            preservation: Preservation::Flexible,
            encoder_preference: EncoderPreference::Gpu,
            output_format: OutputFormat::MkvFallback,
            quality_mode: QualityMode::Search,
            quality_check: QualityCheck::Sampled,
            quality_target: 9500,
            review_threshold: 9300,
        };

        assert_eq!(
            event_json(&event),
            "{\"type\":\"settings_selected\",\"codec\":\"hevc\",\"preservation\":\"flexible\",\"encoder_preference\":\"gpu\",\"output_format\":\"mkv-fallback\",\"quality_mode\":\"search\",\"quality_check\":\"sampled\",\"quality_target\":9500,\"review_threshold\":9300}"
        );
    }

    #[test]
    // AI-FUNC-SUMMARY: Verifies the quality events serialize with their optional fields; returns test assertion result; side effects: none.
    fn serializes_quality_events_as_json() {
        let measured = Event::QualityMeasured {
            index: 2,
            score: 9412,
            subsample: 5,
            target: 9500,
            passed: true,
            caveat: None,
        };
        assert_eq!(
            event_json(&measured),
            "{\"type\":\"quality_measured\",\"index\":2,\"score\":9412,\"subsample\":5,\"target\":9500,\"passed\":true,\"caveat\":null}"
        );

        let search = Event::QualitySearch {
            index: 1,
            iteration: 3,
            encoder: "libsvtav1".to_string(),
            quality: "crf=30".to_string(),
            score: Some(9550),
        };
        assert_eq!(
            event_json(&search),
            "{\"type\":\"quality_search\",\"index\":1,\"iteration\":3,\"encoder\":\"libsvtav1\",\"quality\":\"crf=30\",\"score\":9550}"
        );
    }

    #[test]
    // AI-FUNC-SUMMARY: Validates adapted file-attempt event JSON serialization; returns test assertion result; side effects: none.
    fn serializes_adapted_file_attempt_event_as_json() {
        let event = Event::FileAttempt {
            index: 2,
            attempt: 3,
            total_attempts: 5,
            encoder: "av1_nvenc".to_string(),
            quality: "crf=24".to_string(),
            plan: "adapted".to_string(),
            target_pix_fmt: Some("yuv420p10le".to_string()),
            fallback_reason: Some("unsupported pixel format".to_string()),
        };

        assert_eq!(
            event_json(&event),
            "{\"type\":\"file_attempt\",\"index\":2,\"attempt\":3,\"total_attempts\":5,\"encoder\":\"av1_nvenc\",\"quality\":\"crf=24\",\"plan\":\"adapted\",\"target_pix_fmt\":\"yuv420p10le\",\"fallback_reason\":\"unsupported pixel format\"}"
        );
    }

    #[test]
    // AI-FUNC-SUMMARY: Builds or derives formats size reduction data; returns the computed value; side effects: none.
    fn formats_size_reduction() {
        assert_eq!(format_bytes(1536), "1.50 KB");
        assert_eq!(format_bytes(5 * 1024 * 1024), "5.00 MB");
        assert_eq!(
            size_change_label(10 * 1024 * 1024, 4 * 1024 * 1024),
            "saved 6.00 MB, 60.0% smaller"
        );
        assert_eq!(
            conversion_size_label(ConversionResult {
                source_bytes: 10 * 1024 * 1024,
                output_bytes: 4 * 1024 * 1024,
            }),
            "10.0 MB -> 4.00 MB (saved 6.00 MB, 60.0% smaller)"
        );
    }

    #[test]
    // AI-FUNC-SUMMARY: Provides prefers vulkan before cpu fallback behavior; returns the declared result; side effects: see implementation.
    fn prefers_vulkan_before_cpu_fallback() {
        let encoders = r#"
 V..... av1_vulkan
 V..... libsvtav1
 V..... libaom-av1
"#;

        let encoder = detect_encoder_from_listing(encoders).unwrap();

        assert_eq!(encoder.name, "av1_vulkan");
        assert_eq!(encoder.kind, EncoderKind::Vulkan);
    }

    #[test]
    // AI-FUNC-SUMMARY: Validates Media Foundation AV1 is preferred before Vulkan fallback; returns test assertion result; side effects: none.
    fn prefers_media_foundation_before_vulkan() {
        let encoders = r#"
 V..... av1_vulkan
 V..... av1_mf
 V..... libsvtav1
"#;

        let encoder = detect_encoder_from_listing(encoders).unwrap();

        assert_eq!(encoder.name, "av1_mf");
        assert_eq!(encoder.kind, EncoderKind::Hardware);
    }

    #[test]
    // AI-FUNC-SUMMARY: Provides prefers vendor hardware before vulkan behavior; returns the declared result; side effects: see implementation.
    fn prefers_vendor_hardware_before_vulkan() {
        let encoders = r#"
 V..... av1_nvenc
 V..... av1_vulkan
 V..... libsvtav1
"#;

        let encoder = detect_encoder_from_listing(encoders).unwrap();

        assert_eq!(encoder.name, "av1_nvenc");
        assert_eq!(encoder.kind, EncoderKind::Hardware);
    }

    #[test]
    // AI-FUNC-SUMMARY: Provides prefers nvenc before qsv behavior; returns the declared result; side effects: see implementation.
    fn prefers_nvenc_before_qsv() {
        let encoders = r#"
 V..... av1_qsv
 V..... av1_nvenc
 V..... libsvtav1
"#;

        let encoder = detect_encoder_from_listing(encoders).unwrap();

        assert_eq!(encoder.name, "av1_nvenc");
        assert_eq!(encoder.kind, EncoderKind::Hardware);
    }

    #[test]
    // AI-FUNC-SUMMARY: Validates automatic exact selection prefers hardware before software; returns test assertion result; side effects: none.
    fn automatic_exact_selection_prefers_hardware_before_software() {
        let encoders = r#"
 V..... av1_nvenc
 V..... av1_vulkan
 V..... libsvtav1
 V..... libaom-av1
"#;

        let encoder = detect_encoder_from_listing(encoders).unwrap();

        assert_eq!(encoder.name, "av1_nvenc");
        assert_eq!(encoder.kind, EncoderKind::Hardware);
    }

    #[test]
    // AI-FUNC-SUMMARY: Validates automatic exact selection retains hardware candidates without software; returns test assertion result; side effects: none.
    fn automatic_exact_selection_supports_hardware_only_listing() {
        let encoders = r#"
 V..... av1_qsv
 V..... av1_mf
"#;

        let encoder = detect_encoder_from_listing(encoders).unwrap();

        assert_eq!(encoder.name, "av1_qsv");
        assert_eq!(encoder.kind, EncoderKind::Hardware);
    }

    #[test]
    // AI-FUNC-SUMMARY: Discovers or probes keeps encoder candidates in probe order data; returns collected metadata; side effects: may read filesystem or subprocess output.
    fn keeps_encoder_candidates_in_probe_order() {
        let encoders = r#"
 V..... libsvtav1
 V..... av1_qsv
 V..... av1_mf
 V..... av1_nvenc
"#;

        let names: Vec<String> = detect_encoder_candidates_from_listing(encoders, TargetCodec::Av1)
            .into_iter()
            .map(|encoder| encoder.name)
            .collect();

        assert_eq!(names, ["av1_nvenc", "av1_qsv", "av1_mf", "libsvtav1"]);
    }

    #[test]
    // AI-FUNC-SUMMARY: Validates exact transcode plans order GPU encoders before CPU fallback; returns test assertion result; side effects: none.
    fn exact_transcode_plans_order_gpu_before_cpu() {
        let encoders = r#"
 V..... libsvtav1
 V..... av1_qsv
 V..... av1_mf
 V..... av1_nvenc
 V..... librav1e
"#;

        let selection = EncoderSelection {
            candidates: detect_encoder_candidates_from_listing(encoders, TargetCodec::Av1),
            explicit: false,
        };
        let video = sample_video("h264", None);
        let names: Vec<String> = transcode_plans(
            &selection,
            EncoderPreference::Auto,
            Preservation::Strict,
            TargetCodec::Av1,
            &video,
            &plan_test_item(&video),
        )
        .into_iter()
        .map(|plan| plan.encoder.name.clone())
        .collect();

        assert_eq!(
            names,
            ["av1_nvenc", "av1_qsv", "av1_mf", "libsvtav1", "librav1e"]
        );
    }

    #[test]
    // AI-FUNC-SUMMARY: Validates consistency mode uses exact GPU then exact CPU plans without adaptation; returns test assertion result; side effects: none.
    fn consistency_plans_do_not_adapt_source_pixel_format() {
        let selection = EncoderSelection {
            candidates: vec![
                Encoder {
                    name: "av1_nvenc".to_string(),
                    kind: EncoderKind::Hardware,
                },
                Encoder {
                    name: "libaom-av1".to_string(),
                    kind: EncoderKind::LibAom,
                },
            ],
            explicit: false,
        };
        let mut video = sample_video("h264", None);
        video.pix_fmt = Some("yuv422p10le".to_string());
        video.bits_per_raw_sample = Some(10);

        let plans = transcode_plans(
            &selection,
            EncoderPreference::Auto,
            Preservation::Strict,
            TargetCodec::Av1,
            &video,
            &plan_test_item(&video),
        );

        assert_eq!(plans.len(), 2);
        assert_eq!(plans[0].encoder.name, "av1_nvenc");
        assert_eq!(plans[0].kind, TranscodePlanKind::Exact);
        assert_eq!(plans[1].encoder.name, "libaom-av1");
        assert_eq!(plans[1].kind, TranscodePlanKind::Exact);
    }

    #[test]
    // AI-FUNC-SUMMARY: Validates hardware mode inserts minimum-difference GPU adaptation before exact CPU fallback; returns test assertion result; side effects: none.
    fn hardware_plans_adapt_gpu_before_cpu_fallback() {
        let selection = EncoderSelection {
            candidates: vec![
                Encoder {
                    name: "av1_nvenc".to_string(),
                    kind: EncoderKind::Hardware,
                },
                Encoder {
                    name: "libaom-av1".to_string(),
                    kind: EncoderKind::LibAom,
                },
            ],
            explicit: false,
        };
        let mut video = sample_video("h264", None);
        video.pix_fmt = Some("yuv422p10le".to_string());
        video.bits_per_raw_sample = Some(10);

        let plans = transcode_plans(
            &selection,
            EncoderPreference::Gpu,
            Preservation::Strict,
            TargetCodec::Av1,
            &video,
            &plan_test_item(&video),
        );

        assert_eq!(plans.len(), 3);
        assert_eq!(plans[0].kind, TranscodePlanKind::Exact);
        assert_eq!(plans[1].encoder.name, "av1_nvenc");
        assert_eq!(plans[1].kind.output_pix_fmt(), Some("yuv420p10le"));
        assert_eq!(plans[2].encoder.name, "libaom-av1");
        assert_eq!(plans[2].kind, TranscodePlanKind::Exact);
    }

    #[test]
    // AI-FUNC-SUMMARY: Validates automatic adaptation rejects alpha and RGB formats and preserves HDR depth; returns test assertion result; side effects: none.
    fn hardware_pixel_format_adaptation_respects_safety_boundaries() {
        let mut video = sample_video("h264", None);
        video.pix_fmt = Some("yuva444p10le".to_string());
        video.bits_per_raw_sample = Some(10);
        assert_eq!(closest_hardware_pixel_format(&video), None);

        video.pix_fmt = Some("gbrp10le".to_string());
        assert_eq!(closest_hardware_pixel_format(&video), None);

        video.pix_fmt = Some("yuv422p".to_string());
        video.bits_per_raw_sample = Some(8);
        video.color_transfer = Some("smpte2084".to_string());
        assert_eq!(
            closest_hardware_pixel_format(&video).map(|value| value.0),
            Some("yuv420p10le")
        );

        video.color_transfer = None;
        video.bits_per_raw_sample = None;
        for pix_fmt in ["yuv420p9le", "yuv422p14le", "yuv444p16le"] {
            video.pix_fmt = Some(pix_fmt.to_string());
            assert_eq!(
                closest_hardware_pixel_format(&video).map(|value| value.0),
                Some("yuv420p10le"),
                "{pix_fmt} must not be reduced to 8-bit"
            );
        }

        video.pix_fmt = Some("yuv422p10le".to_string());
        video.bits_per_raw_sample = Some(8);
        assert_eq!(
            closest_hardware_pixel_format(&video).map(|value| value.0),
            Some("yuv420p10le")
        );
    }

    #[test]
    // AI-FUNC-SUMMARY: Validates encoder retry classification remains conservative for disk/input errors; returns test assertion result; side effects: none.
    fn classifies_only_known_encoder_failures_as_retryable() {
        assert!(ffmpeg_failure_is_encoder_retryable(
            "No NVENC capable devices found"
        ));
        assert!(ffmpeg_failure_is_encoder_retryable(
            "Unsupported pixel format yuv422p10le"
        ));
        assert!(!ffmpeg_failure_is_encoder_retryable(
            "No space left on device"
        ));
        assert!(!ffmpeg_failure_is_encoder_retryable(
            "Invalid data found when processing input"
        ));
        // An unexplained message that names a failure stays terminal.
        assert!(!ffmpeg_failure_is_encoder_retryable(
            "conversion failed for reasons unknown"
        ));
        // ffmpeg dying without saying anything is usually a hardware wrapper
        // crashing, which the next encoder may well survive.
        assert!(ffmpeg_failure_is_encoder_retryable(
            "frame= 2366 fps=83 q=44.0 size=1280KiB time=00:01:38.54"
        ));
        assert!(ffmpeg_failure_is_encoder_retryable(""));
        assert!(!validation_failure_is_retryable(
            "ffprobe failed for output.mp4 with status 1: process crashed"
        ));
        assert!(!validation_failure_is_retryable(
            "output color metadata changed; after metadata repair: ffprobe failed for output.mp4"
        ));
    }

    #[test]
    // AI-FUNC-SUMMARY: Builds or derives builds encoder runtime check args data; returns the computed value; side effects: none.
    fn builds_encoder_runtime_check_args() {
        let encoder = Encoder {
            name: "av1_nvenc".to_string(),
            kind: EncoderKind::Hardware,
        };

        let args = encoder_runtime_check_args(&encoder);

        assert!(args.windows(2).any(|pair| pair == ["-c:v", "av1_nvenc"]));
        assert!(args.windows(2).any(|pair| pair == ["-f", "lavfi"]));
        assert!(args.windows(2).any(|pair| pair == ["-f", "null"]));
        assert!(args.iter().any(|arg| arg == "-nostdin"));
        assert!(args.iter().any(|arg| arg == "testsrc2=size=640x360:rate=1"));
    }

    #[test]
    // AI-FUNC-SUMMARY: Validates Media Foundation AV1 uses hardware bitrate quality arguments; returns test assertion result; side effects: none.
    fn media_foundation_av1_uses_hardware_bitrate_quality() {
        let encoder = Encoder {
            name: "av1_mf".to_string(),
            kind: EncoderKind::Hardware,
        };

        assert_eq!(
            encoder_runtime_quality_args(&encoder),
            ["-b:v".to_string(), "600k".to_string()]
        );
    }

    #[test]
    // AI-FUNC-SUMMARY: Validates Media Foundation AV1 transcode args use target bitrate and not CRF; returns test assertion result; side effects: creates and removes a synthetic temp file.
    fn media_foundation_av1_transcode_args_use_bitrate() {
        let root = make_temp_dir("mf-quality");
        let input = root.join("clip.mp4");
        fs::write(&input, b"").unwrap();
        let video = sample_video("h264", Some(8_000_000));
        let streams = vec![sample_stream(0, "video", "h264")];
        let item = WorkItem::new(
            input,
            video,
            streams,
            Vec::new(),
            ItemPolicy {
                keep_original: true,
                tmp_dir: None,
                output_format: OutputFormat::Mp4,
                target_codec: TargetCodec::Av1,
                preservation: Preservation::Strict,
            },
        )
        .unwrap();
        let encoder = Encoder {
            name: "av1_mf".to_string(),
            kind: EncoderKind::Hardware,
        };

        let args = build_ffmpeg_args(&item, &encoder);

        assert!(args.windows(2).any(|pair| pair == ["-c:v:0", "av1_mf"]));
        assert!(args.windows(2).any(|pair| pair[0] == "-b:v"));
        assert!(!args.iter().any(|arg| arg == "-crf"));

        remove_temp_dir(root);
    }

    #[test]
    // AI-FUNC-SUMMARY: Provides filters candidate video paths by extension only behavior; returns the declared result; side effects: see implementation.
    fn filters_candidate_video_paths_by_extension_only() {
        assert!(is_candidate_video_path(Path::new("clip.MP4")));
        assert!(is_candidate_video_path(Path::new("movie.mkv")));
        assert!(!is_candidate_video_path(Path::new("cover.jpg")));
        assert!(!is_candidate_video_path(Path::new("README")));
    }

    #[test]
    // AI-FUNC-SUMMARY: Provides ffmpeg args lock source frame rate behavior; returns the declared result; side effects: see implementation.
    fn ffmpeg_args_lock_source_frame_rate() {
        let root = make_temp_dir("args");
        let input = root.join("clip.mkv");
        fs::write(&input, b"").unwrap();
        let mut video = sample_video("h264", Some(8_000_000));
        video.avg_frame_rate = Some("30000/1001".to_string());
        video.nominal_frame_rate = Some("60000/1001".to_string());
        video.color_range = Some("tv".to_string());
        video.color_space = Some("bt709".to_string());
        video.color_transfer = Some("bt709".to_string());
        video.color_primaries = Some("bt709".to_string());
        video.chroma_location = Some("left".to_string());
        let item = WorkItem::new(
            input,
            video,
            vec![
                sample_stream(0, "video", "h264"),
                sample_stream(1, "audio", "aac"),
            ],
            Vec::new(),
            ItemPolicy {
                keep_original: true,
                tmp_dir: None,
                output_format: OutputFormat::Mp4,
                target_codec: TargetCodec::Av1,
                preservation: Preservation::Strict,
            },
        )
        .unwrap();
        let encoder = Encoder {
            name: "libsvtav1".to_string(),
            kind: EncoderKind::SvtAv1,
        };

        let args = build_ffmpeg_args(&item, &encoder);

        assert!(args.iter().any(|arg| arg == "-nostdin"));
        assert!(args.windows(2).any(|pair| pair == ["-r:v:0", "60000/1001"]));
        assert!(args.windows(2).any(|pair| pair == ["-fps_mode:v:0", "cfr"]));
        assert!(args.windows(2).any(|pair| pair == ["-map", "0:0"]));
        assert!(args.windows(2).any(|pair| pair == ["-map", "0:1"]));
        assert!(!args.windows(2).any(|pair| pair == ["-map", "0"]));
        assert!(args.windows(2).any(|pair| pair == ["-map_chapters", "-1"]));
        assert!(args.windows(2).any(|pair| pair == ["-c", "copy"]));
        assert!(args.windows(2).any(|pair| pair == ["-c:v:0", "libsvtav1"]));
        assert!(!args.iter().any(|arg| arg == "-c:a"));
        assert!(
            args.windows(2)
                .any(|pair| pair == ["-pix_fmt:v:0", "yuv420p"])
        );
        assert!(
            args.windows(2)
                .any(|pair| pair == ["-color_range:v:0", "tv"])
        );
        assert!(
            args.windows(2)
                .any(|pair| pair == ["-colorspace:v:0", "bt709"])
        );
        assert!(
            args.windows(2)
                .any(|pair| pair == ["-color_trc:v:0", "bt709"])
        );
        assert!(
            args.windows(2)
                .any(|pair| pair == ["-color_primaries:v:0", "bt709"])
        );
        assert!(
            args.windows(2)
                .any(|pair| pair == ["-chroma_sample_location:v:0", "left"])
        );
        assert!(args.windows(2).any(|pair| pair == [
            "-bsf:v:0",
            "av1_metadata=color_primaries=1:transfer_characteristics=1:matrix_coefficients=1:color_range=0:chroma_sample_position=1"
        ]));
        assert!(!args.iter().any(|arg| arg == "-vf" || arg == "-filter:v"));

        remove_temp_dir(root);
    }

    #[test]
    // AI-FUNC-SUMMARY: Builds or derives builds av1 metadata bitstream filter for known color metadata data; returns the computed value; side effects: none.
    fn builds_av1_metadata_bitstream_filter_for_known_color_metadata() {
        let mut video = sample_video("h264", Some(8_000_000));
        video.color_range = Some("tv".to_string());
        video.color_space = Some("bt709".to_string());
        video.color_transfer = Some("bt709".to_string());
        video.color_primaries = Some("bt709".to_string());
        video.chroma_location = Some("left".to_string());

        assert_eq!(
            metadata_bsf_arg(TargetCodec::Av1, &video, &MetadataExemptions::default()),
            Some(
                "av1_metadata=color_primaries=1:transfer_characteristics=1:matrix_coefficients=1:color_range=0:chroma_sample_position=1"
                    .to_string()
            )
        );
    }

    #[test]
    // AI-FUNC-SUMMARY: Builds or derives builds av1 metadata repair args without reencoding data; returns the computed value; side effects: none.
    fn builds_av1_metadata_repair_args_without_reencoding() {
        let args = build_av1_metadata_repair_args(
            Path::new("temp.mp4"),
            Path::new("repair.mp4"),
            &sample_video("av1", Some(8_000_000)),
            "av1_metadata=chroma_sample_position=1",
            &[0, 1, 2],
            &ChapterPolicy::Ordinary,
            OutputContainer::Mp4,
        );

        assert!(args.windows(2).any(|pair| pair == ["-i", "temp.mp4"]));
        assert!(args.windows(2).any(|pair| pair == ["-map", "0:0"]));
        assert!(args.windows(2).any(|pair| pair == ["-map", "0:1"]));
        assert!(args.windows(2).any(|pair| pair == ["-map", "0:2"]));
        assert!(!args.windows(2).any(|pair| pair == ["-map", "0"]));
        assert!(args.windows(2).any(|pair| pair == ["-map_chapters", "-1"]));
        assert!(args.windows(2).any(|pair| pair == ["-c", "copy"]));
        assert!(
            args.windows(2)
                .any(|pair| pair == ["-bsf:v:0", "av1_metadata=chroma_sample_position=1"])
        );
        assert!(
            args.windows(2)
                .any(|pair| pair == ["-movflags", "+faststart+write_colr"])
        );
        assert!(args.iter().any(|arg| arg == "repair.mp4"));
    }

    #[test]
    // AI-FUNC-SUMMARY: Provides metadata repair args repeat color output metadata behavior; returns the declared result; side effects: see implementation.
    fn metadata_repair_args_repeat_color_output_metadata() {
        let mut video = sample_video("av1", Some(8_000_000));
        video.color_range = Some("tv".to_string());
        video.color_space = Some("bt709".to_string());
        video.color_transfer = Some("bt709".to_string());
        video.color_primaries = Some("bt709".to_string());
        video.chroma_location = Some("left".to_string());

        let args = build_av1_metadata_repair_args(
            Path::new("temp.mp4"),
            Path::new("repair.mp4"),
            &video,
            "av1_metadata=chroma_sample_position=1",
            &[0],
            &ChapterPolicy::Ordinary,
            OutputContainer::Mp4,
        );

        assert!(
            args.windows(2)
                .any(|pair| pair == ["-color_range:v:0", "tv"])
        );
        assert!(
            args.windows(2)
                .any(|pair| pair == ["-chroma_sample_location:v:0", "left"])
        );
    }

    #[test]
    // AI-FUNC-SUMMARY: Provides metadata repair paths stay next to temp output behavior; returns the declared result; side effects: see implementation.
    fn metadata_repair_paths_stay_next_to_temp_output() {
        let temp = Path::new("/tmp/.myvidcomp-clip.tmp.mp4");

        assert_eq!(
            metadata_repair_temp_path(temp),
            PathBuf::from("/tmp/.myvidcomp-clip.tmp.mp4.repairing.mp4")
        );
        assert_eq!(
            metadata_backup_temp_path(temp),
            PathBuf::from("/tmp/.myvidcomp-clip.tmp.mp4.before-repair.mp4")
        );
    }

    #[test]
    // AI-FUNC-SUMMARY: Performs replace with repaired output keeps repaired file operation; returns operation status or result; side effects: may spawn processes, move files, or write progress.
    fn replace_with_repaired_output_keeps_repaired_file() {
        let root = make_temp_dir("metadata-repair");
        let original = root.join("temp.mp4");
        let repaired = root.join("repair.mp4");
        let backup = root.join("backup.mp4");
        fs::write(&original, b"old").unwrap();
        fs::write(&repaired, b"new").unwrap();

        replace_with_repaired_output(&original, &repaired, &backup).unwrap();

        assert_eq!(fs::read(&original).unwrap(), b"new");
        assert!(!backup.exists());
        assert!(!repaired.exists());
        remove_temp_dir(root);
    }

    #[test]
    // AI-FUNC-SUMMARY: Builds or derives maps common color values to av1 metadata codes data; returns the computed value; side effects: none.
    fn maps_common_color_values_to_av1_metadata_codes() {
        assert_eq!(av1_color_primaries_value("bt709"), Some("1"));
        assert_eq!(av1_color_primaries_value("bt2020"), Some("9"));
        assert_eq!(av1_transfer_characteristics_value("smpte2084"), Some("16"));
        assert_eq!(
            av1_transfer_characteristics_value("arib-std-b67"),
            Some("18")
        );
        assert_eq!(av1_matrix_coefficients_value("bt2020nc"), Some("9"));
        assert_eq!(codec::color_range_value(TargetCodec::Av1, "pc"), Some(1));
    }

    #[test]
    // AI-FUNC-SUMMARY: Validates rejects unmappable av1 metadata before transcoding conditions; returns success, failure, or test assertion result; side effects: may inspect files or emit test failures.
    fn rejects_unmappable_av1_metadata_before_transcoding() {
        let mut video = sample_video("h264", Some(8_000_000));
        video.chroma_location = Some("center".to_string());

        let reason = unsupported_metadata_reason(TargetCodec::Av1, &video).unwrap();

        assert!(reason.contains("chroma position center"));
    }

    #[test]
    // AI-FUNC-SUMMARY: Validates accepts mappable av1 metadata before transcoding conditions; returns success, failure, or test assertion result; side effects: may inspect files or emit test failures.
    fn accepts_mappable_av1_metadata_before_transcoding() {
        let mut video = sample_video("h264", Some(8_000_000));
        video.color_range = Some("tv".to_string());
        video.color_space = Some("bt709".to_string());
        video.color_transfer = Some("bt709".to_string());
        video.color_primaries = Some("bt709".to_string());
        video.chroma_location = Some("left".to_string());

        assert!(unsupported_metadata_reason(TargetCodec::Av1, &video).is_none());
    }

    #[test]
    // AI-FUNC-SUMMARY: Provides compares frame rates with small tolerance behavior; returns the declared result; side effects: see implementation.
    fn compares_frame_rates_with_small_tolerance() {
        assert!(fps_matches(29.970, 29.971));
        assert!(!fps_matches(29.970, 30.000));
    }

    #[test]
    // AI-FUNC-SUMMARY: Parses parses nominal frame rate from probe input; returns parsed values or errors; side effects: none.
    fn parses_nominal_frame_rate_from_probe() {
        let info = parse_video_probe(
            r#"
codec_name=mpeg2video
width=720
height=480
avg_frame_rate=31/1
r_frame_rate=24000/1001
duration=60.0
"#,
        )
        .unwrap()
        .unwrap();

        assert!(fps_matches(info.fps, 31.0));
        assert!(fps_matches(info.nominal_fps, 23.976));
        assert_eq!(info.avg_frame_rate, Some("31/1".to_string()));
        assert_eq!(info.nominal_frame_rate, Some("24000/1001".to_string()));
    }

    #[test]
    // AI-FUNC-SUMMARY: Parses parses color metadata from probe input; returns parsed values or errors; side effects: none.
    fn parses_color_metadata_from_probe() {
        let info = parse_video_probe(
            r#"
codec_name=h264
width=1920
height=1080
sample_aspect_ratio=1:1
display_aspect_ratio=16:9
field_order=progressive
avg_frame_rate=30/1
r_frame_rate=30/1
color_range=tv
color_space=bt709
color_transfer=bt709
color_primaries=bt709
chroma_location=left
duration=60.0
"#,
        )
        .unwrap()
        .unwrap();

        assert_eq!(info.color_range, Some("tv".to_string()));
        assert_eq!(info.color_space, Some("bt709".to_string()));
        assert_eq!(info.color_transfer, Some("bt709".to_string()));
        assert_eq!(info.color_primaries, Some("bt709".to_string()));
        assert_eq!(info.chroma_location, Some("left".to_string()));
        assert_eq!(info.sample_aspect_ratio, Some("1:1".to_string()));
        assert_eq!(info.display_aspect_ratio, Some("16:9".to_string()));
        assert_eq!(info.field_order, Some("progressive".to_string()));
    }

    #[test]
    // AI-FUNC-SUMMARY: Discovers or probes keeps streams with missing codec names in signature data; returns collected metadata; side effects: may read filesystem or subprocess output.
    fn keeps_streams_with_missing_codec_names_in_signature() {
        let streams = parse_stream_probe(
            r#"
index=0|codec_name=h264|codec_type=video
index=1|codec_type=data
index=2|codec_name=aac|codec_type=audio
"#,
        );

        assert_eq!(streams.len(), 3);
        assert_eq!(streams[1].codec_type, "data");
        assert_eq!(streams[1].codec_name, "unknown");
    }

    #[test]
    // AI-FUNC-SUMMARY: Validates duplicate ffprobe stream entries are deduplicated by stream index; returns test assertion result; side effects: none.
    fn deduplicates_stream_probe_entries_by_ffprobe_index() {
        let streams = parse_stream_probe(
            r#"
index=4|codec_name=h264|codec_type=video|codec_tag_string=avc1|id=0x1|TAG:handler_name=VideoHandler
index=9|codec_name=aac|codec_type=audio|codec_tag_string=mp4a|id=2|TAG:handler_name=SoundHandler
index=9|codec_name=aac|codec_type=audio
index=1|codec_name=aac|codec_type=audio
index=2|codec_name=timed_id3|codec_type=data
"#,
        );

        assert_eq!(streams.len(), 4);
        assert_eq!(streams[0].index, 4);
        assert_eq!(streams[0].track_id, Some(1));
        assert_eq!(streams[0].codec_tag_string.as_deref(), Some("avc1"));
        assert_eq!(streams[0].handler_name.as_deref(), Some("VideoHandler"));
        assert_eq!(streams[1].index, 9);
        assert_eq!(streams[1].track_id, Some(2));
    }

    #[test]
    // AI-FUNC-SUMMARY: Provides ignores unknown color metadata from probe behavior; returns the declared result; side effects: see implementation.
    fn ignores_unknown_color_metadata_from_probe() {
        let info = parse_video_probe(
            r#"
codec_name=h264
width=1920
height=1080
avg_frame_rate=30/1
r_frame_rate=30/1
color_range=unknown
color_space=unknown
color_transfer=unknown
color_primaries=unknown
duration=60.0
"#,
        )
        .unwrap()
        .unwrap();

        assert_eq!(info.color_range, None);
        assert_eq!(info.color_space, None);
        assert_eq!(info.color_transfer, None);
        assert_eq!(info.color_primaries, None);
    }

    #[test]
    // AI-FUNC-SUMMARY: Provides output frame rate prefers nominal source rate behavior; returns the declared result; side effects: see implementation.
    fn output_frame_rate_prefers_nominal_source_rate() {
        let mut video = sample_video("mpeg2video", Some(2_000_000));
        video.avg_frame_rate = Some("31/1".to_string());
        video.nominal_frame_rate = Some("60000/1001".to_string());

        assert_eq!(output_frame_rate_arg(&video), Some("60000/1001"));
    }

    #[test]
    // AI-FUNC-SUMMARY: Provides output pixel format normalizes full range yuvj behavior; returns the declared result; side effects: see implementation.
    fn output_pixel_format_normalizes_full_range_yuvj() {
        let mut video = sample_video("mjpeg", Some(2_000_000));
        video.pix_fmt = Some("yuvj420p".to_string());

        assert_eq!(output_pix_fmt_arg(&video), Some("yuv420p"));
    }

    #[test]
    // AI-FUNC-SUMMARY: Validates accepts average frame rate drift when nominal rate matches conditions; returns success, failure, or test assertion result; side effects: may inspect files or emit test failures.
    fn accepts_average_frame_rate_drift_when_nominal_rate_matches() {
        let mut source = sample_video("mpeg2video", Some(2_000_000));
        source.fps = 31.0;
        source.nominal_fps = 23.976;
        let mut output = sample_video("av1", Some(1_000_000));
        output.fps = 23.976;
        output.nominal_fps = 23.976;

        assert!(frame_rate_validation_error(&source, &output, Path::new("clip.mp4")).is_none());
    }

    #[test]
    // AI-FUNC-SUMMARY: Validates rejects frame rate change when no rate pair matches conditions; returns success, failure, or test assertion result; side effects: may inspect files or emit test failures.
    fn rejects_frame_rate_change_when_no_rate_pair_matches() {
        let mut source = sample_video("h264", Some(2_000_000));
        source.fps = 30.0;
        source.nominal_fps = 30.0;
        let mut output = sample_video("av1", Some(1_000_000));
        output.fps = 24.0;
        output.nominal_fps = 24.0;

        let err = frame_rate_validation_error(&source, &output, Path::new("clip.mp4")).unwrap();

        assert!(err.contains("avg 30.000 fps / nominal 30.000 fps"));
        assert!(err.contains("avg 24.000 fps / nominal 24.000 fps"));
    }

    #[test]
    // AI-FUNC-SUMMARY: Validates rejects changed color metadata conditions; returns success, failure, or test assertion result; side effects: may inspect files or emit test failures.
    fn rejects_changed_color_metadata() {
        let mut source = sample_video("h264", Some(2_000_000));
        source.color_space = Some("bt709".to_string());
        let mut output = sample_video("av1", Some(1_000_000));
        output.color_space = Some("bt2020nc".to_string());

        let err = color_metadata_validation_error(
            &source,
            &output,
            Path::new("clip.mp4"),
            &MetadataExemptions::default(),
        )
        .unwrap();

        assert!(err.contains("output color space changed from bt709 to bt2020nc"));
    }

    #[test]
    // AI-FUNC-SUMMARY: Validates rejects changed pixel format conditions; returns success, failure, or test assertion result; side effects: may inspect files or emit test failures.
    fn rejects_changed_pixel_format() {
        let mut source = sample_video("h264", Some(2_000_000));
        source.pix_fmt = Some("yuv420p".to_string());
        let mut output = sample_video("av1", Some(1_000_000));
        output.pix_fmt = Some("yuv422p".to_string());

        let err = pixel_format_validation_error(&source, &output, Path::new("clip.mp4")).unwrap();

        assert!(err.contains("output pixel format changed from yuv420p to yuv422p"));
    }

    #[test]
    // AI-FUNC-SUMMARY: Validates accepts normalized full range yuvj pixel format conditions; returns success, failure, or test assertion result; side effects: may inspect files or emit test failures.
    fn accepts_normalized_full_range_yuvj_pixel_format() {
        let mut source = sample_video("mjpeg", Some(2_000_000));
        source.pix_fmt = Some("yuvj420p".to_string());
        let mut output = sample_video("av1", Some(1_000_000));
        output.pix_fmt = Some("yuv420p".to_string());

        assert!(pixel_format_validation_error(&source, &output, Path::new("clip.mp4")).is_none());
    }

    #[test]
    // AI-FUNC-SUMMARY: Validates rejects missing color metadata conditions; returns success, failure, or test assertion result; side effects: may inspect files or emit test failures.
    fn rejects_missing_color_metadata() {
        let mut source = sample_video("h264", Some(2_000_000));
        source.color_transfer = Some("bt709".to_string());
        let output = sample_video("av1", Some(1_000_000));

        let err = color_metadata_validation_error(
            &source,
            &output,
            Path::new("clip.mp4"),
            &MetadataExemptions::default(),
        )
        .unwrap();

        assert!(err.contains("output color transfer is missing, expected bt709"));
    }

    #[test]
    // AI-FUNC-SUMMARY: Validates accepts missing av1 mp4 chroma location report conditions; returns success, failure, or test assertion result; side effects: may inspect files or emit test failures.
    fn accepts_missing_av1_mp4_chroma_location_report() {
        let mut source = sample_video("h264", Some(2_000_000));
        source.chroma_location = Some("topleft".to_string());
        let mut output = sample_video("av1", Some(1_000_000));
        output.chroma_location = None;

        assert!(
            color_metadata_validation_error(
                &source,
                &output,
                Path::new("clip.mp4"),
                &MetadataExemptions::default(),
            )
            .is_none()
        );
    }

    #[test]
    // AI-FUNC-SUMMARY: Provides still rejects changed chroma location report behavior; returns the declared result; side effects: see implementation.
    fn still_rejects_changed_chroma_location_report() {
        let mut source = sample_video("h264", Some(2_000_000));
        source.chroma_location = Some("topleft".to_string());
        let mut output = sample_video("av1", Some(1_000_000));
        output.chroma_location = Some("left".to_string());

        let err = color_metadata_validation_error(
            &source,
            &output,
            Path::new("clip.mp4"),
            &MetadataExemptions::default(),
        )
        .unwrap();

        assert!(err.contains("output chroma location changed from topleft to left"));
    }

    #[test]
    // AI-FUNC-SUMMARY: Validates rejects changed display metadata conditions; returns success, failure, or test assertion result; side effects: may inspect files or emit test failures.
    fn rejects_changed_display_metadata() {
        let mut source = sample_video("h264", Some(2_000_000));
        source.sample_aspect_ratio = Some("4:3".to_string());
        let mut output = sample_video("av1", Some(1_000_000));
        output.sample_aspect_ratio = Some("1:1".to_string());

        let err =
            display_metadata_validation_error(&source, &output, Path::new("clip.mp4")).unwrap();

        assert!(err.contains("output sample aspect ratio changed from 4:3 to 1:1"));
    }

    #[test]
    // AI-FUNC-SUMMARY: Validates near-equivalent sample aspect ratio metadata is accepted; returns test assertion result; side effects: none.
    fn accepts_near_equivalent_sample_aspect_ratio_metadata() {
        let mut source = sample_video("h264", Some(2_000_000));
        source.sample_aspect_ratio = Some("26159:26133".to_string());
        let mut output = sample_video("av1", Some(1_000_000));
        output.sample_aspect_ratio = Some("74520:74447".to_string());

        assert!(
            display_metadata_validation_error(&source, &output, Path::new("clip.mp4")).is_none()
        );
    }

    #[test]
    // AI-FUNC-SUMMARY: Validates metadata repair is attempted only for remux-repairable failures; returns test assertion result; side effects: none.
    fn classifies_validation_errors_that_may_need_remux_repair() {
        assert!(validation_error_may_need_remux_repair(
            "output stream count changed from 3 to 4"
        ));
        assert!(validation_error_may_need_remux_repair(
            "output color transfer is missing"
        ));
        assert!(validation_error_may_need_remux_repair(
            "output chroma location is missing"
        ));
        assert!(!validation_error_may_need_remux_repair(
            "output sample aspect ratio changed"
        ));
    }

    #[test]
    // AI-FUNC-SUMMARY: Validates unusable temporary output classification; returns test assertion result; side effects: none.
    fn classifies_unusable_temp_validation_errors() {
        assert!(validation_error_indicates_unusable_temp(
            "output has no readable video stream: .myvidcomp-clip.tmp.mp4"
        ));
        assert!(validation_error_indicates_unusable_temp(
            "output codec is h264, expected av1"
        ));
        assert!(!validation_error_indicates_unusable_temp(
            "output sample aspect ratio changed"
        ));
        assert!(is_temp_output_path(Path::new(
            ".myvidcomp-clip-123.tmp.mp4"
        )));
        assert!(!is_temp_output_path(Path::new("clip.mp4")));
    }

    #[test]
    // AI-FUNC-SUMMARY: Provides skips interlaced sources instead of deinterlacing behavior; returns the declared result; side effects: see implementation.
    fn skips_interlaced_sources_instead_of_deinterlacing() {
        let root = make_temp_dir("interlaced");
        let input = root.join("clip.mpg");
        fs::write(&input, b"").unwrap();
        let mut video = sample_video("mpeg2video", Some(2_000_000));
        video.field_order = Some("tt".to_string());
        let streams = vec![sample_stream(0, "video", "mpeg2video")];

        assert!(matches!(
            WorkItem::new(
                input,
                video,
                streams,
                Vec::new(),
                ItemPolicy {
                    keep_original: true,
                    tmp_dir: None,
                    output_format: OutputFormat::Mp4,
                    target_codec: TargetCodec::Av1,
                    preservation: Preservation::Strict,
                },
            ),
            Err(SkipReason::UnsafeReplacement(_))
        ));

        remove_temp_dir(root);
    }

    #[test]
    // AI-FUNC-SUMMARY: Validates MP4-incompatible data streams are skipped before encoding; returns test assertion result; side effects: creates and removes a synthetic temp file.
    fn skips_mp4_incompatible_data_streams_before_encoding() {
        let root = make_temp_dir("data-stream");
        let input = root.join("clip.ts");
        fs::write(&input, b"").unwrap();
        let video = sample_video("h264", Some(2_000_000));
        let streams = vec![
            sample_stream(0, "video", "h264"),
            sample_stream(1, "audio", "aac"),
            sample_stream(2, "data", "timed_id3"),
        ];

        assert!(matches!(
            WorkItem::new(
                input,
                video,
                streams,
                Vec::new(),
                ItemPolicy {
                    keep_original: true,
                    tmp_dir: None,
                    output_format: OutputFormat::Mp4,
                    target_codec: TargetCodec::Av1,
                    preservation: Preservation::Strict,
                },
            ),
            Err(SkipReason::UnsafeReplacement(_))
        ));

        remove_temp_dir(root);
    }

    #[test]
    // AI-FUNC-SUMMARY: Validates validates only first video codec changes conditions; returns success, failure, or test assertion result; side effects: may inspect files or emit test failures.
    fn validates_only_first_video_codec_changes() {
        let source = vec![
            StreamSignature {
                codec_type: "video".to_string(),
                codec_name: "h264".to_string(),
            },
            StreamSignature {
                codec_type: "audio".to_string(),
                codec_name: "aac".to_string(),
            },
            StreamSignature {
                codec_type: "video".to_string(),
                codec_name: "mjpeg".to_string(),
            },
        ];
        let output = vec![
            sample_stream(0, "video", "av1"),
            sample_stream(1, "audio", "aac"),
            sample_stream(2, "video", "mjpeg"),
        ];

        assert!(
            stream_signature_validation_error(
                &source,
                &output,
                Path::new("clip.mp4"),
                TargetCodec::Av1
            )
            .is_none()
        );
    }

    #[test]
    // AI-FUNC-SUMMARY: Validates rejects non first stream codec change conditions; returns success, failure, or test assertion result; side effects: may inspect files or emit test failures.
    fn rejects_non_first_stream_codec_change() {
        let source = vec![
            StreamSignature {
                codec_type: "video".to_string(),
                codec_name: "h264".to_string(),
            },
            StreamSignature {
                codec_type: "audio".to_string(),
                codec_name: "aac".to_string(),
            },
        ];
        let output = vec![
            sample_stream(0, "video", "av1"),
            sample_stream(1, "audio", "alac"),
        ];

        let err = stream_signature_validation_error(
            &source,
            &output,
            Path::new("clip.mp4"),
            TargetCodec::Av1,
        )
        .unwrap();

        assert!(err.contains("output stream #1 codec changed from aac to alac"));
    }

    #[test]
    // AI-FUNC-SUMMARY: Builds or derives estimates quality from bitrate data; returns the computed value; side effects: none.
    fn estimates_quality_from_bitrate() {
        let high = sample_video("h264", Some(18_000_000));
        let low = sample_video("h264", Some(1_000_000));

        assert!(estimate_av1_crf(&high) < estimate_av1_crf(&low));
    }

    #[test]
    // AI-FUNC-SUMMARY: Builds or derives maps source crf metadata data; returns the computed value; side effects: none.
    fn maps_source_crf_metadata() {
        let mut video = sample_video("hevc", Some(5_000_000));
        video.encoder_tag = Some("x265 - crf=18".to_string());

        assert_eq!(estimate_av1_crf(&video), 16);
    }

    #[test]
    // AI-FUNC-SUMMARY: Provides classifies common codecs for quality mapping behavior; returns the declared result; side effects: see implementation.
    fn classifies_common_codecs_for_quality_mapping() {
        assert_eq!(source_codec_family("h264"), SourceCodecFamily::H264);
        assert_eq!(source_codec_family("avc1"), SourceCodecFamily::H264);
        assert_eq!(source_codec_family("hevc"), SourceCodecFamily::H265);
        assert_eq!(source_codec_family("hvc1"), SourceCodecFamily::H265);
        assert_eq!(source_codec_family("vp8"), SourceCodecFamily::Vp8);
        assert_eq!(source_codec_family("vp9"), SourceCodecFamily::Vp9);
        assert_eq!(source_codec_family("mpeg1video"), SourceCodecFamily::Mpeg1);
        assert_eq!(source_codec_family("mpeg2video"), SourceCodecFamily::Mpeg2);
        assert_eq!(source_codec_family("mpeg4"), SourceCodecFamily::Mpeg4Part2);
        assert_eq!(source_codec_family("divx"), SourceCodecFamily::Mpeg4Part2);
        assert_eq!(source_codec_family("wmv3"), SourceCodecFamily::Vc1);
        assert_eq!(source_codec_family("mjpeg"), SourceCodecFamily::Mjpeg);
    }

    #[test]
    // AI-FUNC-SUMMARY: Builds or derives maps common source codecs to av1 crf from bitrate data; returns the computed value; side effects: none.
    fn maps_common_source_codecs_to_av1_crf_from_bitrate() {
        let bit_rate = Some(8_000_000);

        assert_eq!(estimate_av1_crf(&sample_video("hevc", bit_rate)), 18);
        assert_eq!(estimate_av1_crf(&sample_video("vp9", bit_rate)), 20);
        assert_eq!(estimate_av1_crf(&sample_video("h264", bit_rate)), 20);
        assert_eq!(estimate_av1_crf(&sample_video("vp8", bit_rate)), 23);
        assert_eq!(estimate_av1_crf(&sample_video("mpeg4", bit_rate)), 23);
        assert_eq!(estimate_av1_crf(&sample_video("mpeg2video", bit_rate)), 26);
        assert_eq!(estimate_av1_crf(&sample_video("mpeg1video", bit_rate)), 26);
    }

    #[test]
    // AI-FUNC-SUMMARY: Builds or derives maps common source quantizers to av1 crf data; returns the computed value; side effects: none.
    fn maps_common_source_quantizers_to_av1_crf() {
        assert_eq!(
            map_source_quantizer_to_av1(&sample_video("hevc", None), 28.0),
            26
        );
        assert_eq!(
            map_source_quantizer_to_av1(&sample_video("vp9", None), 28.0),
            26
        );
        assert_eq!(
            map_source_quantizer_to_av1(&sample_video("h264", None), 28.0),
            28
        );
        assert_eq!(
            map_source_quantizer_to_av1(&sample_video("vp8", None), 28.0),
            24
        );
        assert_eq!(
            map_source_quantizer_to_av1(&sample_video("mpeg4", None), 28.0),
            24
        );
        assert_eq!(
            map_source_quantizer_to_av1(&sample_video("mpeg2video", None), 28.0),
            22
        );
        assert_eq!(
            map_source_quantizer_to_av1(&sample_video("mpeg1video", None), 28.0),
            22
        );
    }

    #[test]
    // AI-FUNC-SUMMARY: Builds or derives maps common source codecs to hardware bitrate ratios data; returns the computed value; side effects: none.
    fn maps_common_source_codecs_to_hardware_bitrate_ratios() {
        assert_eq!(hardware_bitrate_ratio("hevc"), 0.85);
        assert_eq!(hardware_bitrate_ratio("vp9"), 0.85);
        assert_eq!(hardware_bitrate_ratio("h264"), 0.70);
        assert_eq!(hardware_bitrate_ratio("vp8"), 0.60);
        assert_eq!(hardware_bitrate_ratio("mpeg4"), 0.60);
        assert_eq!(hardware_bitrate_ratio("mpeg2video"), 0.50);
        assert_eq!(hardware_bitrate_ratio("mpeg1video"), 0.50);
        assert_eq!(hardware_bitrate_ratio("mjpeg"), 0.35);
        assert_eq!(hardware_bitrate_ratio("prores"), 0.35);
    }

    // AI-FUNC-SUMMARY: Builds a minimal work item so plan tests can ask for a quality setting; returns the item; side effects: none.
    fn plan_test_item(video: &VideoInfo) -> WorkItem {
        WorkItem {
            input_path: PathBuf::from("clip.mkv"),
            output_path: PathBuf::from("clip.mp4"),
            old_path: None,
            temp_path: PathBuf::from(".myvidcomp-clip-1.tmp.mp4"),
            recovery_path: PathBuf::from("clip.mkv.myvidcomp.recover"),
            output_container: OutputContainer::Mp4,
            video: video.clone(),
            mapped_stream_indexes: vec![0],
            stream_signature: Vec::new(),
            chapter_policy: ChapterPolicy::Ordinary,
            estimated_crf: 28,
        }
    }
    // AI-FUNC-SUMMARY: Provides sample video behavior; returns the declared result; side effects: see implementation.
    fn sample_video(codec: &str, bit_rate: Option<u64>) -> VideoInfo {
        VideoInfo {
            codec_name: codec.to_string(),
            profile: None,
            width: 1920,
            height: 1080,
            sample_aspect_ratio: Some("1:1".to_string()),
            display_aspect_ratio: Some("16:9".to_string()),
            field_order: Some("progressive".to_string()),
            fps: 30.0,
            nominal_fps: 30.0,
            avg_frame_rate: Some("30/1".to_string()),
            nominal_frame_rate: Some("30/1".to_string()),
            duration_seconds: 120.0,
            bit_rate,
            pix_fmt: Some("yuv420p".to_string()),
            bits_per_raw_sample: Some(8),
            color_range: None,
            color_space: None,
            color_transfer: None,
            color_primaries: None,
            chroma_location: None,
            encoder_tag: None,
        }
    }

    // AI-FUNC-SUMMARY: Builds concise test stream metadata with an actual index and no optional carrier fields; returns a StreamInfo fixture; side effects: none.
    fn sample_stream(index: usize, codec_type: &str, codec_name: &str) -> StreamInfo {
        StreamInfo {
            index,
            codec_type: codec_type.to_string(),
            codec_name: codec_name.to_string(),
            codec_tag_string: None,
            track_id: None,
            handler_name: None,
        }
    }

    // AI-FUNC-SUMMARY: Builds one ordinary synthetic ISO BMFF box for parser tests; returns header plus payload bytes; side effects: none.
    fn bmff_atom(kind: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(payload.len() + 8);
        bytes.extend_from_slice(&u32::try_from(payload.len() + 8).unwrap().to_be_bytes());
        bytes.extend_from_slice(kind);
        bytes.extend_from_slice(payload);
        bytes
    }

    // AI-FUNC-SUMMARY: Builds one extended-size synthetic ISO BMFF box for parser tests; returns 64-bit header plus payload bytes; side effects: none.
    fn bmff_extended_atom(kind: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(payload.len() + 16);
        bytes.extend_from_slice(&1_u32.to_be_bytes());
        bytes.extend_from_slice(kind);
        bytes.extend_from_slice(&u64::try_from(payload.len() + 16).unwrap().to_be_bytes());
        bytes.extend_from_slice(payload);
        bytes
    }

    // AI-FUNC-SUMMARY: Builds one size-zero synthetic ISO BMFF box extending to its parent end; returns header plus payload bytes; side effects: none.
    fn bmff_zero_atom(kind: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(payload.len() + 8);
        bytes.extend_from_slice(&0_u32.to_be_bytes());
        bytes.extend_from_slice(kind);
        bytes.extend_from_slice(payload);
        bytes
    }

    // AI-FUNC-SUMMARY: Builds a minimal moov/trak/tkhd/tref/chap fixture targeting one chapter track; returns synthetic file bytes using requested size variants; side effects: none.
    fn chapter_reference_fixture(target_id: u32, extended_moov: bool, zero_chap: bool) -> Vec<u8> {
        let mut tkhd_payload = vec![0_u8; 16];
        tkhd_payload[12..16].copy_from_slice(&1_u32.to_be_bytes());
        let tkhd = bmff_atom(b"tkhd", &tkhd_payload);
        let chap = if zero_chap {
            bmff_zero_atom(b"chap", &target_id.to_be_bytes())
        } else {
            bmff_atom(b"chap", &target_id.to_be_bytes())
        };
        let tref = bmff_atom(b"tref", &chap);
        let mut trak_payload = tkhd;
        trak_payload.extend_from_slice(&tref);
        let trak = bmff_atom(b"trak", &trak_payload);
        if extended_moov {
            bmff_extended_atom(b"moov", &trak)
        } else {
            bmff_atom(b"moov", &trak)
        }
    }

    // AI-FUNC-SUMMARY: Provides make temp dir behavior; returns the declared result; side effects: see implementation.
    fn make_temp_dir(name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = env::temp_dir().join(format!("myvidcomp-test-{name}-{stamp}"));
        fs::create_dir(&path).unwrap();
        path
    }

    // AI-FUNC-SUMMARY: Provides remove temp dir behavior; returns the declared result; side effects: see implementation.
    fn remove_temp_dir(path: PathBuf) {
        let _ = fs::remove_dir_all(path);
    }

    #[test]
    // AI-FUNC-SUMMARY: Validates output-format wire values parse correctly; returns test assertion result; side effects: none.
    fn parses_output_format_values() {
        assert_eq!(parse_output_format("mp4").unwrap(), OutputFormat::Mp4);
        assert_eq!(
            parse_output_format("mkv-fallback").unwrap(),
            OutputFormat::MkvFallback
        );
        assert_eq!(
            parse_output_format("MKV_Fallback").unwrap(),
            OutputFormat::MkvFallback
        );
        assert!(parse_output_format("webm").is_err());
    }

    #[test]
    // AI-FUNC-SUMMARY: Validates CLI --output-format overrides config; returns test assertion result; side effects: none.
    fn cli_output_format_overrides_config() {
        let root = make_temp_dir("cli-outfmt");
        let config = root.join("config.yaml");
        fs::write(&config, "output_format: mkv-fallback\n").unwrap();
        let cli = Cli::parse_with_default_config(
            [
                root.to_string_lossy().to_string(),
                "--output-format".to_string(),
                "mp4".to_string(),
            ],
            &config,
        )
        .unwrap();
        assert_eq!(cli.output_format, OutputFormat::Mp4);

        let cli2 =
            Cli::parse_with_default_config([root.to_string_lossy().to_string()], &config).unwrap();
        assert_eq!(cli2.output_format, OutputFormat::MkvFallback);

        remove_temp_dir(root);
    }

    #[test]
    // AI-FUNC-SUMMARY: Validates MKV fallback switches output container when streams are MP4-incompatible; returns test assertion result; side effects: creates and removes a synthetic temp file.
    fn mkv_fallback_switches_container_for_incompatible_streams() {
        let root = make_temp_dir("mkv-fallback");
        let input = root.join("clip.mkv");
        fs::write(&input, b"").unwrap();
        let video = sample_video("h264", Some(2_000_000));
        let streams = vec![
            sample_stream(0, "video", "h264"),
            sample_stream(1, "subtitle", "subrip"),
        ];

        let item = WorkItem::new(
            input.clone(),
            video,
            streams,
            Vec::new(),
            ItemPolicy {
                keep_original: true,
                tmp_dir: None,
                output_format: OutputFormat::MkvFallback,
                target_codec: TargetCodec::Av1,
                preservation: Preservation::Strict,
            },
        )
        .unwrap();
        assert_eq!(item.output_container, OutputContainer::Mkv);
        assert_eq!(item.output_path, root.join("clip.mkv"));
        assert_eq!(
            item.temp_path.extension().and_then(OsStr::to_str),
            Some("mkv")
        );

        remove_temp_dir(root);
    }

    #[test]
    // AI-FUNC-SUMMARY: Validates MKV fallback stays MP4 when streams are compatible; returns test assertion result; side effects: creates and removes a synthetic temp file.
    fn mkv_fallback_stays_mp4_for_compatible_streams() {
        let root = make_temp_dir("mkv-mp4");
        let input = root.join("clip.mkv");
        fs::write(&input, b"").unwrap();
        let video = sample_video("h264", Some(2_000_000));
        let streams = vec![
            sample_stream(0, "video", "h264"),
            sample_stream(1, "audio", "aac"),
        ];

        let item = WorkItem::new(
            input,
            video,
            streams,
            Vec::new(),
            ItemPolicy {
                keep_original: true,
                tmp_dir: None,
                output_format: OutputFormat::MkvFallback,
                target_codec: TargetCodec::Av1,
                preservation: Preservation::Strict,
            },
        )
        .unwrap();
        assert_eq!(item.output_container, OutputContainer::Mp4);
        assert_eq!(item.output_path, root.join("clip.mp4"));

        remove_temp_dir(root);
    }

    #[test]
    // AI-FUNC-SUMMARY: Validates skip reason for incompatible stream includes stream index and codec detail; returns test assertion result; side effects: creates and removes a synthetic temp file.
    fn skip_reason_includes_stream_detail() {
        let streams = vec![
            sample_stream(0, "video", "h264"),
            sample_stream(1, "subtitle", "ass"),
        ];
        let reason = mp4_incompatible_stream_reason(&streams).unwrap();
        assert!(reason.contains("stream #1"));
        assert!(reason.contains("ass"));
    }

    #[test]
    // AI-FUNC-SUMMARY: Validates explicit mappings preserve sparse actual ffprobe indexes; returns test assertion result; side effects: none.
    fn stream_mapping_uses_actual_probe_indexes() {
        let args = stream_map_args(&[2, 7, 11]);
        assert_eq!(
            args,
            ["-map", "0:2", "-map", "0:7", "-map", "0:11"]
                .into_iter()
                .map(str::to_string)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    // AI-FUNC-SUMMARY: Validates bounded atom parsing handles ordinary, extended-size, and parent-ending chapter-reference boxes; returns test assertion result; side effects: creates and removes synthetic files.
    fn parses_bmff_chapter_reference_size_variants() {
        let root = make_temp_dir("chapter-atoms");
        for (name, extended, zero) in [
            ("ordinary", false, false),
            ("extended", true, false),
            ("zero", false, true),
        ] {
            let path = root.join(format!("{name}.mp4"));
            fs::write(&path, chapter_reference_fixture(42, extended, zero)).unwrap();
            assert_eq!(
                parse_bmff_chapter_targets(&path).unwrap(),
                HashSet::from([42])
            );
        }
        remove_temp_dir(root);
    }

    #[test]
    // AI-FUNC-SUMMARY: Validates a tref-confirmed text data carrier is excluded while meaningful chapters are retained and MP4 remains selected; returns test assertion result; side effects: creates and removes a synthetic file.
    fn confirmed_chapter_carrier_is_filtered_and_stays_mp4() {
        let root = make_temp_dir("chapter-filter");
        let input = root.join("clip.mp4");
        fs::write(&input, chapter_reference_fixture(42, false, false)).unwrap();
        let mut carrier = sample_stream(9, "data", "bin_data");
        carrier.codec_tag_string = Some("text".to_string());
        carrier.track_id = Some(42);
        carrier.handler_name = Some("Chapter Track".to_string());
        let chapters = vec![ChapterInfo {
            start_seconds: 0.0,
            end_seconds: 60.0,
            title: "Opening".to_string(),
        }];
        let item = WorkItem::new(
            input,
            sample_video("h264", Some(2_000_000)),
            vec![sample_stream(3, "video", "h264"), carrier],
            chapters.clone(),
            ItemPolicy {
                keep_original: true,
                tmp_dir: None,
                output_format: OutputFormat::MkvFallback,
                target_codec: TargetCodec::Av1,
                preservation: Preservation::Strict,
            },
        )
        .unwrap();
        assert_eq!(item.output_container, OutputContainer::Mp4);
        assert_eq!(item.mapped_stream_indexes, [3]);
        assert_eq!(item.stream_signature.len(), 1);
        assert_eq!(
            item.chapter_policy,
            ChapterPolicy::ConfirmedMeaningful(chapters)
        );
        remove_temp_dir(root);
    }

    #[test]
    // AI-FUNC-SUMMARY: Validates malformed atoms around a possible carrier produce a specific safe classification failure; returns test assertion result; side effects: creates and removes a synthetic file.
    fn malformed_possible_chapter_carrier_is_safely_rejected() {
        let root = make_temp_dir("chapter-malformed");
        let input = root.join("clip.mp4");
        fs::write(&input, [0_u8, 0, 0, 20, b'm', b'o', b'o', b'v']).unwrap();
        let mut carrier = sample_stream(1, "data", "bin_data");
        carrier.codec_tag_string = Some("text".to_string());
        carrier.track_id = Some(2);
        let err = source_stream_policy(
            &input,
            vec![sample_stream(0, "video", "h264"), carrier],
            Vec::new(),
            &sample_video("h264", None),
        )
        .unwrap_err();
        assert!(err.contains("malformed or ambiguous ISO BMFF atoms"));
        remove_temp_dir(root);
    }

    #[test]
    // AI-FUNC-SUMMARY: Validates empty and unnamed full-duration chapter sets use the confirmed-empty suppression policy; returns test assertion result; side effects: none.
    fn placeholder_chapters_are_suppressed() {
        assert!(chapters_are_placeholder_or_empty(&[], 120.0));
        assert!(chapters_are_placeholder_or_empty(
            &[ChapterInfo {
                start_seconds: 0.02,
                end_seconds: 120.08,
                title: " ".to_string(),
            }],
            120.0
        ));
        assert_eq!(ChapterPolicy::ConfirmedEmpty.map_value(), "-1");
        let args = build_av1_metadata_repair_args(
            Path::new("temp.mp4"),
            Path::new("repair.mp4"),
            &sample_video("av1", None),
            "av1_metadata=chroma_sample_position=1",
            &[0],
            &ChapterPolicy::ConfirmedEmpty,
            OutputContainer::Mp4,
        );
        assert!(args.windows(2).any(|pair| pair == ["-map_chapters", "-1"]));
        assert!(chapter_validation_error(&[], &[], Path::new("output.mp4")).is_none());
    }

    #[test]
    // AI-FUNC-SUMMARY: Validates ffprobe chapter records retain start, end, and exact title metadata; returns test assertion result; side effects: none.
    fn parses_chapter_probe_metadata() {
        let chapters = parse_chapter_probe(
            "start_time=0.000000|end_time=12.500000|TAG:title=Intro\\|Setup\nstart_time=12.500000|end_time=30.000000|TAG:title=Main\n",
        )
        .unwrap();
        assert_eq!(
            chapters,
            [
                ChapterInfo {
                    start_seconds: 0.0,
                    end_seconds: 12.5,
                    title: "Intro|Setup".to_string(),
                },
                ChapterInfo {
                    start_seconds: 12.5,
                    end_seconds: 30.0,
                    title: "Main".to_string(),
                },
            ]
        );
    }

    #[test]
    // AI-FUNC-SUMMARY: Validates incomplete or reversed ffprobe chapter records fail instead of being mistaken for an empty placeholder; returns test assertion result; side effects: none.
    fn rejects_malformed_chapter_probe_metadata() {
        assert!(parse_chapter_probe("start_time=0.0|TAG:title=Missing end\n").is_err());
        assert!(parse_chapter_probe("start_time=2.0|end_time=1.0\n").is_err());
    }

    #[test]
    // AI-FUNC-SUMMARY: Validates meaningful chapter policy maps chapters in encode and repair and enforces timestamp-tolerant exact-title semantics; returns test assertion result; side effects: creates and removes a synthetic file.
    fn meaningful_chapters_drive_args_and_semantic_validation() {
        let root = make_temp_dir("chapter-meaningful");
        let input = root.join("clip.mkv");
        fs::write(&input, b"").unwrap();
        let chapters = vec![ChapterInfo {
            start_seconds: 1.0,
            end_seconds: 10.0,
            title: "Part 1".to_string(),
        }];
        let mut item = WorkItem::new(
            input,
            sample_video("h264", None),
            vec![sample_stream(2, "video", "h264")],
            Vec::new(),
            ItemPolicy {
                keep_original: true,
                tmp_dir: None,
                output_format: OutputFormat::Mp4,
                target_codec: TargetCodec::Av1,
                preservation: Preservation::Strict,
            },
        )
        .unwrap();
        item.chapter_policy = ChapterPolicy::ConfirmedMeaningful(chapters.clone());
        let encoder = Encoder {
            name: "libsvtav1".to_string(),
            kind: EncoderKind::SvtAv1,
        };
        let encode_args = build_ffmpeg_args(&item, &encoder);
        assert!(encode_args.windows(2).any(|pair| pair == ["-map", "0:2"]));
        assert!(
            encode_args
                .windows(2)
                .any(|pair| pair == ["-map_chapters", "0"])
        );
        let repair_args = build_av1_metadata_repair_args(
            Path::new("temp.mp4"),
            Path::new("repair.mp4"),
            &item.video,
            "av1_metadata=chroma_sample_position=1",
            &[4],
            &item.chapter_policy,
            OutputContainer::Mp4,
        );
        assert!(repair_args.windows(2).any(|pair| pair == ["-map", "0:4"]));
        assert!(
            repair_args
                .windows(2)
                .any(|pair| pair == ["-map_chapters", "0"])
        );
        let close = vec![ChapterInfo {
            start_seconds: 1.03,
            end_seconds: 9.97,
            title: "Part 1".to_string(),
        }];
        assert!(chapter_validation_error(&chapters, &close, Path::new("output.mp4")).is_none());
        let wrong_title = vec![ChapterInfo {
            title: "part 1".to_string(),
            ..close[0].clone()
        }];
        assert!(
            chapter_validation_error(&chapters, &wrong_title, Path::new("output.mp4")).is_some()
        );
        remove_temp_dir(root);
    }

    #[test]
    // AI-FUNC-SUMMARY: Validates ordinary bin_data remains unsafe under MKV fallback while ASS and subrip subtitle fallback remains allowed; returns test assertion result; side effects: none.
    fn matroska_fallback_rejects_data_but_accepts_text_subtitles() {
        let data = vec![
            sample_stream(0, "video", "h264"),
            sample_stream(5, "data", "bin_data"),
        ];
        let err = select_output_container(OutputFormat::MkvFallback, &data).unwrap_err();
        assert!(err.contains("stream #5"));
        assert!(err.contains("not conservatively Matroska-copy-compatible"));
        for codec in ["ass", "subrip"] {
            let streams = vec![
                sample_stream(0, "video", "h264"),
                sample_stream(6, "subtitle", codec),
            ];
            assert_eq!(
                select_output_container(OutputFormat::MkvFallback, &streams).unwrap(),
                OutputContainer::Mkv
            );
        }
    }

    #[test]
    // AI-FUNC-SUMMARY: Validates skip reason for interlaced source includes field order detail; returns test assertion result; side effects: none.
    fn skip_reason_includes_interlace_detail() {
        let mut video = sample_video("h264", Some(2_000_000));
        video.field_order = Some("tt".to_string());
        let reason = interlaced_source_reason(&video).unwrap();
        assert!(reason.contains("tt"));
        assert!(reason.contains("interlaced"));
    }

    #[test]
    // AI-FUNC-SUMMARY: Validates the conservative MP4 audio policy rejects codecs without universal static copy support; returns test assertion result; side effects: none.
    fn conservative_mp4_audio_whitelist_rejects_unsafe_codecs() {
        assert!(!is_mp4_audio_codec("pcm_s16le"));
        assert!(!is_mp4_audio_codec("dts"));
        assert!(!is_mp4_audio_codec("truehd"));
        assert!(!is_mp4_audio_codec("mp2"));
        assert!(!is_mp4_audio_codec("vorbis"));
    }

    #[test]
    // AI-FUNC-SUMMARY: Validates the conservative MP4 video policy keeps its established baseline; returns test assertion result; side effects: none.
    fn conservative_mp4_video_whitelist_rejects_unproven_codecs() {
        assert!(!is_mp4_video_codec("mpeg2video"));
        assert!(!is_mp4_video_codec("vp9"));
        assert!(!is_mp4_video_codec("h263"));
        assert!(!is_mp4_video_codec("vp8"));
    }
}
