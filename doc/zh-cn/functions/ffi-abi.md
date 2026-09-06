# 嵌入 ABI

应用程序驱动引擎所用的导出 C 边界。面向读者的说明见
[../ffi-abi.md](../ffi-abi.md)。

| 声明 | 种类 | 用途 |
|---|---|---|
| `on_event` | function | 通过 FFI 的 JSON 回调发送一条结构化事件，无返回值。 |
| `myvidcomp_cancellation_token_new` | function | 为 FFI 调用方分配一个取消令牌，返回一个不透明的令牌指针。 |
| `myvidcomp_cancellation_token_request_stop` | function | 在 FFI 取消令牌上请求优雅停止，无返回值。 |
| `myvidcomp_cancellation_token_free` | function | 释放一个 FFI 取消令牌，无返回值。 |
| `myvidcomp_string_free` | function | 释放本库分配的 FFI 字符串，无返回值。 |
| `myvidcomp_ffi_abi_version` | function | 报告所支持的最新 FFI ABI 版本，返回一个常量。 |
| `myvidcomp_run_blocking_v1` | function | 为 FFI 调用方运行一整趟转换，并通过 JSON 回调报告进度。 |
| `myvidcomp_review_list` | function | 列出某个文件夹中等待“留哪一份”决定的转换，返回分配好的 JSON 文本。 |
| `myvidcomp_review_resolve` | function | 对一项待复核内容执行留新、留原或两份都留的决定，成功时返回 null，否则返回分配好的错误字符串。 |
| `run_ffi_blocking` | function | 为 FFI 调用方运行共用的工作流程，返回 null 或分配好的错误字符串。 |
| `run_options_from_ffi_v1` | function | 把带版本的 FFI 运行选项转换为安全的 Rust 运行选项，返回选项或一条校验错误。 |
| `ffi_choice` | function | 读取一个可选的枚举值 C 字符串并归一化，返回解析出的值或该类型的默认值。 |
| `ffi_isize` | function | 把 FFI 传入的计数转换为平台整数，返回该计数或一条范围错误。 |
| `required_ffi_string` | function | 把一个必填的 C 字符串转换为 Rust 字符串，返回该值或校验错误。 |
| `optional_ffi_string` | function | 把一个可选的 C 字符串转换为 Rust 字符串，为空指针或空串时返回无。 |
| `ffi_bool` | function | 把 FFI 的字节布尔值转换为 Rust bool，非零时返回真。 |
| `ffi_string_ptr` | function | 把一个 Rust 字符串转换为分配好的 C 字符串指针，返回供 FFI 调用方使用的指针。 |
| `ffi_c_string` | function | 把文本转换为 C 字符串，替换掉内部的 NUL 字节，返回 CString。 |
| `json_escape` | function | 把文本转义为 JSON 字符串字面量，返回带引号的值。 |
