# MyVidComp 文档（简体中文）

这是 **MyVidComp**（`myvidcomp`）的简体中文文档树。MyVidComp 是一个批量视频转换器：它把一整个
文件夹的视频变小，测量每个结果与源有多接近，并且绝不删除任何它没有把握的东西。英文权威树
（`doc/en-us/`）与其并行维护；翻译与结构对齐规则见 [translation-guide.md](translation-guide.md)。

**这些文档是代码的权威描述。** 仓库根目录的 [AGENTS.md](../../AGENTS.md) 刻意只包含面向代理的
指示：工作流、编写规则、行为契约和发布流程，其余内容都指向这里。代码变更时先更新这些页面；当文档与
代码不一致时，以代码为准核实，然后修正页面。

## 目录

- [architecture.md](architecture.md) — MyVidComp 是什么、转换流水线、仓库布局、视频工具查找和安全模型。
- [options.md](options.md) — 每一项设置、它的传输值，以及可以在哪里设置。
- [quality.md](quality.md) — 如何选定画质设置，以及如何测量结果。
- [review.md](review.md) — 保留下来等用户决定的转换，以及如何处理它们。
- [codecs.md](codecs.md) — AV1、H.265 和 H.266 之间有什么不同。
- [output-format.md](output-format.md) — 容器策略、保守的 MP4 复制白名单、MKV 回退规则和跳过原因。
- [chapter-carrier.md](chapter-carrier.md) — 基于证据的 QuickTime 章节承载轨处理：ISO BMFF `tref/chap` 检测、占位章节抑制和有效章节重建。
- [encoders.md](encoders.md) — 编码器检测与排序、运行时编码检查、重试分类和 Windows 隐藏子进程创建。
- [validation.md](validation.md) — 编码后输出验证规则：流签名、视频属性、元数据、时长和章节语义。
- [safety.md](safety.md) — 临时文件、缓存临时文件复用、提交/恢复和优雅停止。
- [ffi-abi.md](ffi-abi.md) — 带版本的嵌入 ABI、它的入口点，以及事件 JSON。
- [packaging.md](packaging.md) — Windows 双架构打包、视频工具捆绑和包验证。
- [gui.md](gui.md) — 应用程序的结构、自适应布局、设置和用词规则。
- [android.md](android.md) — 手机版本、它需要什么，以及还缺什么。
- [ci.md](ci.md) — 自动运行的检查，以及如何在本地运行同样的检查。
- [version-history.md](version-history.md) — 按日期记录的行为变更及其原因。
- [translation-guide.md](translation-guide.md) — 本仓库的英译中指南和术语表。
- [functions/INDEX.md](functions/INDEX.md) — 函数索引：`src/` 中的全部声明，按行为域分组，链接到各域完整文档。

## 当前状态

Rust 核心是 `src/lib.rs`，加上几个能独立成章的部分各自的模块：`options.rs`、`codec.rs`、`vmaf.rs`、
`search.rs`、`review.rs` 和 `ffi.rs`。应用程序位于 `gui/`，用同一套源码为 Windows、macOS、Linux 和
Android 构建。

本文档树描述 0.1.1 版本：三种输出编码、用 VMAF 测量画质、带复核的平衡严格程度，以及单版本嵌入 ABI。

`gui/lib/` 的逐文件 Dart 声明页并不维护；[gui.md](gui.md) 覆盖应用程序结构，此类页面可在日后补充，
无需调整本树结构。
