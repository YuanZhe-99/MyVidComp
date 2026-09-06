# Android

The phone build, what it needs from the platform, and what it still lacks.

## What works

The application builds, installs and runs on Android. The interface is the same one the desktop
uses, laid out for a phone. The conversion engine is compiled for `arm64-v8a` and packaged with
the application, and Android unpacks it as a real file the engine can load.

## What is missing

The media tools are not bundled yet, so a fresh install can browse folders and change settings but
cannot convert anything. It says so plainly on the first screen rather than failing part-way
through a conversion.

There is no official Android build of FFmpeg to bundle, and the one this project needs is
unusually demanding: it has to include `libvmaf` for quality measurement, `libx265`, `libsvtav1`
and `libvvenc` for the three output codecs, and be aligned for 16 KB memory pages. Adding it means
either building FFmpeg with the Android toolchain or taking a build from a project that publishes
command-line binaries for Android. `scripts/fetch-android-ffmpeg.ps1` installs one once you have
it.

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

scripts/build-android-core.ps1        # compile the engine
scripts/fetch-android-ffmpeg.ps1 ...  # install the media tools
scripts/package-android.ps1           # build the installable package
```

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
