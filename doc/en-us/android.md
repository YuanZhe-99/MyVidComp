# Android

The phone build: what it ships, what it needs from the platform, and where it differs from the
desktop.

## What is in the package

Everything a conversion needs. The interface is the same one the desktop uses, laid out for a
phone. The conversion engine is compiled for `arm64-v8a`, and the FFmpeg tools are built and
packaged alongside it, so a fresh install converts videos without downloading anything.

## The media tools

No published Android build of FFmpeg carries what this project needs, so the shipped binaries are
built from source by `scripts/build-android-ffmpeg.sh`. It produces one FFmpeg with `libvmaf` for
quality measurement, `libx265`, `libsvtav1` and `libvvenc` for the three output codecs, `libdav1d`
for fast AV1 decoding, and MediaCodec for the phone's own video hardware.

The build runs on Linux and needs clang, cmake, ninja, meson and make, plus an Android NDK
sysroot. It does not need the NDK's own compiler: it uses the host's clang, pointed at the NDK
sysroot and runtime libraries. That is what lets it run on an Arm Linux host, including WSL on an
Arm PC, where Google publishes no NDK at all.

If you already have an Arm FFmpeg build for Android from somewhere else,
`scripts/fetch-android-ffmpeg.ps1` installs it in place of this one.

## Reaching the phone's video hardware

Qualcomm, MediaTek, Samsung and Google all expose their video encoders and decoders through one
Android interface, MediaCodec, and through nothing else. There is no vendor-specific path to add:
supporting MediaCodec supports all of them. FFmpeg reaches it through the platform's own C
interface, which needs no Java runtime, so the tools use it as ordinary programs.

Frames are handed to the hardware as it asks for them rather than one at a time. The one-at-a-time
way is the default, and on the phone this was tested on it fails on the very first frame for every
codec, so it is never used.

What each phone actually offers still differs, and phones misreport their own abilities often
enough that a declared encoder cannot be trusted. Every encoder is therefore given one frame to
encode before it is used on a file, and the ones that fail are set aside. H.265 is the codec to
expect hardware support for; AV1 encoding in hardware exists only on the newest chips, and H.266
in none of them.

Apple's equivalent, VideoToolbox, is used the same way by the macOS build.

## Running a program on Android

Android refuses to run a program from an application's own data folder. The one place it will run
one from is the folder it unpacks native libraries into, and only for files named like libraries.
So `ffmpeg` and `ffprobe` ship as `libffmpeg.so` and `libffprobe.so`.

Two settings make that work, and both are needed:

- `packaging { jniLibs { useLegacyPackaging = true } }` in the Gradle build,
- `android:extractNativeLibs="true"` in the manifest.

Without them Android maps native libraries straight out of the package instead of writing them to
disk, so there is no file to run and the tools appear to be missing.

## Permissions

The engine works with ordinary file paths, because it renames and replaces videos in place.
Nothing narrower than all-files access provides that: document permissions hand back opaque
identifiers rather than paths. The application asks for it when the user first chooses a folder,
which opens the system settings screen, because this permission has no in-app dialog.

The manifest also declares a media-processing foreground service, a wake lock and notifications,
so a long conversion is not killed when the screen goes off.

## Choosing a folder

Android has no folder dialog that returns a path the engine can use, so the application browses
folders itself, starting from the storage volumes the platform reports.

## Building

```powershell
cargo install cargo-ndk
rustup target add aarch64-linux-android

scripts/build-android-core.ps1            # compile the engine
scripts/package-android.ps1 -BuildFfmpeg  # media tools, then the package
```

`-BuildFfmpeg` runs the media build through WSL. On Linux, run
`scripts/build-android-ffmpeg.sh` directly. It is slow the first time and does nothing on later
runs, because the finished tools stay where it put them.

The Gradle build also invokes the engine build itself, so `flutter build apk` works on its own once
the Rust toolchain is present.

The NDK version matters. Flutter pins the 28.x series, and the engine has to be built with the same
one so both halves agree on memory page alignment, which Android 15 and later enforce. The build
script picks a 28.x NDK when one is installed and warns when it has to fall back.

## Limits worth knowing

- Android 15 stops a media-processing foreground service after six hours in any twenty-four. A
  large library needs more than one session.
- Phone encoders are fast but have no constant-quality mode, so they are given a bitrate. Some
  models produce poor results at a given bitrate, which is why every encoder is tested with a
  single frame before it is trusted with a file.
- Software AV1 and H.266 encoding on a phone is slow enough to be impractical for anything but
  short clips.
- Storage on the shared volume goes through a translation layer and is slower than app-private
  storage. Leave the working folder empty so temporary files stay beside each video on the same
  volume; pointing it at app storage turns every commit into a full copy.

## Distribution

Sideloading, or a release download. Google Play restricts all-files access to a short list of
application types and requires a declaration form, which is out of scope here.
