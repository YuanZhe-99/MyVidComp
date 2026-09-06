# English → Simplified Chinese Translation Guide

This guide governs how `doc/zh-cn/` is produced and kept in sync with `doc/en-us/` in this
repository. `doc/en-us/` is the authoritative tree; `doc/zh-cn/` is a translation of it, never an
independent source. Read this before writing or updating any Chinese documentation page.

## 1. Scope and workflow

- English content is authored first, directly from the actual source code and `AGENTS.md`.
  Chinese content is then produced from the finished English page using this guide and the
  glossary in Section 5.
- Any change to a function, policy, data format, or feature must update the English page and the
  Chinese page in the same commit. Do not let the two trees drift.
- New terminology encountered while translating goes into Section 5. The glossary covers this
  repository's video/container/FFmpeg domain and the shared documentation vocabulary.

## 2. Structural parity rules

`doc/zh-cn/<path>` must mirror `doc/en-us/<path>` exactly:

- The same set of files exists in both trees — no file present in one language and missing in
  the other.
- The same heading hierarchy and count (`#`, `##`, `###`, ...).
- The same number of tables and table rows, in the same order.
- The same number of fenced code blocks, with **identical code inside** (code is data, not prose).
- The same internal links and anchors, pointing at the translated equivalents.

A verification pass compares heading counts, table-row counts, and code-fence counts between the
two trees file-by-file; both must match exactly.

## 3. What is never translated

- Identifiers: struct/function/variable/field names, file paths, directory names.
- CLI commands and their flags/output.
- Configuration keys (e.g. `target_codec`, `preservation`, `quality_mode`, `tmp_dir`).
- URLs.
- Product, framework, and protocol names: MyVidComp, FFmpeg, FFprobe, AV1, MP4, MKV, Matroska,
  QuickTime, ISO BMFF, Flutter, Dart, Windows, Android, ARM64, x64, MSVC, CMake, PowerShell,
  H.265, HEVC, H.266, VVC, VMAF.
- The `Tier A` / `Tier B` labels used in the function index.
- Anything inside a fenced code block, including comments written as part of example code,
  unless the comment is prose explaining the example outside the executable line — in which case
  translate the explanatory comment text but never the code tokens themselves.

## 4. Style rules

- Use a neutral, declarative technical tone. Do not use the formal pronoun 您; use 你 only if a
  second-person address is unavoidable, otherwise prefer impersonal phrasing.
- Use full-width Chinese punctuation in prose (，。：；「」) but keep all Markdown syntax
  characters (`#`, `` ` ``, `|`, `-`, `*`, `[]()`) in their normal ASCII form so Markdown still
  parses.
- Insert a single space between CJK characters and adjacent Latin letters or digits
  (e.g. "支持 AV1 编码", "保留 60 秒").
- Keep sentences short; prefer splitting a long English sentence into two Chinese sentences over
  producing one dense run-on sentence.
- Numbers, version numbers, file names, and code identifiers stay exactly as written in English.

## 5. Terminology glossary

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

## 6. Review checklist (run before committing a Chinese page)

- [ ] File exists at the same relative path under `doc/zh-cn/` as its English counterpart.
- [ ] Heading count matches (`grep -c '^#'`).
- [ ] Code-fence count matches (`grep -c '^```'`), and code contents are byte-identical to English.
- [ ] Table row counts match.
- [ ] Every glossary term used matches Section 5 exactly; any new term was added to Section 5.
- [ ] Internal links resolve to the Chinese-tree equivalents, not back to `doc/en-us/`.
