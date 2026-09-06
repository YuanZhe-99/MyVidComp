# Embedding ABI

The exported C ABI is the boundary the application uses to drive the conversion engine. It is
versioned: new fields require a new versioned struct and entry point, never in-place growth of an
existing one.

## Version chain

| Version | Struct | Entry point |
|---|---|---|
| 1 (current) | `FfiRunOptionsV1` (by pointer) | `myvidcomp_run_blocking_v1` |

Version 1 is the first version under this name. The previous project exported three struct
versions; those are gone, along with the by-value struct that could never be extended safely.

Rules:

- Each versioned struct fixes `abi_version` to its version number and carries `struct_size`.
- `FfiRunOptionsV1` must never gain fields. Adding an option means adding `FfiRunOptionsV2` and
  `myvidcomp_run_blocking_v2` beside it.
- `myvidcomp_ffi_abi_version` reports the newest supported version.
- The entry point rejects a mismatched `abi_version`, and a `struct_size` smaller than expected. A
  larger one is accepted, so a caller built against a newer header still works.
- Every option string may be blank, meaning "use the default". A caller can zero the struct, fill
  in the folder, and get sensible behaviour.

## Field order

Pointers first, then the 64-bit count, then the 32-bit numbers, then the byte flags. That keeps
the layout free of surprising padding on every supported platform, and it is the order the Dart
mirror declares.

Numbers that would otherwise be fractional are carried as integers: quality values are in
hundredths of a VMAF point, so 9500 means 95.0. The struct contains no floating-point fields.

## Entry points

| Function | Purpose |
|---|---|
| `myvidcomp_ffi_abi_version` | reports the newest supported version |
| `myvidcomp_run_blocking_v1` | runs one conversion pass and reports events |
| `myvidcomp_cancellation_token_new` | allocates a cancellation token |
| `myvidcomp_cancellation_token_request_stop` | requests a graceful stop |
| `myvidcomp_cancellation_token_free` | frees a cancellation token |
| `myvidcomp_string_free` | frees a string the library returned |
| `myvidcomp_review_list` | lists conversions waiting for a decision |
| `myvidcomp_review_resolve` | applies one decision to one pending conversion |

A null return from a run entry point means success; a non-null error string must be released with
`myvidcomp_string_free`. All string pointers are NUL-terminated UTF-8 valid for the duration of the
call. The callback must copy the JSON event string before returning and must not unwind back
across the boundary.

## Event JSON

Events arrive as compact JSON strings with a `type` field. The types are:

| Type | Sent when |
|---|---|
| `log` | a message worth showing |
| `settings_selected` | once, before any file is touched |
| `capability_missing` | a needed capability is absent and something fell back |
| `scan_started`, `scan_finished` | the folder scan begins and ends |
| `dry_run` | a preview run lists what it would do |
| `encoder_candidate_evaluated`, `encoder_selected` | during encoder detection |
| `file_skipped` | a file was left alone, with the reason |
| `file_started`, `file_attempt`, `file_progress` | during one file's conversion |
| `quality_search` | one trial of a tuning search |
| `quality_measured` | a finished conversion was compared with its source |
| `copy_progress`, `stage` | during long non-encoding steps |
| `review_pending` | a conversion was kept for the user to decide about |
| `file_finished` | one file is done |
| `stop_requested` | a graceful stop was accepted |
| `summary` | the run's totals |

Values inside events are stable identifiers (`hevc`, `flexible`, `mkv-fallback`, encoder names).
Everything a person reads is produced on the application side, so the engine never has to know
what language the interface is in.

An unrecognised type is ignored rather than treated as an error, so a newer engine paired with an
older interface still works.

See [functions/ffi-abi.md](functions/ffi-abi.md) for the declaration inventory.
