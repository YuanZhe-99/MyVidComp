# Embedding ABI

The exported C boundary the application drives the engine through. Described for readers in
[../ffi-abi.md](../ffi-abi.md).

| Declaration | Kind | Purpose |
|---|---|---|
| `on_event` | function | Sends a structured event through the FFI JSON callback, returning none. |
| `myvidcomp_cancellation_token_new` | function | Allocates a cancellation token for FFI callers, returning an opaque token pointer. |
| `myvidcomp_cancellation_token_request_stop` | function | Requests graceful stop on an FFI cancellation token, returning none. |
| `myvidcomp_cancellation_token_free` | function | Frees an FFI cancellation token, returning none. |
| `myvidcomp_string_free` | function | Frees an FFI string allocated by this library, returning none. |
| `myvidcomp_ffi_abi_version` | function | Reports the newest supported FFI ABI version, returning a constant. |
| `myvidcomp_run_blocking_v1` | function | Runs a full conversion pass for FFI callers and reports progress through a JSON callback.. |
| `myvidcomp_run_blocking_v2` | function | Runs a full conversion pass for FFI callers accepting V2 decoder preference options, returning null or an error string. |
| `myvidcomp_run_blocking_v3` | function | Runs a full conversion pass for FFI callers accepting V3 preset, metric, audio, deinterlace, size-guard, output-dir, and recursion options. |
| `myvidcomp_review_list` | function | Lists conversions in a folder that are waiting for a keep-or-discard decision, returning allocated JSON text. |
| `myvidcomp_review_resolve` | function | Applies a keep-new, keep-original, or keep-both decision to one pending review, returning null on success or an allocated error string. |
| `myvidcomp_list_encoders` | function | Probes usable runtime encoders for a target codec and returns allocated JSON descriptors. |
| `myvidcomp_preview_probe` | function | Probes a video for preview metadata and returns allocated JSON. |
| `myvidcomp_preview_frame_pair` | function | Extracts synchronized original/converted PNG frames and optional diff heatmap, returning allocated JSON paths. |
| `myvidcomp_preview_clip_sequence` | function | Extracts short synchronized JPEG playback bursts for in-app comparison, returning allocated JSON. |
| `myvidcomp_preview_export_clip` | function | Exports a 6-second side-by-side MP4 comparison clip, returning null or an error string. |
| `run_ffi_blocking` | function | Runs the shared workflow for an FFI caller, returning null or an allocated error string; a panic in the engine comes back as an error instead of aborting the host. |
| `guard_ffi` | function | Runs one FFI body so that a panic becomes an ordinary error, returning the body's value or the error built from the panic message; used by the run, review-list and review-resolve exports. |
| `panic_message` | function | Reads the text out of a panic payload, returning the message or a fixed phrase when the payload is not text. |
| `run_options_from_ffi_v1` | function | Converts versioned FFI run options into safe Rust run options, returning options or a validation error. |
| `run_options_from_ffi_v1_fields` | function | Converts borrowed V1 option fields into Rust `RunOptions`, returning options or a validation error. |
| `run_options_from_ffi_v2` | function | Converts V2 FFI run options into Rust `RunOptions`, returning options or a validation error. |
| `run_options_from_ffi_v2_fields` | function | Converts borrowed V2 option fields including decoder preference into Rust `RunOptions`, returning options or a validation error. |
| `run_options_from_ffi_v3` | function | Converts V3 FFI run options including extended container, speed, metric, and guard policies into Rust `RunOptions`. |
| `ffi_choice` | function | Reads one optional enum-valued C string and normalizes it, returning the parsed value or the type default. |
| `ffi_isize` | function | Converts an FFI count into a platform integer, returning the count or a range error. |
| `required_ffi_string` | function | Converts a required C string into a Rust string, returning value or validation error. |
| `optional_ffi_string` | function | Converts an optional C string into a Rust string, returning none for null or empty values. |
| `ffi_bool` | function | Converts an FFI byte boolean into Rust bool, returning true for nonzero. |
| `ffi_string_ptr` | function | Converts a Rust string into an allocated C string pointer, returning a pointer for FFI callers. |
| `ffi_c_string` | function | Converts text into a C string, replacing interior NUL bytes, returning CString. |
| `json_escape` | function | Escapes text as a JSON string literal, returning the quoted value. |
