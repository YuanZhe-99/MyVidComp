# Android

手机版本、它对平台的要求，以及目前还缺什么。

## 已经可用的部分

应用程序可以在 Android 上构建、安装和运行。界面与桌面版是同一套，只是按手机布局。转换引擎已为
`arm64-v8a` 编译并随应用打包，Android 会把它解包成引擎能加载的真实文件。

## 还缺什么

视频工具尚未随包提供，因此全新安装可以浏览文件夹、修改设置，但还不能转换任何东西。它会在第一个
屏幕上直说这一点，而不是在转换到一半时才失败。

Android 上没有官方的 FFmpeg 构建可供打包，而本项目所需的那种尤其苛刻：它必须包含用于画质测量的
`libvmaf`，用于三种输出编码的 `libx265`、`libsvtav1` 和 `libvvenc`，并且要按 16 KB 内存页对齐。
补上它意味着要么用 Android 工具链自行构建 FFmpeg，要么从某个发布 Android 命令行二进制的项目取用。
拿到之后，`scripts/fetch-android-ffmpeg.ps1` 负责把它装好。

## 在 Android 上运行一个程序

Android 拒绝从应用自己的数据目录运行程序。唯一允许运行的地方是它解包原生库的那个文件夹，而且文件
必须以库的方式命名。因此 `ffmpeg` 和 `ffprobe` 以 `libffmpeg.so` 和 `libffprobe.so` 的名义随包
提供。

有两处设置让这一切成立，缺一不可：

- Gradle 构建中的 `packaging { jniLibs { useLegacyPackaging = true } }`，
- 清单中的 `android:extractNativeLibs="true"`。

没有它们，Android 会直接从安装包里映射原生库而不写到磁盘上，于是根本没有文件可以运行，工具看起来
就像不存在。

## 权限

引擎使用普通的文件路径工作，因为它要就地改名和替换视频。比「所有文件访问」更窄的权限都做不到这一
点：文档类权限交回的是不透明的标识符而非路径。用户首次选择文件夹时应用会请求该权限，这会打开系统
设置页面，因为这项权限没有应用内对话框。

清单还声明了媒体处理前台服务、唤醒锁和通知，这样长时间的转换不会在息屏后被杀掉。

## 选择文件夹

Android 没有能返回引擎可用路径的文件夹对话框，因此应用自己浏览文件夹，从平台报告的存储卷开始。

## 构建

```powershell
cargo install cargo-ndk
rustup target add aarch64-linux-android

scripts/build-android-core.ps1        # compile the engine
scripts/fetch-android-ffmpeg.ps1 ...  # install the media tools
scripts/package-android.ps1           # build the installable package
```

Gradle 构建本身也会调用引擎构建，因此只要装好 Rust 工具链，单独运行 `flutter build apk` 也可以。

NDK 版本很关键。Flutter 固定使用 28.x 系列，引擎必须用同一个版本构建，两边才能在内存页对齐上保持
一致，而 Android 15 及以后会强制检查这一点。构建脚本会在装有 28.x NDK 时选用它，不得不退而求其次
时则会给出警告。

## 值得了解的限制

- Android 15 会在任意二十四小时内让媒体处理前台服务运行六小时后停止。较大的媒体库需要不止一次会话。
- 手机编码器很快，但没有恒定画质模式，因此只能给码率。某些型号在给定码率下效果很差，这正是每个编码
  器在被交付真实文件之前都要先编码一帧来测试的原因。
- 在手机上用软件编码 AV1 和 H.266 慢到除了短片之外都不实用。
- 共享存储卷要经过一层转换，比应用私有存储慢。请把工作文件夹留空，让临时文件留在同一卷上每个视频的
  旁边；把它指向应用存储会让每次提交都变成一次完整复制。

## 分发

侧载，或从发布页下载。Google Play 把「所有文件访问」限制在一小类应用上并要求提交声明表，这超出了
本文的范围。
