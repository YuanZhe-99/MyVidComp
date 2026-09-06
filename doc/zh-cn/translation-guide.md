# 英文 → 简体中文翻译指南

本指南规范本仓库 `doc/zh-cn/` 的产出方式及其与 `doc/en-us/` 的同步方式。`doc/en-us/` 是权威树；`doc/zh-cn/` 是它的翻译，绝不是独立的来源。撰写或更新任何中文文档页之前请先阅读本指南。

## 1. 范围与流程

- 英文内容先行撰写，直接依据实际源代码和 `AGENTS.md`。中文内容随后基于定稿的英文页面、按本指南和第 5 节术语表产出。
- 任何函数、策略、数据格式或功能的变更，必须在同一次提交中更新英文页和中文页。两棵树不允许漂移。
- 翻译中遇到的新术语进入第 5 节。术语表覆盖本仓库的视频/容器/FFmpeg 领域及共享的文档词汇。

## 2. 结构对齐规则

`doc/zh-cn/<path>` 必须与 `doc/en-us/<path>` 精确镜像：

- 两棵树存在相同的文件集合——不允许一种语言有而另一种缺失的文件。
- 相同的标题层级和数量（`#`、`##`、`###`、……）。
- 相同数量的表格和表格行，顺序一致。
- 相同数量的围栏代码块，代码内容**完全一致**（代码是数据，不是散文）。
- 相同的内部链接和锚点，指向翻译后的对应页面。

校验流程逐文件比较两棵树的标题数、表格行数和代码围栏数；两者必须完全一致。

## 3. 永不翻译的内容

- 标识符：结构体/函数/变量/字段名、文件路径、目录名。
- CLI 命令及其参数/输出。
- 配置键（如 `conversion_mode`、`output_format`、`tmp_dir`）。
- URL。
- 产品、框架和协议名：MyVidComp、FFmpeg、FFprobe、AV1、MP4、MKV、Matroska、QuickTime、ISO BMFF、Flutter、Dart、Windows、ARM64、x64、MSVC、CMake、PowerShell。
- 函数索引使用的 `Tier A` / `Tier B` 标签。
- 围栏代码块内的任何内容，包括作为示例代码一部分的注释，除非注释是解释示例的散文且位于可执行行之外——此时翻译解释性注释文本，但绝不翻译代码 token 本身。

## 4. 风格规则

- 使用中性、陈述性的技术语气。不使用敬称"您"；仅在无法避免第二人称时使用"你"，否则优先使用客观表述。
- 散文使用全角中文标点（，。：；「」），但所有 Markdown 语法字符（`#`、`` ` ````、`|`、`-`、`*`、`[]()`）保持正常 ASCII 形式，使 Markdown 仍可解析。
- CJK 字符与相邻的拉丁字母或数字之间插入一个空格（如"支持 AV1 编码"、"保留 60 秒"）。
- 保持句子简短；优先把一个长英文句拆成两个中文句，而不是写成一个密集的长句。
- 数字、版本号、文件名和代码标识符保持英文原样。

## 5. 术语表

| English | 中文 | Notes |
|---|---|---|
| transcode / transcoding | 转码 | 编解码转换；不译作"编码"以免与 encode 混淆 |
| encode / encoding | 编码 | |
| AV1 / H.264 / HEVC | AV1 / H.264 / HEVC | 不译 |
| container | 容器 | 封装格式，如 MP4、MKV |
| MP4 / MKV / Matroska | MP4 / MKV / Matroska | 不译 |
| stream | 流 | 媒体流（视频/音频/字幕/数据） |
| stream layout | 流布局 | 流的数量、顺序与类型 |
| stream mapping | 流映射 | FFmpeg `-map` 参数行为 |
| chapter | 章节 | |
| chapter carrier | 章节承载轨 | QuickTime 中承载章节引用信息的轨道 |
| placeholder chapter | 占位章节 | 无标题、覆盖全片的空章节 |
| chapter re-authoring | 章节重建 | 从章节元数据重新生成输出章节 |
| stream signature | 流签名 | 用于验证输出流布局的期望集合 |
| pixel format | 像素格式 | |
| bit depth | 位深 | |
| chroma location | 色度位置 | chroma sample location |
| sample aspect ratio (SAR) | 采样宽高比（SAR） | |
| display aspect ratio (DAR) | 显示宽高比（DAR） | |
| interlaced | 隔行扫描 | field order 为隔行的源 |
| frame rate | 帧率 | |
| resolution | 分辨率 | |
| probe / probing | 探测 | ffprobe 读取媒体元数据 |
| encoder | 编码器 | |
| runtime encode check | 运行时编码检查 | 合成编码探针，不信任厂商列表 |
| fallback | 回退 | |
| whitelist | 白名单 | |
| bitstream filter (BSF) | 比特流过滤器（BSF） | |
| validation | 验证 | |
| metadata repair | 元数据修复 | 仅重封装以修正 AV1 元数据 |
| commit | 提交 | 验证通过后临时输出替换源文件的最终步骤 |
| temp file / temp output | 临时文件 / 临时输出 | `.myvidcomp-*.tmp.*` |
| cached temp output | 缓存的临时输出 | 验证通过后复用的既有临时文件 |
| graceful stop | 优雅停止 | 完成当前文件后停止 |
| dry run | 试运行 | 不转码的预览模式 |
| conversion mode | 转换模式 | `consistency` / `hardware` |
| output format | 输出格式 | `mp4` / `mkv-fallback` |
| keep original | 保留原文件 | 转换后保留 `.old` 备份 |
| CRF | CRF | Constant Rate Factor，不译 |
| bitrate | 比特率 | |
| FFI / ABI | FFI / ABI | 不译 |
| GUI | GUI | 不译；指 `gui/` 下的 Flutter 桌面界面 |
| worker isolate | worker isolate | Flutter 术语，不译 |
| packaging | 打包 | |
| checksum | 校验和 | |
| side effects | 副作用 | |
| declaration | 声明 | function/struct/enum/trait/const 统称 |
| function index | 函数索引 | |
| Tier A / Tier B | Tier A / Tier B | 文档覆盖分级标签，不译 |
| l10n / localization | 本地化（l10n） | |

## 6. 提交中文页前的检查清单

- [ ] 文件存在于 `doc/zh-cn/` 下与英文对应页面相同的相对路径。
- [ ] 标题数量一致（`grep -c '^#'`）。
- [ ] 代码围栏数量一致（`grep -c '^```'`），且代码内容与英文逐字节一致。
- [ ] 表格行数一致。
- [ ] 使用的每个术语与第 5 节完全一致；新术语已加入第 5 节。
- [ ] 内部链接指向中文树中的对应页面，而非指回 `doc/en-us/`。
