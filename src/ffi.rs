//! C ABI for embedding the converter in another application.
//!
//! The Flutter desktop and Android front ends load the shared library built
//! from this crate and call the functions below. Options cross the boundary as
//! a pointer to a versioned struct whose first two fields are always
//! `abi_version` and `struct_size`, so a newer library can tell exactly what an
//! older caller sent it.
//!
//! Extending the ABI means adding `FfiRunOptionsV2` and
//! `myvidcomp_run_blocking_v2` beside the existing pair, never growing
//! `FfiRunOptionsV1` in place.

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_void};
use std::path::PathBuf;
use std::ptr;

use super::{
    CancellationToken, Event, EventSink, Preservation, QualityCheck, QualityMode, RunOptions,
    TargetCodec, default_runtime_binary, event_json, list_reviews, normalize_encoder_choice,
    normalize_encoder_preference_choice, normalize_output_format_choice,
    normalize_preservation_choice, normalize_quality_check_choice, normalize_quality_mode_choice,
    normalize_target_codec_choice, resolve_review, review_list_json, run_with_events,
    validate_quality_target,
};

/// Newest ABI version this library exports.
pub const MYVIDCOMP_FFI_ABI_VERSION: u32 = 1;
const RUN_OPTIONS_V1_ABI_VERSION: u32 = 1;

/// Run options for ABI version 1.
///
/// Pointers come first, then 64-bit fields, then 32-bit fields, then byte
/// flags, so the layout has no surprising padding on any supported platform.
#[repr(C)]
pub struct FfiRunOptionsV1 {
    pub abi_version: u32,
    pub struct_size: u32,
    pub target_folder: *const c_char,
    pub ffmpeg: *const c_char,
    pub ffprobe: *const c_char,
    pub tmp_dir: *const c_char,
    pub encoder: *const c_char,
    pub encoder_preference: *const c_char,
    pub output_format: *const c_char,
    pub target_codec: *const c_char,
    pub preservation: *const c_char,
    pub quality_mode: *const c_char,
    pub quality_check: *const c_char,
    pub count: i64,
    /// Quality target as a VMAF score in hundredths. Zero selects the default.
    pub quality_target: u32,
    /// Distance below the target that still avoids review, in hundredths.
    /// Zero selects the default.
    pub review_margin: u32,
    /// Threads for quality measurement. Zero matches the machine.
    pub quality_threads: u32,
    pub keep_original: u8,
    pub dry_run: u8,
}

#[repr(C)]
struct FfiOptionsHeader {
    abi_version: u32,
    struct_size: u32,
}

/// Callback that receives one JSON-encoded event.
///
/// The pointer is only valid for the duration of the call. Copy the text before
/// returning, and never let a panic cross back into Rust.
pub type FfiEventCallback = extern "C" fn(*const c_char, *mut c_void);

struct FfiEventSink {
    callback: Option<FfiEventCallback>,
    user_data: *mut c_void,
}

impl EventSink for FfiEventSink {
    // AI-FUNC-SUMMARY: Sends a structured event through the FFI JSON callback; returns none; side effects: invokes foreign callback synchronously when present.
    fn on_event(&mut self, event: Event) {
        let Some(callback) = self.callback else {
            return;
        };
        let json = event_json(&event);
        let json = ffi_c_string(json);
        callback(json.as_ptr(), self.user_data);
    }
}

#[unsafe(no_mangle)]
// AI-FUNC-SUMMARY: Allocates a cancellation token for FFI callers; returns an opaque token pointer; side effects: allocates heap memory that must be freed by myvidcomp_cancellation_token_free.
pub extern "C" fn myvidcomp_cancellation_token_new() -> *mut CancellationToken {
    Box::into_raw(Box::new(CancellationToken::new()))
}

#[unsafe(no_mangle)]
#[doc = "# Safety\n`token` must be null or a valid live pointer returned by `myvidcomp_cancellation_token_new`. It may be shared with one active run."]
// AI-FUNC-SUMMARY: Requests graceful stop on an FFI cancellation token; returns none; side effects: updates shared token state when the pointer is valid.
pub unsafe extern "C" fn myvidcomp_cancellation_token_request_stop(
    token: *const CancellationToken,
) {
    if token.is_null() {
        return;
    }

    unsafe {
        (*token).request_stop();
    }
}

#[unsafe(no_mangle)]
#[doc = "# Safety\n`token` must be null or a pointer returned by `myvidcomp_cancellation_token_new` that has not already been freed. No active run may use it after this call begins."]
// AI-FUNC-SUMMARY: Frees an FFI cancellation token; returns none; side effects: releases heap memory allocated by myvidcomp_cancellation_token_new.
pub unsafe extern "C" fn myvidcomp_cancellation_token_free(token: *mut CancellationToken) {
    if token.is_null() {
        return;
    }

    unsafe {
        drop(Box::from_raw(token));
    }
}

#[unsafe(no_mangle)]
#[doc = "# Safety\n`value` must be null or a pointer this library returned as an allocated string. Each non-null pointer must be freed exactly once."]
// AI-FUNC-SUMMARY: Frees an FFI string allocated by this library; returns none; side effects: releases heap memory for non-null pointers.
pub unsafe extern "C" fn myvidcomp_string_free(value: *mut c_char) {
    if value.is_null() {
        return;
    }

    unsafe {
        drop(CString::from_raw(value));
    }
}

#[unsafe(no_mangle)]
// AI-FUNC-SUMMARY: Reports the newest supported FFI ABI version; returns a constant; side effects: none.
pub extern "C" fn myvidcomp_ffi_abi_version() -> u32 {
    MYVIDCOMP_FFI_ABI_VERSION
}

#[unsafe(no_mangle)]
#[doc = "# Safety\n`options` must be null or point to an allocation containing at least the `abi_version` and `struct_size` fields. ABI V1 requires `abi_version` 1; when `struct_size` covers `FfiRunOptionsV1`, the remaining fields must be valid. All non-null string pointers must point to valid NUL-terminated UTF-8 for the duration of the call. `token` must be null or a valid live cancellation token pointer. The callback must copy the JSON string before returning and must not unwind across the FFI boundary."]
// AI-FUNC-SUMMARY:
// Purpose: Runs a full conversion pass for FFI callers and reports progress through a JSON callback.
// Inputs: Pointer to versioned run options, optional event callback and user data, and optional cancellation token pointer.
// Returns: Null on success, or an allocated error string that must be freed with myvidcomp_string_free.
// Side effects: Scans files, starts ffmpeg/ffprobe tools, writes temporary/output files, and invokes the callback synchronously during the run.
// Notes: Blank option strings select each option's default, so a caller may zero the struct and only fill in what it cares about.
pub unsafe extern "C" fn myvidcomp_run_blocking_v1(
    options: *const FfiRunOptionsV1,
    callback: Option<FfiEventCallback>,
    user_data: *mut c_void,
    token: *const CancellationToken,
) -> *mut c_char {
    let options = match unsafe { run_options_from_ffi_v1(options) } {
        Ok(options) => options,
        Err(err) => return ffi_string_ptr(err),
    };

    run_ffi_blocking(options, callback, user_data, token)
}

#[unsafe(no_mangle)]
#[doc = "# Safety\n`folder` must be a valid NUL-terminated UTF-8 pointer for the duration of the call. The returned pointer must be freed with myvidcomp_string_free."]
// AI-FUNC-SUMMARY: Lists conversions in a folder that are waiting for a keep-or-discard decision; returns allocated JSON text; side effects: reads the folder and its review sidecar files.
pub unsafe extern "C" fn myvidcomp_review_list(folder: *const c_char) -> *mut c_char {
    let folder = match unsafe { required_ffi_string(folder, "folder") } {
        Ok(value) => PathBuf::from(value),
        Err(err) => return ffi_string_ptr(format!("{{\"error\":{}}}", json_escape(&err))),
    };

    ffi_string_ptr(review_list_json(&list_reviews(&folder)))
}

#[unsafe(no_mangle)]
#[doc = "# Safety\n`sidecar` and `decision` must be valid NUL-terminated UTF-8 pointers for the duration of the call. Any returned pointer must be freed with myvidcomp_string_free."]
// AI-FUNC-SUMMARY: Applies a keep-new, keep-original, or keep-both decision to one pending review; returns null on success or an allocated error string; side effects: renames or deletes the reviewed files and their sidecar.
pub unsafe extern "C" fn myvidcomp_review_resolve(
    sidecar: *const c_char,
    decision: *const c_char,
) -> *mut c_char {
    let sidecar = match unsafe { required_ffi_string(sidecar, "sidecar") } {
        Ok(value) => PathBuf::from(value),
        Err(err) => return ffi_string_ptr(err),
    };
    let decision = match unsafe { required_ffi_string(decision, "decision") } {
        Ok(value) => value,
        Err(err) => return ffi_string_ptr(err),
    };

    match resolve_review(&sidecar, &decision) {
        Ok(()) => ptr::null_mut(),
        Err(err) => ffi_string_ptr(err),
    }
}

// AI-FUNC-SUMMARY: Runs the shared workflow for an FFI caller; returns null or an allocated error string; side effects: performs the whole conversion run and invokes the callback.
fn run_ffi_blocking(
    options: RunOptions,
    callback: Option<FfiEventCallback>,
    user_data: *mut c_void,
    token: *const CancellationToken,
) -> *mut c_char {
    let cancellation = if token.is_null() {
        CancellationToken::new()
    } else {
        unsafe { (*token).clone() }
    };
    let mut events = FfiEventSink {
        callback,
        user_data,
    };

    match run_with_events(options, &mut events, cancellation) {
        Ok(_) => ptr::null_mut(),
        Err(err) => ffi_string_ptr(err),
    }
}

// AI-FUNC-SUMMARY: Converts versioned FFI run options into safe Rust run options; returns options or a validation error; side effects: reads the versioned struct and C strings through raw pointers.
unsafe fn run_options_from_ffi_v1(options: *const FfiRunOptionsV1) -> Result<RunOptions, String> {
    if options.is_null() {
        return Err("options pointer is required".to_string());
    }

    let header = unsafe { &*options.cast::<FfiOptionsHeader>() };
    if header.abi_version != RUN_OPTIONS_V1_ABI_VERSION {
        return Err(format!(
            "unsupported FFI ABI version {}; myvidcomp_run_blocking_v1 requires version {RUN_OPTIONS_V1_ABI_VERSION}",
            header.abi_version
        ));
    }

    let expected_size = size_of::<FfiRunOptionsV1>() as u32;
    if header.struct_size < expected_size {
        return Err(format!(
            "FFI options struct is too small: {} < {expected_size}",
            header.struct_size
        ));
    }

    let options = unsafe { &*options };
    let defaults = RunOptions::default();

    let quality_target = if options.quality_target == 0 {
        defaults.quality_target
    } else {
        validate_quality_target(options.quality_target)?
    };
    let review_margin = if options.review_margin == 0 {
        defaults.review_margin
    } else {
        options.review_margin
    };

    Ok(RunOptions {
        target_folder: PathBuf::from(unsafe {
            required_ffi_string(options.target_folder, "target_folder")?
        }),
        count: ffi_isize(options.count)?,
        keep_original: ffi_bool(options.keep_original),
        dry_run: ffi_bool(options.dry_run),
        ffmpeg: unsafe { optional_ffi_string(options.ffmpeg, "ffmpeg") }?
            .unwrap_or_else(|| default_runtime_binary("ffmpeg")),
        ffprobe: unsafe { optional_ffi_string(options.ffprobe, "ffprobe") }?
            .unwrap_or_else(|| default_runtime_binary("ffprobe")),
        tmp_dir: unsafe { optional_ffi_string(options.tmp_dir, "tmp_dir") }?.map(PathBuf::from),
        encoder: unsafe { optional_ffi_string(options.encoder, "encoder") }?
            .and_then(|value| normalize_encoder_choice(&value)),
        encoder_preference: unsafe {
            ffi_choice(
                options.encoder_preference,
                "encoder_preference",
                normalize_encoder_preference_choice,
            )
        }?,
        output_format: unsafe {
            ffi_choice(
                options.output_format,
                "output_format",
                normalize_output_format_choice,
            )
        }?,
        target_codec: unsafe {
            ffi_choice::<TargetCodec>(
                options.target_codec,
                "target_codec",
                normalize_target_codec_choice,
            )
        }?,
        preservation: unsafe {
            ffi_choice::<Preservation>(
                options.preservation,
                "preservation",
                normalize_preservation_choice,
            )
        }?,
        quality_mode: unsafe {
            ffi_choice::<QualityMode>(
                options.quality_mode,
                "quality_mode",
                normalize_quality_mode_choice,
            )
        }?,
        quality_check: unsafe {
            ffi_choice::<QualityCheck>(
                options.quality_check,
                "quality_check",
                normalize_quality_check_choice,
            )
        }?,
        quality_target,
        review_margin,
        quality_threads: options.quality_threads,
    })
}

// AI-FUNC-SUMMARY: Reads one optional enum-valued C string and normalizes it; returns the parsed value or the type default; side effects: reads through a raw pointer when non-null.
unsafe fn ffi_choice<T: Default>(
    value: *const c_char,
    name: &str,
    normalize: fn(&str) -> Result<Option<T>, String>,
) -> Result<T, String> {
    let text = unsafe { optional_ffi_string(value, name) }?;
    match text {
        Some(text) => Ok(normalize(&text)?.unwrap_or_default()),
        None => Ok(T::default()),
    }
}

// AI-FUNC-SUMMARY: Converts an FFI count into a platform integer; returns the count or a range error; side effects: none.
fn ffi_isize(count: i64) -> Result<isize, String> {
    isize::try_from(count).map_err(|_| format!("count is out of range for this platform: {count}"))
}

// AI-FUNC-SUMMARY: Converts a required C string into a Rust string; returns value or validation error; side effects: reads through a raw pointer.
unsafe fn required_ffi_string(value: *const c_char, name: &str) -> Result<String, String> {
    let value = unsafe { optional_ffi_string(value, name) }?;
    value.ok_or_else(|| format!("{name} is required"))
}

// AI-FUNC-SUMMARY: Converts an optional C string into a Rust string; returns none for null or empty values; side effects: reads through a raw pointer when non-null.
unsafe fn optional_ffi_string(value: *const c_char, name: &str) -> Result<Option<String>, String> {
    if value.is_null() {
        return Ok(None);
    }

    let text = unsafe { CStr::from_ptr(value) }
        .to_str()
        .map_err(|err| format!("{name} is not valid UTF-8: {err}"))?
        .to_string();
    if text.is_empty() {
        Ok(None)
    } else {
        Ok(Some(text))
    }
}

// AI-FUNC-SUMMARY: Converts an FFI byte boolean into Rust bool; returns true for nonzero; side effects: none.
fn ffi_bool(value: u8) -> bool {
    value != 0
}

// AI-FUNC-SUMMARY: Converts a Rust string into an allocated C string pointer; returns a pointer for FFI callers; side effects: allocates heap memory.
fn ffi_string_ptr(value: String) -> *mut c_char {
    ffi_c_string(value).into_raw()
}

// AI-FUNC-SUMMARY: Converts text into a C string, replacing interior NUL bytes; returns CString; side effects: none.
fn ffi_c_string(value: String) -> CString {
    CString::new(value.replace('\0', "\\0")).expect("interior NUL bytes were replaced")
}

// AI-FUNC-SUMMARY: Escapes text as a JSON string literal; returns the quoted value; side effects: none.
fn json_escape(value: &str) -> String {
    super::json_string(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    // AI-FUNC-SUMMARY: Builds a zeroed options struct for tests; returns the struct with only the folder set; side effects: none.
    fn base_options(folder: &CString) -> FfiRunOptionsV1 {
        FfiRunOptionsV1 {
            abi_version: RUN_OPTIONS_V1_ABI_VERSION,
            struct_size: size_of::<FfiRunOptionsV1>() as u32,
            target_folder: folder.as_ptr(),
            ffmpeg: ptr::null(),
            ffprobe: ptr::null(),
            tmp_dir: ptr::null(),
            encoder: ptr::null(),
            encoder_preference: ptr::null(),
            output_format: ptr::null(),
            target_codec: ptr::null(),
            preservation: ptr::null(),
            quality_mode: ptr::null(),
            quality_check: ptr::null(),
            count: -1,
            quality_target: 0,
            review_margin: 0,
            quality_threads: 0,
            keep_original: 1,
            dry_run: 0,
        }
    }

    #[test]
    // AI-FUNC-SUMMARY: Verifies that a struct with only the folder set produces the documented defaults; returns nothing; side effects: none.
    fn blank_options_fall_back_to_defaults() {
        let folder = CString::new("/videos").unwrap();
        let options = base_options(&folder);

        let converted = unsafe { run_options_from_ffi_v1(&raw const options) }.unwrap();

        assert_eq!(converted.target_folder, PathBuf::from("/videos"));
        assert_eq!(converted.count, -1);
        assert!(converted.keep_original);
        assert!(!converted.dry_run);
        assert_eq!(converted.target_codec, TargetCodec::Av1);
        assert_eq!(converted.preservation, Preservation::Flexible);
        assert_eq!(converted.quality_mode, QualityMode::Search);
        assert_eq!(converted.quality_check, QualityCheck::Sampled);
        assert_eq!(
            converted.quality_target,
            RunOptions::default().quality_target
        );
        assert_eq!(converted.review_margin, RunOptions::default().review_margin);
    }

    #[test]
    // AI-FUNC-SUMMARY: Verifies that every enum-valued string is parsed into the matching option; returns nothing; side effects: none.
    fn reads_every_option_string() {
        let folder = CString::new("/videos").unwrap();
        let codec = CString::new("hevc").unwrap();
        let preservation = CString::new("strict").unwrap();
        let quality_mode = CString::new("estimate").unwrap();
        let quality_check = CString::new("full").unwrap();
        let preference = CString::new("gpu").unwrap();
        let format = CString::new("mkv-fallback").unwrap();

        let mut options = base_options(&folder);
        options.target_codec = codec.as_ptr();
        options.preservation = preservation.as_ptr();
        options.quality_mode = quality_mode.as_ptr();
        options.quality_check = quality_check.as_ptr();
        options.encoder_preference = preference.as_ptr();
        options.output_format = format.as_ptr();
        options.quality_target = 9300;
        options.review_margin = 150;

        let converted = unsafe { run_options_from_ffi_v1(&raw const options) }.unwrap();

        assert_eq!(converted.target_codec, TargetCodec::Hevc);
        assert_eq!(converted.preservation, Preservation::Strict);
        assert_eq!(converted.quality_mode, QualityMode::Estimate);
        assert_eq!(converted.quality_check, QualityCheck::Full);
        assert_eq!(converted.quality_target, 9300);
        assert_eq!(converted.review_threshold(), 9150);
    }

    #[test]
    // AI-FUNC-SUMMARY: Verifies the four ways a caller can supply an unusable options pointer; returns nothing; side effects: none.
    fn rejects_malformed_options() {
        let folder = CString::new("/videos").unwrap();

        let err = unsafe { run_options_from_ffi_v1(ptr::null()) }.unwrap_err();
        assert!(err.contains("options pointer is required"));

        let mut bad_version = base_options(&folder);
        bad_version.abi_version = 99;
        let err = unsafe { run_options_from_ffi_v1(&raw const bad_version) }.unwrap_err();
        assert!(err.contains("unsupported FFI ABI version"));

        let mut bad_size = base_options(&folder);
        bad_size.struct_size = 8;
        let err = unsafe { run_options_from_ffi_v1(&raw const bad_size) }.unwrap_err();
        assert!(err.contains("too small"));

        let bad_codec = CString::new("h264").unwrap();
        let mut invalid = base_options(&folder);
        invalid.target_codec = bad_codec.as_ptr();
        let err = unsafe { run_options_from_ffi_v1(&raw const invalid) }.unwrap_err();
        assert!(err.contains("invalid output codec"));
    }

    #[test]
    // AI-FUNC-SUMMARY: Verifies the exported ABI version constant matches the accessor; returns nothing; side effects: none.
    fn reports_abi_version() {
        assert_eq!(myvidcomp_ffi_abi_version(), MYVIDCOMP_FFI_ABI_VERSION);
        assert_eq!(MYVIDCOMP_FFI_ABI_VERSION, 1);
    }
}
