# Windows 打包

## 双架构 GUI 打包

`scripts/package-gui-windows-all.ps1` 是受支持的双架构发布入口。它构建显式的 `arm64`/`x64` 构建目录，并验证每个打包 EXE/DLL 的 PE machine 字段——绝不回到"哪个 Flutter 构建目录最近被修改"，绝不把 x64 运行时二进制混入 ARM64 包。

架构映射：

| 包 | Flutter 目标 | Rust 目标 |
|---|---|---|
| `MyVidComp-windows-arm64` | `windows-arm64` | `aarch64-pc-windows-msvc` |
| `MyVidComp-windows-x64` | `windows-x64` | `x86_64-pc-windows-msvc` |

双包脚本用 Flutter 构建宿主目标，用同一 Visual Studio CMake 生成器构建另一目标。每个包包含 Flutter 发布包、匹配的 `myvidcomp_core.dll`、`bin/ffmpeg.exe`、`bin/ffprobe.exe`、README 文件和 `SHA256SUMS.txt`。

## 安装程序

每个架构还会附带一个安装程序，由 Inno Setup 从同一个 `installer.iss` 构建，通过 `ARM64` 符号切换，
和这里其他项目的做法一致。它打包的是暂存好的安装包目录，而不是 Flutter 的原始输出，因此安装程序和
zip 里的内容完全一致。

它按当前用户安装到 `{autopf}`，不会请求管理员权限。`AppId` 这个 GUID 永远不能更改：Windows 靠它
识别升级，换一个就会在原有副本旁边再装一份。

版本号是传进去的，而不是写死在脚本里：

```powershell
scripts/package-gui-windows.ps1 -Platform x64 -DownloadFfmpeg
scripts/package-gui-windows-installer.ps1 -Platform x64
```

## FFmpeg 捆绑

- 包布局为相对路径：`bin/ffmpeg.exe` 与 `bin/ffprobe.exe`。绝不把开发者机器的绝对路径写入包文件。
- `-DownloadFfmpeg` 自动下载匹配的 BtbN FFmpeg 构建。下载缓存记录其来源 URL；显式 URL 变更使缓存归档失效；部分下载绝不替换有效缓存；`-RefreshFfmpeg` 供可变的 `latest` URL 使用。
- 在 `MyVidComp.exe`、`myvidcomp_core.dll`、`flutter_windows.dll`、FFmpeg/FFprobe 和任何运行时 DLL 全部通过包脚本对所选架构的 PE machine 检查之前，不得发布 GUI 包。

## CLI 打包

`scripts/package.sh` 为 `x86_64-pc-windows-gnu` 构建 CLI（`myvidcomp.exe`）；Windows ARM64 则用 `aarch64-pc-windows-gnullvm`，配 ARM64 FFmpeg/FFprobe 捆绑和匹配的 llvm-mingw ARM64 运行时 DLL（如 `libunwind.dll`）。未检查导入 DLL 前不得发布 Windows ARM64 `myvidcomp.exe`——llvm-mingw 构建可能需要在可执行文件旁放置运行时 DLL。发布包包含 `myvidcomp.exe`、可选 `bin/` 运行时工具、`config.yaml`、`README.md` 和 `SHA256SUMS.txt`。

## 包配置模板

`config.example.yaml` 被复制为发布 `config.yaml`。它保持 `conversion_mode` 与 `output_format` 为注释，因为旧 MyVidComp 构建会拒绝未知配置键；仅对支持这些键的构建取消注释。

## 构建标记

每个构建嵌入一个 `BUILD_MARKER`，显示在 `--version` 输出中（如 `chapter-carrier.20260721a`）。它标识打包构建的行为世代。
