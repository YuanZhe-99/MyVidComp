//! Deciding whether the source can be decoded on a graphics card.
//!
//! Decoding happens twice for every file: once while converting it and once
//! more while measuring the result against it. Moving that off the processor
//! frees it for the encode and for the measurement, which always stays on the
//! processor because the filter needs frames in ordinary memory.
//!
//! Two questions have to be answered, and neither one alone is enough.
//! `ffmpeg -hwaccels` lists what the build was compiled with, not what the
//! machine can do: it names `cuda` on a computer with no NVIDIA card at all.
//! Creating the device answers whether the driver is there. Only decoding two
//! real frames answers whether this codec can be decoded, and that is the
//! question that matters, because hardware without an AV1 decoder refuses AV1
//! while handling H.265 perfectly.

use std::collections::HashMap;
use std::path::Path;
use std::process::Stdio;

use super::{DecoderPreference, HwAccel, first_error_line, process_command};

// AI-FUNC-SUMMARY:
// Purpose: Lists the decoding methods worth trying on this platform, in the order to try them.
// Inputs: None.
// Returns: The candidate methods.
// Side effects: None.
// Notes: Vulkan, D3D12 and OpenCL decoding are immature, so they are reachable only by naming one of them; an automatic choice must be dull.
pub fn candidates() -> &'static [HwAccel] {
    #[cfg(target_os = "windows")]
    {
        // d3d11va needs nothing beyond the display driver; dxva2 is its older
        // sibling; cuda and qsv need a vendor runtime that may be absent even
        // when the card is present.
        &[
            HwAccel::D3d11va,
            HwAccel::Dxva2,
            HwAccel::Cuda,
            HwAccel::Qsv,
        ]
    }
    #[cfg(target_os = "macos")]
    {
        &[HwAccel::VideoToolbox]
    }
    #[cfg(target_os = "android")]
    {
        &[HwAccel::MediaCodec]
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "android")))]
    {
        &[HwAccel::Vaapi, HwAccel::Cuda]
    }
}

// AI-FUNC-SUMMARY: Reads which decoding methods this FFmpeg build knows about; returns the methods worth trying here, in preference order; side effects: runs ffmpeg once.
pub fn detect_support(ffmpeg: &str) -> Vec<HwAccel> {
    let output = process_command(ffmpeg)
        .args(["-hide_banner", "-hwaccels"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    let Ok(output) = output else {
        return Vec::new();
    };

    let mut text = String::from_utf8_lossy(&output.stdout).to_string();
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    let advertised: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.contains(':'))
        .collect();

    candidates()
        .iter()
        .copied()
        .filter(|method| advertised.iter().any(|name| *name == method.as_str()))
        .collect()
}

// AI-FUNC-SUMMARY:
// Purpose: Checks that a decoding method's device can actually be created here.
// Inputs: The ffmpeg command and the method.
// Returns: Success, or the reason it cannot be used.
// Side effects: Runs ffmpeg once on a generated frame.
// Notes: -init_hw_device rather than -hwaccel, because a generated source is not a compressed stream and -hwaccel over one proves nothing.
pub fn check_device(ffmpeg: &str, method: HwAccel) -> Result<(), String> {
    let output = process_command(ffmpeg)
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-nostdin",
            "-init_hw_device",
            method.as_str(),
            "-f",
            "lavfi",
            "-i",
            "nullsrc=s=64x64:d=0.1",
            "-frames:v",
            "1",
            "-f",
            "null",
            "-",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .map_err(|err| format!("failed to ask ffmpeg about {}: {err}", method.as_str()))?;

    if output.status.success() {
        return Ok(());
    }

    let text = String::from_utf8_lossy(&output.stderr);
    Err(first_error_line(&text))
}

// AI-FUNC-SUMMARY:
// Purpose: Checks that a real file's codec can be decoded by a method.
// Inputs: The ffmpeg command, the method and the file.
// Returns: Success, or the reason this file cannot use it.
// Side effects: Runs ffmpeg once, decoding two frames.
// Notes: Creating the device succeeding says nothing about the codec: hardware with no AV1 decoder refuses AV1 while handling H.265 without complaint.
pub fn check_decode(ffmpeg: &str, method: HwAccel, input: &Path) -> Result<(), String> {
    let output = process_command(ffmpeg)
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-nostdin",
            "-hwaccel",
            method.as_str(),
            "-i",
            &input.to_string_lossy(),
            "-frames:v",
            "2",
            "-f",
            "null",
            "-",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .map_err(|err| format!("failed to test decoding with {}: {err}", method.as_str()))?;

    let text = String::from_utf8_lossy(&output.stderr);
    // A refused hardware decoder still exits zero because ffmpeg falls back to
    // the processor on its own, so the text has to be read as well.
    if output.status.success() && !text.contains("hwaccel initialisation returned error") {
        return Ok(());
    }

    Err(first_error_line(&text))
}

/// What a run decided about hardware decoding, and what it has learned since.
///
/// One failure is enough evidence: a method that crashed or was refused once is
/// not tried again for the rest of the run, which is what keeps a bad driver
/// from costing one wasted encode per file.
#[derive(Debug, Clone, Default)]
pub struct Decoding {
    method: Option<HwAccel>,
    /// Which source codecs the method has been proven on, and which it refused.
    checked: HashMap<String, bool>,
    /// Set once the method has failed during real work.
    disabled: bool,
    /// Whether the comparison itself can run on the card as well as the
    /// decoding. Almost never: see vmaf::detect_vmaf_cuda_support.
    scoring: bool,
}

impl Decoding {
    // AI-FUNC-SUMMARY:
    // Purpose: Chooses the decoding method for a run.
    // Inputs: The ffmpeg command and what the user asked for.
    // Returns: The decision, plus a sentence to report when the request could not be met.
    // Side effects: Runs ffmpeg to list methods and to create a device.
    // Notes: An automatic choice reports nothing when it falls back, because falling back is what it is for.
    pub fn choose(ffmpeg: &str, preference: DecoderPreference) -> (Self, Option<String>) {
        match preference {
            DecoderPreference::Cpu => (Self::default(), None),
            DecoderPreference::Method(method) => match check_device(ffmpeg, method) {
                Ok(()) => (
                    Self {
                        method: Some(method),
                        ..Self::default()
                    },
                    None,
                ),
                Err(reason) => (
                    Self::default(),
                    Some(format!(
                        "{} cannot be used on this computer, so decoding stays on the processor: {reason}",
                        method.as_str()
                    )),
                ),
            },
            DecoderPreference::Auto | DecoderPreference::Gpu => {
                let mut last_reason = None;
                for method in detect_support(ffmpeg) {
                    match check_device(ffmpeg, method) {
                        Ok(()) => {
                            return (
                                Self {
                                    method: Some(method),
                                    ..Self::default()
                                },
                                None,
                            );
                        }
                        Err(reason) => last_reason = Some(reason),
                    }
                }

                let complaint = matches!(preference, DecoderPreference::Gpu).then(|| {
                    match last_reason {
                        Some(reason) => format!(
                            "no graphics card here can decode video, so decoding stays on the processor: {reason}"
                        ),
                        None => "this FFmpeg build offers no hardware decoding, so decoding stays on the processor".to_string(),
                    }
                });
                (Self::default(), complaint)
            }
        }
    }

    // AI-FUNC-SUMMARY: Reports which method the run settled on; returns the method, or none when decoding stays on the processor; side effects: none.
    pub fn method(&self) -> Option<HwAccel> {
        if self.disabled { None } else { self.method }
    }

    // AI-FUNC-SUMMARY:
    // Purpose: Decides whether one file can be decoded on the graphics card.
    // Inputs: The ffmpeg command, the file and its codec name.
    // Returns: The method to use, or none when this file has to stay on the processor.
    // Side effects: May run ffmpeg once per codec to decode two frames.
    // Notes: The answer is remembered per codec, so a folder of one codec pays for one check.
    pub fn method_for(&mut self, ffmpeg: &str, input: &Path, codec: &str) -> Option<HwAccel> {
        let method = self.method()?;
        if let Some(usable) = self.checked.get(codec) {
            return usable.then_some(method);
        }

        let usable = check_decode(ffmpeg, method, input).is_ok();
        self.checked.insert(codec.to_string(), usable);
        usable.then_some(method)
    }

    // AI-FUNC-SUMMARY: Stops using the graphics card for the rest of the run; returns none; side effects: updates the decision.
    pub fn disable(&mut self) {
        self.disabled = true;
    }

    // AI-FUNC-SUMMARY: Records that this build can also score on the card; returns none; side effects: updates the decision.
    pub fn enable_gpu_scoring(&mut self) {
        self.scoring = true;
    }

    // AI-FUNC-SUMMARY: Reports whether the comparison itself should run on the card; returns true only while the card is still in use; side effects: none.
    pub fn scores_on_gpu(&self) -> bool {
        self.scoring && self.method() == Some(HwAccel::Cuda)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    // AI-FUNC-SUMMARY: Verifies the platform candidates are sensible and name themselves the way ffmpeg does; returns nothing; side effects: none.
    fn offers_dull_candidates_first() {
        let candidates = candidates();
        assert!(!candidates.is_empty());

        #[cfg(target_os = "windows")]
        assert_eq!(candidates.first().copied(), Some(HwAccel::D3d11va));
        #[cfg(target_os = "macos")]
        assert_eq!(candidates.first().copied(), Some(HwAccel::VideoToolbox));
        #[cfg(target_os = "android")]
        assert_eq!(candidates.first().copied(), Some(HwAccel::MediaCodec));

        // Nothing experimental is ever chosen automatically.
        assert!(!candidates.contains(&HwAccel::Vulkan));
        assert!(!candidates.contains(&HwAccel::D3d12va));

        for method in candidates {
            assert_eq!(
                super::super::parse_decoder_preference(method.as_str()),
                Ok(DecoderPreference::Method(*method))
            );
        }
    }

    #[test]
    // AI-FUNC-SUMMARY: Verifies a run that never chose a method never reports one; returns nothing; side effects: none.
    fn the_processor_is_the_default() {
        let mut decoding = Decoding::default();
        assert_eq!(decoding.method(), None);
        // Nothing is asked of ffmpeg when the processor is doing the decoding.
        assert_eq!(
            decoding.method_for("ffmpeg", Path::new("clip.mkv"), "hevc"),
            None
        );
    }

    #[test]
    // AI-FUNC-SUMMARY: Verifies one failure stops the graphics card being used again; returns nothing; side effects: none.
    fn one_failure_ends_it_for_the_run() {
        let mut decoding = Decoding {
            method: Some(HwAccel::D3d11va),
            ..Decoding::default()
        };
        assert_eq!(decoding.method(), Some(HwAccel::D3d11va));

        decoding.disable();

        assert_eq!(decoding.method(), None);
    }

    #[test]
    // AI-FUNC-SUMMARY: Verifies a codec that was refused once is not tested again; returns nothing; side effects: none.
    fn a_refused_codec_is_remembered() {
        let mut decoding = Decoding {
            method: Some(HwAccel::D3d11va),
            ..Decoding::default()
        };
        decoding.checked.insert("av1".to_string(), false);
        decoding.checked.insert("hevc".to_string(), true);

        assert_eq!(
            decoding.method_for("ffmpeg", Path::new("clip.mkv"), "av1"),
            None
        );
        assert_eq!(
            decoding.method_for("ffmpeg", Path::new("clip.mkv"), "hevc"),
            Some(HwAccel::D3d11va)
        );
    }
}
