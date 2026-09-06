//! Everything that differs between the three output codecs.
//!
//! Each codec brings its own encoders, its own way of expressing a quality
//! setting, and its own bitstream filter for carrying colour metadata through
//! the encode. Keeping that here means the conversion pipeline itself never has
//! to ask which codec it is producing.

use super::{EncoderKind, TargetCodec, VideoInfo};

/// Which family of quality knobs an encoder exposes.
///
/// Every entry maps onto one arm of the argument builders, so adding an encoder
/// is a matter of picking the family it behaves like.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum QualityStyle {
    /// A target bitrate, because the encoder has no usable constant-quality mode.
    Bitrate,
    /// A CRF value on roughly the x264 scale.
    Crf,
    /// A quantizer on the encoder's own scale.
    Quantizer,
}

// AI-FUNC-SUMMARY: Lists the encoders that can produce one codec, best first; returns the ordered candidate table; side effects: none.
pub(crate) fn encoder_candidates(codec: TargetCodec) -> &'static [(&'static str, EncoderKind)] {
    match codec {
        TargetCodec::Av1 => &[
            ("av1_videotoolbox", EncoderKind::Hardware),
            ("av1_nvenc", EncoderKind::Hardware),
            ("av1_qsv", EncoderKind::Hardware),
            ("av1_amf", EncoderKind::Hardware),
            ("av1_mf", EncoderKind::Hardware),
            ("av1_mediacodec", EncoderKind::MediaCodec),
            ("av1_vulkan", EncoderKind::Vulkan),
            ("libsvtav1", EncoderKind::SvtAv1),
            ("libaom-av1", EncoderKind::LibAom),
            ("librav1e", EncoderKind::Rav1e),
        ],
        TargetCodec::Hevc => &[
            ("hevc_videotoolbox", EncoderKind::Hardware),
            ("hevc_nvenc", EncoderKind::Hardware),
            ("hevc_qsv", EncoderKind::Hardware),
            ("hevc_amf", EncoderKind::Hardware),
            ("hevc_mf", EncoderKind::Hardware),
            ("hevc_mediacodec", EncoderKind::MediaCodec),
            ("hevc_vulkan", EncoderKind::Vulkan),
            ("libx265", EncoderKind::X265),
        ],
        // There is no hardware H.266 encoder in any FFmpeg build today.
        TargetCodec::Vvc => &[("libvvenc", EncoderKind::Vvenc)],
    }
}

// AI-FUNC-SUMMARY: Expands a short encoder alias into the full name for one codec; returns the canonical encoder name; side effects: none.
pub(crate) fn canonical_encoder_name(name: &str, codec: TargetCodec) -> String {
    let lowered = name.trim().to_ascii_lowercase();
    let prefix = match codec {
        TargetCodec::Av1 => "av1",
        TargetCodec::Hevc => "hevc",
        // VVC has only a software encoder, so no vendor alias can resolve.
        TargetCodec::Vvc => return vvc_alias(&lowered),
    };

    match lowered.as_str() {
        "videotoolbox" | "nvenc" | "qsv" | "amf" | "vulkan" | "mediacodec" => {
            format!("{prefix}_{lowered}")
        }
        "mf" | "mediafoundation" | "media-foundation" => format!("{prefix}_mf"),
        "svt" | "svt-av1" | "libsvt" => "libsvtav1".to_string(),
        "aom" | "libaom" => "libaom-av1".to_string(),
        "rav1e" => "librav1e".to_string(),
        "x265" | "libx265" | "cpu" if matches!(codec, TargetCodec::Hevc) => "libx265".to_string(),
        other => other.to_string(),
    }
}

// AI-FUNC-SUMMARY: Expands a short encoder alias for the VVC codec; returns the canonical encoder name; side effects: none.
fn vvc_alias(lowered: &str) -> String {
    match lowered {
        "vvenc" | "libvvenc" | "cpu" | "x266" => "libvvenc".to_string(),
        other => other.to_string(),
    }
}

// AI-FUNC-SUMMARY: Reports which quality knob an encoder family exposes; returns the style; side effects: none.
pub(crate) fn quality_style(kind: EncoderKind) -> QualityStyle {
    match kind {
        // Hardware encoders across every vendor either lack a constant-quality
        // mode or expose one that behaves differently per driver, so they all
        // get an explicit bitrate.
        EncoderKind::Hardware | EncoderKind::MediaCodec => QualityStyle::Bitrate,
        EncoderKind::Vulkan | EncoderKind::SvtAv1 | EncoderKind::LibAom | EncoderKind::X265 => {
            QualityStyle::Crf
        }
        EncoderKind::Rav1e | EncoderKind::Vvenc => QualityStyle::Quantizer,
    }
}

/// The lowest and highest quality value an encoder accepts, and the step a
/// search should move by. Lower always means better quality.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct QualityRange {
    pub(crate) min: f64,
    pub(crate) max: f64,
    pub(crate) step: f64,
}

// AI-FUNC-SUMMARY: Reports the searchable quality range for an encoder family; returns the range and step; side effects: none.
pub(crate) fn quality_range(kind: EncoderKind) -> QualityRange {
    match kind {
        EncoderKind::SvtAv1 | EncoderKind::LibAom | EncoderKind::Vulkan => QualityRange {
            min: 10.0,
            max: 55.0,
            step: 1.0,
        },
        EncoderKind::X265 => QualityRange {
            min: 10.0,
            max: 46.0,
            step: 0.5,
        },
        // rav1e counts quantizers from 0 to 255.
        EncoderKind::Rav1e => QualityRange {
            min: 20.0,
            max: 255.0,
            step: 5.0,
        },
        EncoderKind::Vvenc => QualityRange {
            min: 15.0,
            max: 55.0,
            step: 1.0,
        },
        // Hardware encoders are searched by scaling the source bitrate instead,
        // so this range is only a safety net.
        EncoderKind::Hardware | EncoderKind::MediaCodec => QualityRange {
            min: 10.0,
            max: 51.0,
            step: 1.0,
        },
    }
}

// AI-FUNC-SUMMARY: Converts a quality value chosen for AV1 into the equivalent for another codec; returns the converted value; side effects: none.
pub(crate) fn convert_quality(av1_crf: u8, kind: EncoderKind) -> f64 {
    // These are working conversions, not measured equivalences. They only set
    // the starting point; a quality search corrects from there, and the fast
    // estimate accepts the approximation in exchange for skipping test encodes.
    let value = match kind {
        EncoderKind::SvtAv1 | EncoderKind::LibAom | EncoderKind::Vulkan => f64::from(av1_crf),
        // x265 reaches the same quality a few points lower than SVT-AV1.
        EncoderKind::X265 => f64::from(av1_crf) * 0.8 - 1.0,
        // VVenC quantizers run coarser again than x265 CRF.
        EncoderKind::Vvenc => f64::from(av1_crf) * 0.8 + 3.0,
        // rav1e's 0-255 scale is roughly the CRF scale times five.
        EncoderKind::Rav1e => f64::from(av1_crf) * 5.0,
        EncoderKind::Hardware | EncoderKind::MediaCodec => f64::from(av1_crf),
    };

    let range = quality_range(kind);
    value.clamp(range.min, range.max)
}

// AI-FUNC-SUMMARY: Reports how much of the source bitrate a codec needs for comparable quality; returns a multiplier; side effects: none.
pub(crate) fn codec_bitrate_multiplier(codec: TargetCodec) -> f64 {
    match codec {
        TargetCodec::Av1 => 1.0,
        // HEVC needs roughly a third more bitrate than AV1 for the same result.
        TargetCodec::Hevc => 1.3,
        // Published comparisons put VVC slightly ahead of AV1.
        TargetCodec::Vvc => 0.95,
    }
}
// AI-FUNC-SUMMARY: Reports the only pixel format an encoder accepts, when it accepts just one; returns the format or none; side effects: none.
pub(crate) fn required_pixel_format(encoder: &str) -> Option<&'static str> {
    match encoder {
        // VVenC in FFmpeg is built for 10-bit 4:2:0 and rejects everything else,
        // so every source has to be converted before it can be encoded.
        "libvvenc" => Some("yuv420p10le"),
        _ => None,
    }
}

/// The bitstream filter that carries colour metadata into the encoded stream,
/// and the option names it uses.
pub(crate) struct MetadataFilter {
    /// Filter name, for example `hevc_metadata`.
    pub(crate) name: &'static str,
    pub(crate) color_primaries: &'static str,
    pub(crate) transfer: &'static str,
    pub(crate) matrix: &'static str,
    pub(crate) range: &'static str,
    /// Chroma position option, when the filter has one.
    pub(crate) chroma: Option<&'static str>,
}

// AI-FUNC-SUMMARY: Reports the metadata bitstream filter for one codec; returns the filter description or none when the codec has no usable filter; side effects: none.
pub(crate) fn metadata_filter(codec: TargetCodec) -> Option<MetadataFilter> {
    match codec {
        TargetCodec::Av1 => Some(MetadataFilter {
            name: "av1_metadata",
            color_primaries: "color_primaries",
            transfer: "transfer_characteristics",
            matrix: "matrix_coefficients",
            range: "color_range",
            chroma: Some("chroma_sample_position"),
        }),
        TargetCodec::Hevc => Some(MetadataFilter {
            // Note the British spelling and the different range option: this
            // filter is not a drop-in rename of the AV1 one.
            name: "hevc_metadata",
            color_primaries: "colour_primaries",
            transfer: "transfer_characteristics",
            matrix: "matrix_coefficients",
            range: "video_full_range_flag",
            chroma: Some("chroma_sample_loc_type"),
        }),
        // vvc_metadata exists but exposes only access-unit-delimiter handling,
        // so there is no way to state colour metadata for an H.266 stream.
        TargetCodec::Vvc => None,
    }
}

// AI-FUNC-SUMMARY: Maps an ffprobe chroma location onto the value one codec can record; returns the value or none when the codec cannot express it; side effects: none.
pub(crate) fn chroma_location_value(codec: TargetCodec, chroma_location: &str) -> Option<u32> {
    let lowered = chroma_location.trim().to_ascii_lowercase();
    match codec {
        // AV1 only distinguishes "unknown", "vertical" and "co-located", so the
        // common `center` position of MPEG-derived sources cannot be recorded.
        TargetCodec::Av1 => match lowered.as_str() {
            "left" | "vertical" => Some(1),
            "topleft" | "top-left" | "colocated" => Some(2),
            _ => None,
        },
        // HEVC records the full set the H.273 tables define, so nothing is lost.
        TargetCodec::Hevc => match lowered.as_str() {
            "left" | "vertical" => Some(0),
            "center" | "centre" => Some(1),
            "topleft" | "top-left" | "colocated" => Some(2),
            "top" => Some(3),
            "bottomleft" | "bottom-left" => Some(4),
            "bottom" => Some(5),
            _ => None,
        },
        TargetCodec::Vvc => None,
    }
}

// AI-FUNC-SUMMARY: Maps a colour range onto the value one codec records; returns the value or none; side effects: none.
pub(crate) fn color_range_value(codec: TargetCodec, color_range: &str) -> Option<u32> {
    let lowered = color_range.trim().to_ascii_lowercase();
    let full = match lowered.as_str() {
        "tv" | "mpeg" | "limited" => false,
        "pc" | "jpeg" | "full" => true,
        _ => return None,
    };

    match codec {
        // Both filters take the same 0/1 flag, they just name the option
        // differently.
        TargetCodec::Av1 | TargetCodec::Hevc => Some(u32::from(full)),
        TargetCodec::Vvc => None,
    }
}

// AI-FUNC-SUMMARY: Reports whether a source can be encoded to one codec without changing its pixel format; returns true when no conversion is needed; side effects: none.
pub(crate) fn codec_accepts_source(codec: TargetCodec, video: &VideoInfo) -> bool {
    let Some(pix_fmt) = video.pix_fmt.as_deref() else {
        return true;
    };

    match codec {
        // libvvenc takes 10-bit 4:2:0 and nothing else.
        TargetCodec::Vvc => pix_fmt == "yuv420p10le",
        // libx265 covers 8/10/12-bit 4:2:0, 4:2:2, 4:4:4, greyscale and alpha,
        // which is everything the tool is likely to meet.
        TargetCodec::Hevc => true,
        TargetCodec::Av1 => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    // AI-FUNC-SUMMARY: Verifies each codec offers the encoders that build actually ships; returns nothing; side effects: none.
    fn lists_encoders_per_codec() {
        let av1: Vec<&str> = encoder_candidates(TargetCodec::Av1)
            .iter()
            .map(|(name, _)| *name)
            .collect();
        assert!(av1.contains(&"libsvtav1"));
        assert!(av1.contains(&"av1_nvenc"));
        assert!(!av1.iter().any(|name| name.starts_with("hevc")));

        let hevc: Vec<&str> = encoder_candidates(TargetCodec::Hevc)
            .iter()
            .map(|(name, _)| *name)
            .collect();
        assert!(hevc.contains(&"libx265"));
        assert!(hevc.contains(&"hevc_nvenc"));
        assert!(!hevc.iter().any(|name| name.starts_with("av1")));

        let vvc: Vec<&str> = encoder_candidates(TargetCodec::Vvc)
            .iter()
            .map(|(name, _)| *name)
            .collect();
        assert_eq!(vvc, vec!["libvvenc"]);
    }

    #[test]
    // AI-FUNC-SUMMARY: Verifies a vendor alias resolves to the encoder for the chosen codec; returns nothing; side effects: none.
    fn resolves_aliases_per_codec() {
        assert_eq!(
            canonical_encoder_name("nvenc", TargetCodec::Av1),
            "av1_nvenc"
        );
        assert_eq!(
            canonical_encoder_name("nvenc", TargetCodec::Hevc),
            "hevc_nvenc"
        );
        assert_eq!(canonical_encoder_name("mf", TargetCodec::Hevc), "hevc_mf");
        assert_eq!(canonical_encoder_name("svt", TargetCodec::Av1), "libsvtav1");
        assert_eq!(canonical_encoder_name("x265", TargetCodec::Hevc), "libx265");
        assert_eq!(canonical_encoder_name("cpu", TargetCodec::Vvc), "libvvenc");
        assert_eq!(
            canonical_encoder_name("libsvtav1", TargetCodec::Av1),
            "libsvtav1"
        );
    }

    #[test]
    // AI-FUNC-SUMMARY: Verifies each encoder family reports the knob it actually exposes; returns nothing; side effects: none.
    fn reports_quality_styles() {
        assert_eq!(quality_style(EncoderKind::Hardware), QualityStyle::Bitrate);
        assert_eq!(
            quality_style(EncoderKind::MediaCodec),
            QualityStyle::Bitrate
        );
        assert_eq!(quality_style(EncoderKind::X265), QualityStyle::Crf);
        assert_eq!(quality_style(EncoderKind::SvtAv1), QualityStyle::Crf);
        assert_eq!(quality_style(EncoderKind::Vvenc), QualityStyle::Quantizer);
        assert_eq!(quality_style(EncoderKind::Rav1e), QualityStyle::Quantizer);
    }

    #[test]
    // AI-FUNC-SUMMARY: Verifies converted quality values stay inside each encoder range; returns nothing; side effects: none.
    fn converts_quality_between_codecs() {
        assert_eq!(convert_quality(30, EncoderKind::SvtAv1), 30.0);
        assert!((convert_quality(30, EncoderKind::X265) - 23.0).abs() < 0.001);
        assert!((convert_quality(30, EncoderKind::Vvenc) - 27.0).abs() < 0.001);

        for kind in [
            EncoderKind::SvtAv1,
            EncoderKind::X265,
            EncoderKind::Vvenc,
            EncoderKind::Rav1e,
            EncoderKind::LibAom,
        ] {
            let range = quality_range(kind);
            for crf in 12..=36u8 {
                let value = convert_quality(crf, kind);
                assert!(value >= range.min && value <= range.max);
            }
        }
    }

    #[test]
    // AI-FUNC-SUMMARY: Verifies HEVC records every chroma position and AV1 records only two; returns nothing; side effects: none.
    fn maps_chroma_locations_per_codec() {
        assert_eq!(chroma_location_value(TargetCodec::Av1, "left"), Some(1));
        assert_eq!(chroma_location_value(TargetCodec::Av1, "topleft"), Some(2));
        // The position most MPEG-derived sources report, which AV1 cannot state.
        assert_eq!(chroma_location_value(TargetCodec::Av1, "center"), None);

        assert_eq!(chroma_location_value(TargetCodec::Hevc, "left"), Some(0));
        assert_eq!(chroma_location_value(TargetCodec::Hevc, "center"), Some(1));
        assert_eq!(chroma_location_value(TargetCodec::Hevc, "topleft"), Some(2));
        assert_eq!(chroma_location_value(TargetCodec::Hevc, "top"), Some(3));
        assert_eq!(
            chroma_location_value(TargetCodec::Hevc, "bottomleft"),
            Some(4)
        );
        assert_eq!(chroma_location_value(TargetCodec::Hevc, "bottom"), Some(5));

        assert_eq!(chroma_location_value(TargetCodec::Vvc, "left"), None);
    }

    #[test]
    // AI-FUNC-SUMMARY: Verifies the metadata filter names and option names per codec; returns nothing; side effects: none.
    fn describes_metadata_filters() {
        let av1 = metadata_filter(TargetCodec::Av1).unwrap();
        assert_eq!(av1.name, "av1_metadata");
        assert_eq!(av1.range, "color_range");

        let hevc = metadata_filter(TargetCodec::Hevc).unwrap();
        assert_eq!(hevc.name, "hevc_metadata");
        assert_eq!(hevc.color_primaries, "colour_primaries");
        assert_eq!(hevc.range, "video_full_range_flag");
        assert_eq!(hevc.chroma, Some("chroma_sample_loc_type"));

        assert!(metadata_filter(TargetCodec::Vvc).is_none());
    }

    #[test]
    // AI-FUNC-SUMMARY: Verifies colour range maps to the shared zero-or-one flag; returns nothing; side effects: none.
    fn maps_color_range() {
        assert_eq!(color_range_value(TargetCodec::Av1, "tv"), Some(0));
        assert_eq!(color_range_value(TargetCodec::Av1, "pc"), Some(1));
        assert_eq!(color_range_value(TargetCodec::Hevc, "limited"), Some(0));
        assert_eq!(color_range_value(TargetCodec::Hevc, "full"), Some(1));
        assert_eq!(color_range_value(TargetCodec::Av1, "unknown"), None);
    }

    #[test]
    // AI-FUNC-SUMMARY: Verifies only VVC forces a pixel-format change; returns nothing; side effects: none.
    fn reports_forced_pixel_formats() {
        assert_eq!(required_pixel_format("libvvenc"), Some("yuv420p10le"));
        assert_eq!(required_pixel_format("libx265"), None);
        assert_eq!(required_pixel_format("libsvtav1"), None);

        let video = VideoInfo {
            pix_fmt: Some("yuv420p".to_string()),
            ..Default::default()
        };
        assert!(!codec_accepts_source(TargetCodec::Vvc, &video));
        assert!(codec_accepts_source(TargetCodec::Hevc, &video));

        let ten_bit = VideoInfo {
            pix_fmt: Some("yuv420p10le".to_string()),
            ..Default::default()
        };
        assert!(codec_accepts_source(TargetCodec::Vvc, &ten_bit));
    }

    #[test]
    // AI-FUNC-SUMMARY: Verifies the per-codec bitrate multipliers keep their documented order; returns nothing; side effects: none.
    fn orders_bitrate_multipliers() {
        assert!(
            codec_bitrate_multiplier(TargetCodec::Vvc) < codec_bitrate_multiplier(TargetCodec::Av1)
        );
        assert!(
            codec_bitrate_multiplier(TargetCodec::Av1)
                < codec_bitrate_multiplier(TargetCodec::Hevc)
        );
    }
}
