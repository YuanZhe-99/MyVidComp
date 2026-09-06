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
    /// still reaches the quality target.
    #[default]
    Search,
    /// Derive a setting from the source bitrate and codec without test encodes.
    Estimate,
}

impl QualityMode {
    // AI-FUNC-SUMMARY: Maps a quality mode to its stable wire value; returns static label; side effects: none.
    pub fn as_str(&self) -> &'static str {
        match self {
            QualityMode::Search => "search",
            QualityMode::Estimate => "estimate",
        }
    }

    // AI-FUNC-SUMMARY: Maps a quality mode to its human-readable log label; returns static label; side effects: none.
    pub fn label(&self) -> &'static str {
        match self {
            QualityMode::Search => "measure and tune",
            QualityMode::Estimate => "fast estimate",
        }
    }
}

// AI-FUNC-SUMMARY: Parses a quality-mode wire value; returns the mode or a user-facing error; side effects: none.
pub fn parse_quality_mode(value: &str) -> Result<QualityMode, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "search" | "vmaf" | "measure" => Ok(QualityMode::Search),
        "estimate" | "fast" | "heuristic" => Ok(QualityMode::Estimate),
        other => Err(format!(
            "invalid quality mode: {other}. Supported values: search, estimate"
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
    /// Compare every frame. Accurate and slow.
    Full,
    /// Compare every fifth frame. Close enough to decide, several times faster.
    #[default]
    Sampled,
    /// Do not compare. Flexible conversions are still measured, because their
    /// review decision depends on a score.
    Off,
}

impl QualityCheck {
    // AI-FUNC-SUMMARY: Maps a quality check to its stable wire value; returns static label; side effects: none.
    pub fn as_str(&self) -> &'static str {
        match self {
            QualityCheck::Full => "full",
            QualityCheck::Sampled => "sampled",
            QualityCheck::Off => "off",
        }
    }

    // AI-FUNC-SUMMARY: Maps a quality check to its human-readable log label; returns static label; side effects: none.
    pub fn label(&self) -> &'static str {
        match self {
            QualityCheck::Full => "check every frame",
            QualityCheck::Sampled => "check sampled frames",
            QualityCheck::Off => "no quality check",
        }
    }

    // AI-FUNC-SUMMARY: Reports how many frames to skip between measurements; returns the libvmaf n_subsample value; side effects: none.
    pub fn subsample(&self) -> u32 {
        match self {
            QualityCheck::Full => 1,
            QualityCheck::Sampled | QualityCheck::Off => 5,
        }
    }
}

// AI-FUNC-SUMMARY: Parses a quality-check wire value; returns the setting or a user-facing error; side effects: none.
pub fn parse_quality_check(value: &str) -> Result<QualityCheck, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "full" | "all" | "every" => Ok(QualityCheck::Full),
        "sampled" | "sample" | "fast" => Ok(QualityCheck::Sampled),
        "off" | "none" | "no" => Ok(QualityCheck::Off),
        other => Err(format!(
            "invalid quality check: {other}. Supported values: full, sampled, off"
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
