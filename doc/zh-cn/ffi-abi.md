# 嵌入 ABI

导出的 C ABI 是应用程序驱动转换引擎所用的边界。它带版本：新增字段需要新的带版本结构体和入口点，
绝不在原有结构体上就地扩展。

## 版本链

| 版本 | 结构体 | 入口点 |
|---|---|---|
| 1 | `FfiRunOptionsV1`（按指针） | `myvidcomp_run_blocking_v1` |
| 2（当前） | `FfiRunOptionsV2`（按指针） | `myvidcomp_run_blocking_v2` |

版本 1 是本名称下的第一个版本。上一个项目导出过三个结构体版本；它们已经不复存在，那个永远无法安全
扩展的按值结构体也一并移除。

版本 2 增加了 `decoder_preference`，并且整体嵌入版本 1 而不是重复它的字段，这样两种布局不会走样。
仍然使用版本 1 的调用方照常工作，并取得默认值：在确认显卡可用于该文件时使用显卡。

规则：

- 每个带版本的结构体都把 `abi_version` 固定为自己的版本号，并携带 `struct_size`。
- `FfiRunOptionsV1` 绝不能新增字段。增加选项意味着在它旁边新增 `FfiRunOptionsV2` 和
  `myvidcomp_run_blocking_v2`。
- `myvidcomp_ffi_abi_version` 报告支持的最新版本。
- 入口点会拒绝不匹配的 `abi_version`，以及小于预期的 `struct_size`。较大的值会被接受，因此按更新
  的头文件构建的调用方仍然可用。
- 每个选项字符串都可以为空，表示「使用默认值」。调用方可以把结构体清零，只填入文件夹，就能得到合理
  的行为。

## 字段顺序

先是指针，然后是 64 位计数，接着是 32 位数值，最后是字节标志。这样在所有受支持的平台上都不会出现
意外的填充，Dart 侧的镜像也按同样的顺序声明。

本来会带小数的数值一律以整数携带：画质值以百分之一 VMAF 分为单位，因此 9500 表示 95.0。结构体中
没有任何浮点字段。

## 入口点

| 函数 | 用途 |
|---|---|
| `myvidcomp_ffi_abi_version` | 报告支持的最新版本 |
| `myvidcomp_run_blocking_v1` | 运行一次转换并报告事件 |
| `myvidcomp_run_blocking_v2` | 同上，供同时选择解码方式的调用方使用 |
| `myvidcomp_cancellation_token_new` | 分配一个取消令牌 |
| `myvidcomp_cancellation_token_request_stop` | 请求优雅停止 |
| `myvidcomp_cancellation_token_free` | 释放取消令牌 |
| `myvidcomp_string_free` | 释放库返回的字符串 |
| `myvidcomp_review_list` | 列出等待决定的转换 |
| `myvidcomp_review_resolve` | 对一项待决定的转换应用一个决定 |

运行入口点返回空指针表示成功；非空的错误字符串必须用 `myvidcomp_string_free` 释放。所有字符串指针
都是以 NUL 结尾的 UTF-8，在调用期间有效。回调必须在返回前复制 JSON 事件字符串，并且不得让 panic
穿过该边界。

## 事件 JSON

事件以带 `type` 字段的紧凑 JSON 字符串传递。类型如下：

| 类型 | 发送时机 |
|---|---|
| `log` | 有值得展示的消息 |
| `settings_selected` | 一次，在处理任何文件之前 |
| `capability_missing` | 缺少某项所需能力，已退回其他方式 |
| `scan_started`、`scan_finished` | 文件夹扫描开始和结束 |
| `dry_run` | 预览运行列出它会做什么 |
| `encoder_candidate_evaluated`、`encoder_selected` | 编码器检测期间 |
| `decoder_selected` | 一次，说明本次运行采用的解码方式 |
| `file_skipped` | 某个文件保持原样，并附上原因 |
| `file_started`、`file_attempt`、`file_progress` | 某个文件转换期间 |
| `phase` | 文件的某个阶段开始，并给出它在该阶段最坏情况中的第几步 |
| `run_progress` | 整个运行的进度，跳过的文件也计入 |
| `quality_search` | 调整搜索的一次试编码 |
| `quality_measured` | 完成的转换已与源做过比较 |
| `copy_progress`、`stage` | 耗时较长的非编码步骤期间 |
| `review_pending` | 某次转换被保留下来等用户决定 |
| `file_finished` | 一个文件处理完毕 |
| `stop_requested` | 优雅停止已被接受 |
| `summary` | 本次运行的汇总 |

事件内的取值都是稳定标识符（`hevc`、`flexible`、`mkv-fallback`、编码器名称）。所有给人看的文字都
在应用程序一侧生成，因此引擎无需知道界面用的是哪种语言。

无法识别的类型会被忽略而不是当作错误，因此较新的引擎搭配较旧的界面仍然可用。

声明清单见 [functions/ffi-abi.md](functions/ffi-abi.md)。
