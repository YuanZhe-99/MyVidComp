# Android

手机版本随包带了什么、它对平台的要求，以及它与桌面版的差别。

## 安装包里有什么

转换所需的一切。界面与桌面版是同一套，只是按手机布局。转换引擎已为 `arm64-v8a` 编译，FFmpeg 工具
也一并构建并随包提供，因此全新安装无需再下载任何东西就能转换视频。

## 视频工具

已发布的 Android FFmpeg 构建中没有一个带齐本项目所需的部件，因此随包的二进制由
`scripts/build-android-ffmpeg.sh` 从源码构建。它产出一份 FFmpeg，其中包含用于画质测量的 `libvmaf`，
用于三种输出编码的 `libx265`、`libsvtav1` 和 `libvvenc`，用于快速 AV1 解码的 `libdav1d`，以及通往
手机自身视频硬件的 MediaCodec。

该构建在 Linux 上运行，需要 clang、cmake、ninja、meson 和 make，以及一份 Android NDK sysroot。它
不需要 NDK 自带的编译器：它使用宿主机的 clang，指向 NDK 的 sysroot 和运行时库。正因如此，它可以在
Arm Linux 宿主上运行，包括 Arm PC 上的 WSL——而 Google 根本没有为这种宿主发布过 NDK。

如果你已经从别处拿到了适用于 Android 的 Arm FFmpeg 构建，`scripts/fetch-android-ffmpeg.ps1` 可以
用它替换这一份装好。

## 使用手机的视频硬件

高通、联发科、三星和 Google 都通过同一个 Android 接口 MediaCodec 暴露它们的视频编解码器，别无他
途。没有哪个厂商需要单独适配：支持 MediaCodec 就等于支持它们全部。FFmpeg 通过平台自身的 C 接口访
问它，不需要 Java 运行时，因此这些工具作为普通程序就能使用它。

帧是按硬件索取的节奏交给它的，而不是一次一帧。一次一帧才是默认方式，在测试所用的这台手机上，它对
每一种编码都会在第一帧就失败，因此从不采用。

各款手机实际提供什么仍有差别，而且手机误报自身能力的情况多到不能轻信它声明的编码器。因此每个编码
器在用于文件之前都会先编一帧，失败的那些会被搁置。可以指望有硬件支持的是 H.265；硬件 AV1 编码只
存在于最新的芯片上，H.266 则一款都没有。

苹果的对应物 VideoToolbox 在 macOS 版本中以同样的方式使用。

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

scripts/build-android-core.ps1            # compile the engine
scripts/package-android.ps1 -BuildFfmpeg  # media tools, then the package
```

`-BuildFfmpeg` 会通过 WSL 运行视频工具的构建。在 Linux 上直接运行
`scripts/build-android-ffmpeg.sh`。它第一次很慢，之后就什么都不做了，因为构建好的工具会留在原地。

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
