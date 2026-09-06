# 编码器选择与尝试

MyVidComp 在运行时检测可用的 AV1 编码器，绝不只信任 FFmpeg 厂商编码器列表：每个候选还必须通过合成运行时编码检查。

## 候选池

| 编码器 | 类型 | 说明 |
|---|---|---|
| `av1_nvenc` | NVIDIA GPU | |
| `av1_qsv` | Intel GPU | |
| `av1_amf` | AMD GPU | |
| `av1_mf` | Media Foundation GPU | Windows ARM64（Snapdragon）上优先于 `av1_vulkan` |
| `av1_vulkan` | Vulkan GPU | |
| `libsvtav1` | CPU（SVT-AV1） | |
| `libaom-av1` | CPU | |
| `librav1e` | CPU | |

`--encoder` 接受短别名：`nvenc`、`qsv`、`amf`、`mf`、`vulkan`。`auto`（默认）按转换模式自动排序；显式覆盖固定一个编码器，绝不切换到另一个。

## 检测要求

编码器仅在以下两者都成立时可用：

1. FFmpeg 在编码器列表中公布它。
2. 合成运行时编码检查成功（一个微小的 null 封装编码探针）。

已公布但失败的编码器连同失败详情上报（如缺少 `nvcuda.dll`、该媒体类型无 MFT、Vulkan 转换失败）并被跳过。若无候选存活，运行以终端错误失败，绝不猜测。

## 重试分类

逐文件尝试失败按保守方式分类。可重试：

- 设备/编码器/像素格式失败
- 严格输出不匹配（按计划验证失败）

终端——绝不重试：

- 未知错误、输入问题、磁盘问题、进程失败
- 修复基础设施失败、提交错误

缓存查找和提交完全位于尝试循环之外。

## 子进程创建

所有 Rust `ffmpeg`/`ffprobe` 子进程都通过共享的命令构造函数创建，使 Windows 应用 `CREATE_NO_WINDOW`——GUI 运行期间不闪现控制台窗口。不得重新引入直接 Dart 运行时 `-version` 进程检查（会闪现控制台窗口）。

检测/排序声明见 [functions/encoders-quality.md](functions/encoders-quality.md)，尝试循环见 [functions/transcoding.md](functions/transcoding.md)。
