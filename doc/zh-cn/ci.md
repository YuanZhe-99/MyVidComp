# 自动检查

每次改动会运行什么，以及如何在本地运行同样的检查。

## 每次推送

`.github/workflows/ci.yml` 运行四个任务。

| 任务 | 检查内容 |
|---|---|
| 转换引擎 | 在 Linux 和 Windows 上检查格式、静态检查和测试。同时拒绝旧项目名称的任何再次出现，唯一的例外是那个用于识别旧版安装遗留文件的常量。 |
| 界面 | Dart 格式、静态检查、测试，以及一次 Windows 构建。 |
| Android | 为 `arm64-v8a` 构建引擎，从源码构建视频工具，再围绕两者构建应用，然后确认这三样确实都在包内。 |
| 文档 | 确认两个语言树仍然描述同一件事。 |

引擎任务会安装视频工具，好让需要它们的测试真正运行，而不是悄悄跳过。

## 打版本标签时

`.github/workflows/release.yml` 在推送 `v*` 标签时运行。它首先拒绝继续，除非标签与 `Cargo.toml`
中的版本一致。随后构建两个 Windows 包、命令行包和 Android 包，写出一份覆盖全部文件的校验和文件，
并发布它们。

视频工具的下载地址固定指向某个带日期的构建，而不是滚动更新的那个，因此为同一版本重新运行工作流会
得到相同的包和相同的校验和。

Android 的视频工具是编译出来的而不是下载来的，耗时远超工作流中的其他任何一步。构建结果会在多次运
行之间保留，只有 `scripts/build-android-ffmpeg.sh` 变动时才重新构建——而它也是唯一会改变产出的因
素，因为其中每一个源码版本都是固定的。

## 在本地运行同样的检查

```sh
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test

cd gui
dart format --set-exit-if-changed lib test
flutter analyze
flutter test
flutter build windows --release
```

```powershell
scripts/check-doc-parity.ps1
scripts/package-gui-windows-all.ps1 -DownloadFfmpeg
```

## 运行环境镜像

Windows 任务固定使用 `windows-2025` 而不是 `windows-latest`。较新的镜像缺少双架构打包所需的 Arm64
构建工具，固定版本能让两个工作流都留在有这些工具的镜像上。等上游修好之后再重新评估。

Flutter 固定到确切版本而不是发布通道，这样 Flutter 的新版本就不会在一份能用的检出下面把构建改掉。

## 在别处运行

实现了同一套工作流格式的自建代码托管软件可以运行这些文件，既可以直接读取 `.github/workflows`，也
可以从它自己的目录下的副本读取。Linux 任务在哪里都能跑；Windows 和 Android 任务需要带有相应工具链
的运行器，因此只有 Linux 的运行器只能跑引擎和文档这两个任务。
