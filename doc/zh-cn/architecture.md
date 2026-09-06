# 架构

## MyVidComp 是什么

MyVidComp（`myvidcomp`）是一个 Rust 命令行工具和可嵌入引擎：它把一整个文件夹的视频变小，测量每个
结果与源有多接近，并且绝不删除任何它没有把握的东西。`gui/` 下的应用程序内嵌同一个引擎，消费结构化
事件而不是解析终端输出。引擎把 `ffmpeg` 和 `ffprobe` 作为独立程序运行；它们随包提供，或由调用方指定。

## 转换流水线

每个候选文件依次经过一条流水线：

1. **发现** — 仅读取目录条目和文件扩展名，不做探测。网络硬盘的扫描保持轻量。等待复核决定的文件在此
   跳过。
2. **探测** — `ffprobe` 读取主视频属性（`VideoInfo`）、流布局（`StreamInfo`，含实际索引、编解码器、
   编码器标签、轨道 ID、handler）和章节元数据。已经是目标编码的文件在此跳过。
3. **流策略** — 依据有界的 ISO BMFF `tref/chap` 证据对可能的 QuickTime 章节承载轨分类（见
   [chapter-carrier.md](chapter-carrier.md)）；映射流集合和章节策略在任何其他步骤之前确定。
4. **容器选择** — 严格 MP4 或保守的 MKV 回退（见 [output-format.md](output-format.md)）。
5. **规划** — 根据编码器偏好、严格程度和目标编码，为该文件构建有序的编码尝试列表（见
   [options.md](options.md)）。
6. **画质选定** — 要么对短片段做调整搜索，要么根据源推算（见 [quality.md](quality.md)）。
7. **转码** — 每次尝试执行一次 `ffmpeg`，使用显式 `-map 0:<index>` 参数、进度解析和隐藏的 Windows
   子进程（见 [encoders.md](encoders.md)）。
8. **验证** — 将探测到的输出与期望的视频属性、时长、流签名和章节语义比较（见
   [validation.md](validation.md)）。被归类为可通过重封装修复的问题，在最终验证前执行可选的元数据
   修复。
9. **测量** — 把结果与源做比较（见 [quality.md](quality.md)）。
10. **提交** — 没有任何改动且达到目标的结果替换源文件，带 `.old` 备份和跨设备复制回退。其余结果写在
    原文件旁边，交由用户决定（见 [review.md](review.md) 和 [safety.md](safety.md)）。

## 仓库布局

```text
src/main.rs      Minimal binary entry point.
src/lib.rs       The pipeline: discovery, probing, planning, transcoding, validation, commit.
src/options.rs   Every user-facing option and its wire value.
src/codec.rs     What differs between AV1, H.265 and H.266.
src/vmaf.rs      Quality measurement.
src/search.rs    Choosing a quality setting by sampling.
src/review.rs    Conversions kept for the user to decide about.
src/ffi.rs       The embedding ABI.
gui/             The application, for Windows, macOS, Linux and Android.
scripts/         Packaging and build helpers.
config.example.yaml  Default settings template copied into release packages.
doc/en-us/       Authoritative documentation (this tree).
doc/zh-cn/       Simplified Chinese mirror.
README.md        User-facing quickstart, build, runtime, and output rules.
AGENTS.md        Agent working rules only; behaviour lives in doc/.
```

命令行和嵌入接口通过 `run_with_events` 共用同一套工作流。流水线本身仍是一个文件，因此
[functions/](functions/INDEX.md) 下的函数文档按行为域分组，而不是镜像源文件路径。

## 视频工具查找

MyVidComp 需要 `ffmpeg` 和 `ffprobe`。它先在可执行文件旁边查找，然后是同级的 `bin/` 目录，最后是
`PATH`。macOS 和 Linux 使用 `ffmpeg` 与 `ffprobe`；Windows 使用 `ffmpeg.exe` 与 `ffprobe.exe`；
Android 从它解包原生库的文件夹中运行它们，在那里它们名为 `libffmpeg.so` 和 `libffprobe.so`（见
[android.md](android.md)）。

可选能力都是动态检测的，绝不仅凭系统宣称的编码器列表就予以信任：每个编码器还必须通过一次合成的运行时
编码检查，而画质测量只有在视频工具确实提供时才会启用。

## 配置

`config.yaml` 属于用户：MyVidComp 在需要时读取它，但绝不重写或规范化它。命令行取值优先于配置取值。
某个键存在但取值为空表示「使用默认值」，因此随包附带的模板可以列出每一个键而不必给任何一个赋值。

## 安全模型

在转换和验证成功之前，原文件始终保留。耗时较长的 ffmpeg 写入指向本工具自己的临时文件（配置了
`tmp_dir` 时用它），目标文件夹只在最终提交时才被触碰。通过验证的临时输出会从缓存中复用，包括旧版
安装留下的那些；无法读取的则会被删除。工具没有把握的转换会保留在原文件旁边，而不是替换它。细节和
不变量见 [safety.md](safety.md)。

## 文档维护

每个新的行为领域都必须在同一次改动中补上 `doc/en-us/` 页面以及对应的 `doc/zh-cn/` 翻译。确切规则见
本仓库的 `AGENTS.md`。
