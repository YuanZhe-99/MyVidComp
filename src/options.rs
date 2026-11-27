//! User-facing conversion options.
//!
//! Every option here follows the same shape: a small enum with a stable wire
//! value used by the CLI, the config file and the FFI, a human-readable label
//! used in terminal output, a strict parser, and a normalizer that treats a
//! blank, `auto` or `default` value as "not set".

/// Video codec the converted file is encoded to.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TargetCodec {
    /// AV1: smallest files, slowest software encoding, newest playback support.
    #[default]
    Av1,
    /// H.265 / HEVC: widest device support and the most permissive metadata handling.
    Hevc,
    /// H.266 / VVC: experimental, software only, very slow, few players.
    Vvc,
}

impl TargetCodec {
    // AI-FUNC-SUMMARY: Maps a target codec to its stable wire value; returns static label; side effects: none.
    pub fn as_str(&self) -> &'static str {
        match self {
            TargetCodec::Av1 => "av1",
            TargetCodec::Hevc => "hevc",
            TargetCodec::Vvc => "vvc",
        }
    }

    // AI-FUNC-SUMMARY: Maps a target codec to its human-readable log label; returns static label; side effects: none.
    pub fn label(&self) -> &'static str {
        match self {
            TargetCodec::Av1 => "AV1",
            TargetCodec::Hevc => "H.265 / HEVC",
            TargetCodec::Vvc => "H.266 / VVC",
        }
    }

    // AI-FUNC-SUMMARY: Reports the ffprobe codec name the output must carry; returns static codec name; side effects: none.
    pub fn ffprobe_name(&self) -> &'static str {
        match self {
            TargetCodec::Av1 => "av1",
            TargetCodec::Hevc => "hevc",
            TargetCodec::Vvc => "vvc",
        }
    }

    // AI-FUNC-SUMMARY: Reports the MP4 codec tag this codec needs for broad player support; returns a tag or none; side effects: none.
    pub fn mp4_tag(&self) -> Option<&'static str> {
        match self {
            // FFmpeg writes hev1 by default, which Apple players refuse to decode.
            TargetCodec::Hevc => Some("hvc1"),
            TargetCodec::Av1 | TargetCodec::Vvc => None,
        }
    }
}

// AI-FUNC-SUMMARY: Parses a target-codec wire value; returns the codec or a user-facing error; side effects: none.
pub fn parse_target_codec(value: &str) -> Result<TargetCodec, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "av1" | "av01" => Ok(TargetCodec::Av1),
        "hevc" | "h265" | "h.265" | "x265" => Ok(TargetCodec::Hevc),
        "vvc" | "h266" | "h.266" | "x266" => Ok(TargetCodec::Vvc),
        other => Err(format!(
            "invalid output codec: {other}. Supported values: av1, hevc, vvc"
        )),
    }
}

// AI-FUNC-SUMMARY: Normalizes an optional target-codec value; returns none for blank/auto/default or a parsed codec; side effects: none.
pub fn normalize_target_codec_choice(value: &str) -> Result<Option<TargetCodec>, String> {
    if is_unset_choice(value) {
        Ok(None)
    } else {
        parse_target_codec(value).map(Some)
    }
}

/// How strictly the output must reproduce the source.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Preservation {
    /// Only convert when everything the tool checks is reproduced exactly.
    Strict,
    /// Convert anyway when only recoverable details would change, record every
    /// change, measure quality, and keep both files for the user to compare.
    #[default]
    Flexible,
}

impl Preservation {
    // AI-FUNC-SUMMARY: Maps a preservation policy to its stable wire value; returns static label; side effects: none.
    pub fn as_str(&self) -> &'static str {
        match self {
            Preservation::Strict => "strict",
            Preservation::Flexible => "flexible",
        }
    }

    // AI-FUNC-SUMMARY: Maps a preservation policy to its human-readable log label; returns static label; side effects: none.
    pub fn label(&self) -> &'static str {
        match self {
            Preservation::Strict => "strict preservation",
            Preservation::Flexible => "flexible preservation with review",
        }
    }

    // AI-FUNC-SUMMARY: Reports whether recoverable differences may be accepted; returns true in flexible mode; side effects: none.
    pub fn allows_deviations(&self) -> bool {
        matches!(self, Preservation::Flexible)
    }
}

// AI-FUNC-SUMMARY: Parses a preservation wire value; returns the policy or a user-facing error; side effects: none.
pub fn parse_preservation(value: &str) -> Result<Preservation, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "strict" | "exact" => Ok(Preservation::Strict),
        "flexible" | "relaxed" => Ok(Preservation::Flexible),
        other => Err(format!(
            "invalid preservation setting: {other}. Supported values: strict, flexible"
        )),
    }
}

// AI-FUNC-SUMMARY: Normalizes an optional preservation value; returns none for blank/auto/default or a parsed policy; side effects: none.
pub fn normalize_preservation_choice(value: &str) -> Result<Option<Preservation>, String> {
    if is_unset_choice(value) {
        Ok(None)
    } else {
        parse_preservation(value).map(Some)
    }
}

/// How the encoder quality setting for each file is chosen.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum QualityMode {
    /// Encode short samples at several settings and pick the smallest one that
    /// still reaches the target.
    #[default]
    Search,
    /// Derive a setting from the source bitrate and codec without test encodes.
    Estimate,
    /// Use an explicit fixed CRF/QP value directly without searching or estimating.
    Fixed,
}

impl QualityMode {
    // AI-FUNC-SUMMARY: Maps a quality mode to its stable wire value; returns static label; side effects: none.
    pub fn as_str(&self) -> &'static str {
        match self {
            QualityMode::Search => "search",
            QualityMode::Estimate => "estimate",
            QualityMode::Fixed => "fixed",
        }
    }

    // AI-FUNC-SUMMARY: Maps a quality mode to its human-readable log label; returns static label; side effects: none.
    pub fn label(&self) -> &'static str {
        match self {
            QualityMode::Search => "measure and tune",
            QualityMode::Estimate => "fast estimate",
            QualityMode::Fixed => "fixed quality",
        }
    }
}

// AI-FUNC-SUMMARY: Parses a quality-mode wire value; returns the mode or a user-facing error; side effects: none.
pub fn parse_quality_mode(value: &str) -> Result<QualityMode, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "search" | "vmaf" | "measure" => Ok(QualityMode::Search),
        "estimate" | "fast" | "heuristic" => Ok(QualityMode::Estimate),
        "fixed" | "crf" | "qp" | "manual" => Ok(QualityMode::Fixed),
        other => Err(format!(
            "invalid quality mode: {other}. Supported values: search, estimate, fixed"
        )),
    }
}

// AI-FUNC-SUMMARY: Normalizes an optional quality-mode value; returns none for blank/auto/default or a parsed mode; side effects: none.
pub fn normalize_quality_mode_choice(value: &str) -> Result<Option<QualityMode>, String> {
    if is_unset_choice(value) {
        Ok(None)
    } else {
        parse_quality_mode(value).map(Some)
    }
}

/// How thoroughly the finished file is compared against its source.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum QualityCheck {
    /// Compare several short windows spread across the video. Fast and accurate.
    #[default]
    Segmented,
    /// Compare every fifth frame across the whole file.
    Sampled,
    /// Compare every frame. Accurate and slow.
    Full,
    /// Do not compare. Flexible conversions are still measured, because their
    /// review decision depends on a score.
    Off,
}

impl QualityCheck {
    // AI-FUNC-SUMMARY: Maps a quality check to its stable wire value; returns static label; side effects: none.
    pub fn as_str(&self) -> &'static str {
        match self {
            QualityCheck::Segmented => "segmented",
            QualityCheck::Full => "full",
            QualityCheck::Sampled => "sampled",
            QualityCheck::Off => "off",
        }
    }

    // AI-FUNC-SUMMARY: Maps a quality check to its human-readable log label; returns static label; side effects: none.
    pub fn label(&self) -> &'static str {
        match self {
            QualityCheck::Segmented => "segmented spot check",
            QualityCheck::Full => "check every frame",
            QualityCheck::Sampled => "check sampled frames",
            QualityCheck::Off => "no quality check",
        }
    }

    // AI-FUNC-SUMMARY: Reports how many frames to skip between measurements; returns the libvmaf n_subsample value; side effects: none.
    pub fn subsample(&self) -> u32 {
        match self {
            QualityCheck::Full => 1,
            QualityCheck::Segmented => 3,
            QualityCheck::Sampled | QualityCheck::Off => 5,
        }
    }
}

// AI-FUNC-SUMMARY: Parses a quality-check wire value; returns the setting or a user-facing error; side effects: none.
pub fn parse_quality_check(value: &str) -> Result<QualityCheck, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "segmented" | "spot" | "quick" => Ok(QualityCheck::Segmented),
        "full" | "all" | "every" => Ok(QualityCheck::Full),
        "sampled" | "sample" | "fast" => Ok(QualityCheck::Sampled),
        "off" | "none" | "no" => Ok(QualityCheck::Off),
        other => Err(format!(
            "invalid quality check: {other}. Supported values: segmented, sampled, full, off"
        )),
    }
}

// AI-FUNC-SUMMARY: Normalizes an optional quality-check value; returns none for blank/auto/default or a parsed setting; side effects: none.
pub fn normalize_quality_check_choice(value: &str) -> Result<Option<QualityCheck>, String> {
    if is_unset_choice(value) {
        Ok(None)
    } else {
        parse_quality_check(value).map(Some)
    }
}

/// Which quality metric engine is used to score finished conversions.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum QualityMetric {
    /// Netflix VMAF via `libvmaf`. Most accurate perceptual model.
    #[default]
    Vmaf,
    /// FFmpeg's built-in `xpsnr` filter. Much faster than VMAF.
    Xpsnr,
    /// Fast XPSNR gate first, verifying with VMAF only when borderline.
    Auto,
}

impl QualityMetric {
    // AI-FUNC-SUMMARY: Maps a quality metric to its stable wire value; returns static label; side effects: none.
    pub fn as_str(&self) -> &'static str {
        match self {
            QualityMetric::Vmaf => "vmaf",
            QualityMetric::Xpsnr => "xpsnr",
            QualityMetric::Auto => "auto",
        }
    }

    // AI-FUNC-SUMMARY: Maps a quality metric to its human-readable log label; returns static label; side effects: none.
    pub fn label(&self) -> &'static str {
        match self {
            QualityMetric::Vmaf => "VMAF",
            QualityMetric::Xpsnr => "XPSNR (fast)",
            QualityMetric::Auto => "Auto (XPSNR gate + VMAF verify)",
        }
    }
}

// AI-FUNC-SUMMARY: Parses a quality-metric wire value; returns the metric or a user-facing error; side effects: none.
pub fn parse_quality_metric(value: &str) -> Result<QualityMetric, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "vmaf" | "libvmaf" => Ok(QualityMetric::Vmaf),
        "xpsnr" | "fast" => Ok(QualityMetric::Xpsnr),
        "auto" | "hybrid" | "two-stage" => Ok(QualityMetric::Auto),
        other => Err(format!(
            "invalid quality metric: {other}. Supported values: vmaf, xpsnr, auto"
        )),
    }
}

// AI-FUNC-SUMMARY: Normalizes an optional quality-metric value; returns none for blank/default or a parsed metric; side effects: none.
pub fn normalize_quality_metric_choice(value: &str) -> Result<Option<QualityMetric>, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("default") {
        Ok(None)
    } else {
        parse_quality_metric(trimmed).map(Some)
    }
}

/// Speed vs compression-efficiency preset applied to software encoders.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum EncoderPreset {
    Fast,
    #[default]
    Balanced,
    Slow,
}

impl EncoderPreset {
    // AI-FUNC-SUMMARY: Maps an encoder preset to its stable wire value; returns static label; side effects: none.
    pub fn as_str(&self) -> &'static str {
        match self {
            EncoderPreset::Fast => "fast",
            EncoderPreset::Balanced => "balanced",
            EncoderPreset::Slow => "slow",
        }
    }

    // AI-FUNC-SUMMARY: Maps an encoder preset to its human-readable log label; returns static label; side effects: none.
    pub fn label(&self) -> &'static str {
        match self {
            EncoderPreset::Fast => "fast",
            EncoderPreset::Balanced => "balanced",
            EncoderPreset::Slow => "slow (smaller files)",
        }
    }
}

// AI-FUNC-SUMMARY: Parses an encoder-preset wire value; returns the preset or a user-facing error; side effects: none.
pub fn parse_encoder_preset(value: &str) -> Result<EncoderPreset, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "fast" | "faster" | "speed" => Ok(EncoderPreset::Fast),
        "balanced" | "medium" | "normal" => Ok(EncoderPreset::Balanced),
        "slow" | "slower" | "quality" | "small" => Ok(EncoderPreset::Slow),
        other => Err(format!(
            "invalid encoder preset: {other}. Supported values: fast, balanced, slow"
        )),
    }
}

// AI-FUNC-SUMMARY: Normalizes an optional encoder-preset value; returns none for blank/auto/default or a parsed preset; side effects: none.
pub fn normalize_encoder_preset_choice(value: &str) -> Result<Option<EncoderPreset>, String> {
    if is_unset_choice(value) {
        Ok(None)
    } else {
        parse_encoder_preset(value).map(Some)
    }
}

/// How non-MP4 audio streams are handled when targeting MP4 output.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AudioPolicy {
    /// Only copy audio streams verbatim. Incompatible streams skip or fall back to MKV.
    #[default]
    Copy,
    /// Transcode MP4-incompatible audio streams to AAC and record the change for review.
    TranscodeIncompatible,
}

impl AudioPolicy {
    // AI-FUNC-SUMMARY: Maps an audio policy to its stable wire value; returns static label; side effects: none.
    pub fn as_str(&self) -> &'static str {
        match self {
            AudioPolicy::Copy => "copy",
            AudioPolicy::TranscodeIncompatible => "transcode-incompatible",
        }
    }
}

// AI-FUNC-SUMMARY: Parses an audio-policy wire value; returns the policy or a user-facing error; side effects: none.
pub fn parse_audio_policy(value: &str) -> Result<AudioPolicy, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "copy" | "keep" => Ok(AudioPolicy::Copy),
        "transcode-incompatible" | "transcode_incompatible" | "aac" | "transcode" => {
            Ok(AudioPolicy::TranscodeIncompatible)
        }
        other => Err(format!(
            "invalid audio policy: {other}. Supported values: copy, transcode-incompatible"
        )),
    }
}

// AI-FUNC-SUMMARY: Normalizes an optional audio-policy value; returns none for blank/auto/default or a parsed policy; side effects: none.
pub fn normalize_audio_policy_choice(value: &str) -> Result<Option<AudioPolicy>, String> {
    if is_unset_choice(value) {
        Ok(None)
    } else {
        parse_audio_policy(value).map(Some)
    }
}

/// How interlaced source videos are handled.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum DeinterlacePolicy {
    /// Leave interlaced sources untouched (default safe behavior).
    #[default]
    Skip,
    /// Deinterlace with `bwdif` and record the change for review.
    Bwdif,
}

impl DeinterlacePolicy {
    // AI-FUNC-SUMMARY: Maps a deinterlace policy to its stable wire value; returns static label; side effects: none.
    pub fn as_str(&self) -> &'static str {
        match self {
            DeinterlacePolicy::Skip => "skip",
            DeinterlacePolicy::Bwdif => "bwdif",
        }
    }
}

// AI-FUNC-SUMMARY: Parses a deinterlace-policy wire value; returns the policy or a user-facing error; side effects: none.
pub fn parse_deinterlace_policy(value: &str) -> Result<DeinterlacePolicy, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "skip" | "off" | "none" => Ok(DeinterlacePolicy::Skip),
        "bwdif" | "deinterlace" | "on" => Ok(DeinterlacePolicy::Bwdif),
        other => Err(format!(
            "invalid deinterlace policy: {other}. Supported values: skip, bwdif"
        )),
    }
}

// AI-FUNC-SUMMARY: Normalizes an optional deinterlace-policy value; returns none for blank/auto/default or a parsed policy; side effects: none.
pub fn normalize_deinterlace_policy_choice(
    value: &str,
) -> Result<Option<DeinterlacePolicy>, String> {
    if is_unset_choice(value) {
        Ok(None)
    } else {
        parse_deinterlace_policy(value).map(Some)
    }
}

/// What happens when a finished conversion is not smaller than the source.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SizeGuard {
    /// Commit normally regardless of output size.
    #[default]
    Off,
    /// Keep both files for the user to review when the output is not smaller.
    Review,
    /// Discard the conversion and keep the original untouched when not smaller.
    Skip,
}

impl SizeGuard {
    // AI-FUNC-SUMMARY: Maps a size-guard policy to its stable wire value; returns static label; side effects: none.
    pub fn as_str(&self) -> &'static str {
        match self {
            SizeGuard::Off => "off",
            SizeGuard::Review => "review",
            SizeGuard::Skip => "skip",
        }
    }
}

// AI-FUNC-SUMMARY: Parses a size-guard wire value; returns the policy or a user-facing error; side effects: none.
pub fn parse_size_guard(value: &str) -> Result<SizeGuard, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "off" | "none" | "allow" => Ok(SizeGuard::Off),
        "review" | "keep-review" => Ok(SizeGuard::Review),
        "skip" | "discard" => Ok(SizeGuard::Skip),
        other => Err(format!(
            "invalid size guard: {other}. Supported values: off, review, skip"
        )),
    }
}

// AI-FUNC-SUMMARY: Normalizes an optional size-guard value; returns none for blank/auto/default or a parsed policy; side effects: none.
pub fn normalize_size_guard_choice(value: &str) -> Result<Option<SizeGuard>, String> {
    if is_unset_choice(value) {
        Ok(None)
    } else {
        parse_size_guard(value).map(Some)
    }
}

/// Which encoders are tried first for each file.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum EncoderPreference {
    /// Graphics-card encoders that reproduce the source exactly, then processor
    /// encoders. The safe default.
    #[default]
    Auto,
    /// Graphics-card encoders first even when one has to change the pixel
    /// format. Fastest, and the sensible default on phones.
    Gpu,
    /// Processor encoders only. Slowest, usually the smallest files.
    Cpu,
}

impl EncoderPreference {
    // AI-FUNC-SUMMARY: Maps an encoder preference to its stable wire value; returns static label; side effects: none.
    pub fn as_str(&self) -> &'static str {
        match self {
            EncoderPreference::Auto => "auto",
            EncoderPreference::Gpu => "gpu",
            EncoderPreference::Cpu => "cpu",
        }
    }

    // AI-FUNC-SUMMARY: Maps an encoder preference to its human-readable log label; returns static label; side effects: none.
    pub fn label(&self) -> &'static str {
        match self {
            EncoderPreference::Auto => "balanced (graphics card, then processor)",
            EncoderPreference::Gpu => "graphics card first",
            EncoderPreference::Cpu => "processor only",
        }
    }
}

// AI-FUNC-SUMMARY: Parses an encoder-preference wire value, including the retired conversion-mode names; returns the preference or a user-facing error; side effects: none.
pub fn parse_encoder_preference(value: &str) -> Result<EncoderPreference, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "auto" | "balanced" | "consistency" | "consistency-priority" | "consistency_priority" => {
            Ok(EncoderPreference::Auto)
        }
        "gpu" | "hardware" | "hardware-priority" | "hardware_priority" => {
            Ok(EncoderPreference::Gpu)
        }
        "cpu" | "software" => Ok(EncoderPreference::Cpu),
        other => Err(format!(
            "invalid encoder preference: {other}. Supported values: auto, gpu, cpu"
        )),
    }
}

// AI-FUNC-SUMMARY: Normalizes an optional encoder-preference value; returns none for blank/default or a parsed preference; side effects: none.
pub fn normalize_encoder_preference_choice(
    value: &str,
) -> Result<Option<EncoderPreference>, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("default") {
        Ok(None)
    } else {
        parse_encoder_preference(trimmed).map(Some)
    }
}

/// A quality target expressed as a VMAF score in hundredths, so the whole
/// options surface stays integer-only across the FFI boundary.
pub const DEFAULT_QUALITY_TARGET: u32 = 9500;
/// Default distance below the target at which a result is sent to review.
pub const DEFAULT_REVIEW_MARGIN: u32 = 200;
/// Lowest quality target the tool accepts.
pub const MIN_QUALITY_TARGET: u32 = 5000;
/// Highest quality target the tool accepts.
pub const MAX_QUALITY_TARGET: u32 = 10000;

// AI-FUNC-SUMMARY: Validates a quality target in hundredths of a VMAF point; returns the target or a user-facing error; side effects: none.
pub fn validate_quality_target(value: u32) -> Result<u32, String> {
    if (MIN_QUALITY_TARGET..=MAX_QUALITY_TARGET).contains(&value) {
        Ok(value)
    } else {
        Err(format!(
            "quality target must be between {:.0} and {:.0}",
            MIN_QUALITY_TARGET as f64 / 100.0,
            MAX_QUALITY_TARGET as f64 / 100.0
        ))
    }
}

// AI-FUNC-SUMMARY: Parses a decimal quality target such as "95" or "94.5" into hundredths; returns the value or a user-facing error; side effects: none.
pub fn parse_quality_target(value: &str) -> Result<u32, String> {
    let trimmed = value.trim();
    let parsed: f64 = trimmed
        .parse()
        .map_err(|_| format!("invalid quality target: {trimmed}"))?;
    if !parsed.is_finite() || parsed < 0.0 {
        return Err(format!("invalid quality target: {trimmed}"));
    }
    validate_quality_target((parsed * 100.0).round() as u32)
}

// AI-FUNC-SUMMARY: Formats a hundredths quality value for display; returns a short decimal string; side effects: none.
pub fn format_quality(value: u32) -> String {
    format!("{:.1}", value as f64 / 100.0)
}

// AI-FUNC-SUMMARY: Checks whether an option string means "not set"; returns true for blank, auto, or default; side effects: none.
fn is_unset_choice(value: &str) -> bool {
    let trimmed = value.trim();
    trimmed.is_empty()
        || trimmed.eq_ignore_ascii_case("auto")
        || trimmed.eq_ignore_ascii_case("default")
}

// AI-FUNC-SUMMARY: Parses a decimal VMAF point value such as "2" or "1.5" into hundredths; returns the value or a user-facing error; side effects: none.
pub fn parse_quality_points(value: &str) -> Result<u32, String> {
    let trimmed = value.trim();
    let parsed: f64 = trimmed
        .parse()
        .map_err(|_| format!("invalid quality value: {trimmed}"))?;
    if !parsed.is_finite() || !(0.0..=100.0).contains(&parsed) {
        return Err(format!("invalid quality value: {trimmed}"));
    }
    Ok((parsed * 100.0).round() as u32)
}

/// One of FFmpeg's hardware decoding methods, named the way FFmpeg names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HwAccel {
    D3d11va,
    D3d12va,
    Dxva2,
    Cuda,
    Qsv,
    Vaapi,
    VideoToolbox,
    MediaCodec,
    Vulkan,
}

impl HwAccel {
    // AI-FUNC-SUMMARY: Maps a decoding method to the name ffmpeg knows it by; returns static label; side effects: none.
    pub fn as_str(&self) -> &'static str {
        match self {
            HwAccel::D3d11va => "d3d11va",
            HwAccel::D3d12va => "d3d12va",
            HwAccel::Dxva2 => "dxva2",
            HwAccel::Cuda => "cuda",
            HwAccel::Qsv => "qsv",
            HwAccel::Vaapi => "vaapi",
            HwAccel::VideoToolbox => "videotoolbox",
            HwAccel::MediaCodec => "mediacodec",
            HwAccel::Vulkan => "vulkan",
        }
    }
}

/// Every method that can be named, in a fixed order for error messages.
pub const HW_ACCELS: [HwAccel; 9] = [
    HwAccel::D3d11va,
    HwAccel::D3d12va,
    HwAccel::Dxva2,
    HwAccel::Cuda,
    HwAccel::Qsv,
    HwAccel::Vaapi,
    HwAccel::VideoToolbox,
    HwAccel::MediaCodec,
    HwAccel::Vulkan,
];

/// Whether decoding may happen on a graphics card.
///
/// Decoding is what reads the source, both while converting it and while
/// measuring the result. Doing it on a graphics card frees the processor for
/// the encode and for the quality measurement itself, which stays on the
/// processor in every case.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum DecoderPreference {
    /// Use a graphics card when one is proven to work on this file, and the
    /// processor otherwise.
    #[default]
    Auto,
    /// Ask for a graphics card and say so when none can be used.
    Gpu,
    /// Never use a graphics card. The setting to choose when a score has to be
    /// reproducible on another machine.
    Cpu,
    /// Use exactly this method and nothing else.
    Method(HwAccel),
}

impl DecoderPreference {
    // AI-FUNC-SUMMARY: Maps a decoder preference to its stable wire value; returns the value, which for an explicit choice is the method name; side effects: none.
    pub fn as_str(&self) -> &'static str {
        match self {
            DecoderPreference::Auto => "auto",
            DecoderPreference::Gpu => "gpu",
            DecoderPreference::Cpu => "cpu",
            DecoderPreference::Method(method) => method.as_str(),
        }
    }

    // AI-FUNC-SUMMARY: Maps a decoder preference to its human-readable log label; returns the label; side effects: none.
    pub fn label(&self) -> String {
        match self {
            DecoderPreference::Auto => "a graphics card when one works".to_string(),
            DecoderPreference::Gpu => "a graphics card".to_string(),
            DecoderPreference::Cpu => "the processor".to_string(),
            DecoderPreference::Method(method) => method.as_str().to_string(),
        }
    }

    // AI-FUNC-SUMMARY: Reports whether this preference allows a graphics card at all; returns true unless decoding is pinned to the processor; side effects: none.
    pub fn allows_hardware(&self) -> bool {
        !matches!(self, DecoderPreference::Cpu)
    }
}

// AI-FUNC-SUMMARY: Parses a decoder-preference wire value; returns the preference or a user-facing error; side effects: none.
pub fn parse_decoder_preference(value: &str) -> Result<DecoderPreference, String> {
    let trimmed = value.trim().to_ascii_lowercase();
    match trimmed.as_str() {
        "auto" => return Ok(DecoderPreference::Auto),
        "gpu" | "hardware" => return Ok(DecoderPreference::Gpu),
        "cpu" | "software" | "none" => return Ok(DecoderPreference::Cpu),
        _ => {}
    }

    HW_ACCELS
        .iter()
        .find(|method| method.as_str() == trimmed)
        .map(|method| DecoderPreference::Method(*method))
        .ok_or_else(|| {
            let names = HW_ACCELS
                .iter()
                .map(|method| method.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "invalid decoder preference: {value}. Supported values: auto, gpu, cpu, {names}"
            )
        })
}

// AI-FUNC-SUMMARY: Normalizes an optional decoder-preference value; returns none for blank or default, and a parsed preference otherwise; side effects: none.
pub fn normalize_decoder_preference_choice(
    value: &str,
) -> Result<Option<DecoderPreference>, String> {
    // Unlike the codec, `auto` is a real choice here rather than "not set", so
    // only a blank or the word default means the caller said nothing.
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("default") {
        Ok(None)
    } else {
        parse_decoder_preference(trimmed).map(Some)
    }
}

#[cfg(test)]
mod decoder_tests {
    use super::*;

    #[test]
    // AI-FUNC-SUMMARY: Verifies every decoder preference parses, including each method name; returns nothing; side effects: none.
    fn parses_a_decoder_preference() {
        assert_eq!(
            parse_decoder_preference("auto"),
            Ok(DecoderPreference::Auto)
        );
        assert_eq!(
            parse_decoder_preference(" GPU "),
            Ok(DecoderPreference::Gpu)
        );
        assert_eq!(
            parse_decoder_preference("software"),
            Ok(DecoderPreference::Cpu)
        );
        for method in HW_ACCELS {
            assert_eq!(
                parse_decoder_preference(method.as_str()),
                Ok(DecoderPreference::Method(method))
            );
        }

        let error = parse_decoder_preference("h264_cuvid").unwrap_err();
        assert!(error.contains("Supported values"));
        assert!(error.contains("d3d11va"));
    }

    #[test]
    // AI-FUNC-SUMMARY: Verifies auto is a real choice here rather than the absence of one; returns nothing; side effects: none.
    fn auto_is_a_choice_not_a_blank() {
        // The codec treats `auto` as "not set"; here it means "decide per file",
        // so routing this through that helper would silently lose the setting.
        assert_eq!(normalize_decoder_preference_choice(""), Ok(None));
        assert_eq!(normalize_decoder_preference_choice("default"), Ok(None));
        assert_eq!(
            normalize_decoder_preference_choice("auto"),
            Ok(Some(DecoderPreference::Auto))
        );
    }

    #[test]
    // AI-FUNC-SUMMARY: Verifies each preference round-trips through its wire value; returns nothing; side effects: none.
    fn wire_values_round_trip() {
        let cases = [
            DecoderPreference::Auto,
            DecoderPreference::Gpu,
            DecoderPreference::Cpu,
            DecoderPreference::Method(HwAccel::Cuda),
        ];
        for case in cases {
            assert_eq!(parse_decoder_preference(case.as_str()), Ok(case));
        }
        assert!(DecoderPreference::Auto.allows_hardware());
        assert!(!DecoderPreference::Cpu.allows_hardware());
    }

    #[test]
    // AI-FUNC-SUMMARY: Verifies new v0.3.1 quality, preset, audio, deinterlace, and size-guard options parse cleanly; returns nothing; side effects: none.
    fn parses_extended_options() {
        assert_eq!(QualityCheck::default(), QualityCheck::Segmented);
        assert_eq!(
            parse_quality_check("segmented"),
            Ok(QualityCheck::Segmented)
        );
        assert_eq!(parse_quality_check("spot"), Ok(QualityCheck::Segmented));
        assert_eq!(QualityCheck::Segmented.subsample(), 3);

        assert_eq!(parse_quality_mode("fixed"), Ok(QualityMode::Fixed));
        assert_eq!(parse_quality_metric("vmaf"), Ok(QualityMetric::Vmaf));
        assert_eq!(parse_quality_metric("xpsnr"), Ok(QualityMetric::Xpsnr));
        assert_eq!(parse_quality_metric("auto"), Ok(QualityMetric::Auto));
        assert_eq!(
            normalize_quality_metric_choice("auto"),
            Ok(Some(QualityMetric::Auto))
        );
        assert_eq!(normalize_quality_metric_choice(""), Ok(None));

        assert_eq!(parse_encoder_preset("fast"), Ok(EncoderPreset::Fast));
        assert_eq!(
            parse_encoder_preset("balanced"),
            Ok(EncoderPreset::Balanced)
        );
        assert_eq!(parse_encoder_preset("slow"), Ok(EncoderPreset::Slow));

        assert_eq!(parse_audio_policy("copy"), Ok(AudioPolicy::Copy));
        assert_eq!(
            parse_audio_policy("transcode-incompatible"),
            Ok(AudioPolicy::TranscodeIncompatible)
        );
        assert_eq!(
            parse_deinterlace_policy("skip"),
            Ok(DeinterlacePolicy::Skip)
        );
        assert_eq!(
            parse_deinterlace_policy("bwdif"),
            Ok(DeinterlacePolicy::Bwdif)
        );
        assert_eq!(parse_size_guard("off"), Ok(SizeGuard::Off));
        assert_eq!(parse_size_guard("review"), Ok(SizeGuard::Review));
        assert_eq!(parse_size_guard("skip"), Ok(SizeGuard::Skip));
    }
}
