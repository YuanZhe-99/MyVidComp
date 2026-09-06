# 自动检查

每次改动会运行什么，以及如何在本地运行同样的检查。

## 每次推送

`.github/workflows/ci.yml` 运行四个任务。

| 任务 | 检查内容 |
|---|---|
| 转换引擎 | 在 Linux 和 Windows 上检查格式、静态检查和测试。 |
| 界面 | Dart 格式、静态检查和测试。 |
| Android | 为 `arm64-v8a` 构建引擎，从源码构建视频工具，再围绕两者构建应用，然后确认这三样确实都在包内。 |
| 文档 | 确认两个语言树仍然描述同一件事，并拒绝旧项目名称的任何再次出现，唯一的例外是那个用于识别旧版安装遗留文件的常量。 |

引擎任务会安装视频工具，好让需要它们的测试真正运行，而不是悄悄跳过。

## 打版本标签时

`.github/workflows/release.yml` 在推送 `v*` 标签时运行。它先构建两个 Windows 包、命令行包和
Android 包，随后除非标签与 `Cargo.toml` 中的版本一致，否则拒绝发布；然后写出一份覆盖全部下载文件
的校验和文件，并发布它们。

Windows 包里带的是 FFmpeg 9.0 系列的滚动构建。此前用的是带日期的构建，直到其中一个被发布者删除、
把一次发布卡住为止；固定系列的滚动地址不会过期。

手动启动工作流只会构建全部安装包并到此为止，把它们留作运行产物。只有标签才会发布。

Android 的视频工具是编译出来的而不是下载来的，耗时远超工作流中的其他任何一步。构建结果会在多次运
行之间保留，只有 `scripts/build-android-ffmpeg.sh` 变动时才重新构建——而它也是唯一会改变产出的因
素，因为其中每一个源码版本都是固定的。

## 发布一个版本

在 `Cargo.toml`、`gui/pubspec.yaml` 和 `gui/windows/runner/Runner.rc` 中写入新版本号，在两种语言的
`version-history.md` 中说明改动，然后：

```sh
git commit -am "Release 0.1.2"
git tag -a v0.1.2 -m "MyVidComp 0.1.2"
git push github main
git push github v0.1.2
```

推送标签就是全部的触发条件，不需要再手动启动任何东西。标签必须推到 GitHub 这个远端；只推到别的远
端不会在那里构建任何东西。

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

发布流程在与目标架构相同的运行器上分别构建两个 Windows 安装包：x64 用 `windows-2025`，Arm64 用
`windows-11-arm`。Flutter 只会为它自己运行的机器构建 Windows 应用，所以单个运行器无法同时产出两
者。这里固定 `windows-2025` 而不是 `windows-latest`，是因为较新的镜像缺少打包所需的构建工具。

Flutter 固定到确切版本而不是发布通道，这样 Flutter 的新版本就不会在一份能用的检出下面把构建改掉。

## 在别处运行

实现了同一套工作流格式的自建代码托管软件可以运行这些文件，既可以直接读取 `.github/workflows`，也
可以从它自己的目录下的副本读取。Linux 任务在哪里都能跑；Windows 和 Android 任务需要带有相应工具链
的运行器，因此只有 Linux 的运行器只能跑引擎和文档这两个任务。
