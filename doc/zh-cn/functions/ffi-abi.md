# 嵌入 ABI

应用程序驱动引擎所用的导出 C 边界。面向读者的说明见
[../ffi-abi.md](../ffi-abi.md)。

声明名称与用途说明直接取自源码中的 `AI-FUNC-SUMMARY` 注释，按本仓库的编写规则，这些注释以英文书写，
因此不作翻译。

| 声明 | 种类 | 用途 |
|---|---|---|
| `on_event` | function | Sends a structured event through the FFI JSON callback, returning none. |
| `myvidcomp_cancellation_token_new` | function | Allocates a cancellation token for FFI callers, returning an opaque token pointer. |
| `myvidcomp_cancellation_token_request_stop` | function | Requests graceful stop on an FFI cancellation token, returning none. |
| `myvidcomp_cancellation_token_free` | function | Frees an FFI cancellation token, returning none. |
| `myvidcomp_string_free` | function | Frees an FFI string allocated by this library, returning none. |
| `myvidcomp_ffi_abi_version` | function | Reports the newest supported FFI ABI version, returning a constant. |
| `myvidcomp_run_blocking_v1` | function | Runs a full conversion pass for FFI callers and reports progress through a JSON callback.. |
| `myvidcomp_review_list` | function | Lists conversions in a folder that are waiting for a keep-or-discard decision, returning allocated JSON text. |
| `myvidcomp_review_resolve` | function | Applies a keep-new, keep-original, or keep-both decision to one pending review, returning null on success or an allocated error string. |
| `run_ffi_blocking` | function | Runs the shared workflow for an FFI caller, returning null or an allocated error string. |
| `run_options_from_ffi_v1` | function | Converts versioned FFI run options into safe Rust run options, returning options or a validation error. |
| `ffi_choice` | function | Reads one optional enum-valued C string and normalizes it, returning the parsed value or the type default. |
| `ffi_isize` | function | Converts an FFI count into a platform integer, returning the count or a range error. |
| `required_ffi_string` | function | Converts a required C string into a Rust string, returning value or validation error. |
| `optional_ffi_string` | function | Converts an optional C string into a Rust string, returning none for null or empty values. |
| `ffi_bool` | function | Converts an FFI byte boolean into Rust bool, returning true for nonzero. |
| `ffi_string_ptr` | function | Converts a Rust string into an allocated C string pointer, returning a pointer for FFI callers. |
| `ffi_c_string` | function | Converts text into a C string, replacing interior NUL bytes, returning CString. |
| `json_escape` | function | Escapes text as a JSON string literal, returning the quoted value. |
