# MyVidComp

Make a folder of videos smaller without losing what matters.

MyVidComp converts every video in a folder to a modern codec, measures how close each result looks
to its original, and never deletes anything it is not sure about. When a conversion has to change
something, it keeps both files and asks you which to keep.

## What it does

- **Three output codecs.** AV1 for the smallest files, H.265 for the widest device support, H.266
  if you want to experiment.
- **Quality you choose, not a guess.** Pick a quality level and it finds the smallest setting that
  reaches it, by test-encoding short samples of each video.
- **It checks its own work.** Every result is compared against its original with Netflix's VMAF,
  and scored out of 100.
- **It never quietly loses anything.** A conversion that changed some small detail, or scored below
  your target, is kept beside the original for you to compare and decide.
- **It says why it skipped something.** In plain words, not error codes.

## Install

Download a release for your system, unpack it, and run it. Nothing else is needed: the media tools
come in the package.

| Package | For |
|---|---|
| `MyVidComp-windows-x64.zip` | Windows on Intel or AMD |
| `MyVidComp-windows-arm64.zip` | Windows on Arm |
| `myvidcomp-cli-*.zip` | The command-line tool on its own |
| `MyVidComp-android-arm64.apk` | Android phones and tablets |

The Android package carries its own media tools too, and uses the phone's video hardware where the
phone has any; see [doc/en-us/android.md](doc/en-us/android.md).

## Use it

Open the application, choose a folder, and press Start. Everything else has a sensible default.

From the command line:

```sh
# See what would happen, change nothing
myvidcomp /path/to/videos --dry-run

# Convert everything
myvidcomp /path/to/videos

# Best device compatibility rather than smallest files
myvidcomp /path/to/videos --codec hevc

# Convert only what can be reproduced exactly
myvidcomp /path/to/videos --preservation strict

# See what is waiting for a decision, then keep every converted file
myvidcomp /path/to/videos --list-reviews
myvidcomp /path/to/videos --resolve-reviews keep-new
```

`myvidcomp --help` lists every option. Full descriptions are in
[doc/en-us/options.md](doc/en-us/options.md).

During a run, type `q` then Enter, then `y` then Enter, to stop after the current file. The running
conversion is never interrupted part-way.

## Settings file

With no arguments, MyVidComp reads `config.yaml` from the current folder. Command-line values win.
It never rewrites the file.

```yaml
target_folder: "D:\\Videos"
target_codec: av1
preservation: flexible
quality_target: 95
quality_mode: search
keep_original: true
```

## What it will not do

It never changes resolution, frame rate, or how HDR is handled, and never re-encodes audio or
subtitles. Interlaced video is left alone, because deinterlacing changes the picture itself. Under
strict preservation it also refuses any conversion that cannot reproduce every detail, and says
which detail stopped it.

## Build it

```sh
cargo build --release          # the engine and command-line tool
cd gui && flutter build windows --release
```

```powershell
# Both Windows packages, with the media tools
scripts/package-gui-windows-all.ps1 -DownloadFfmpeg
```

Android needs `cargo install cargo-ndk` and
`rustup target add aarch64-linux-android` first, then `scripts/package-android.ps1`.

## Checks

```sh
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test
cd gui && flutter analyze && flutter test
```

```powershell
scripts/check-doc-parity.ps1
```

## Documentation

[doc/en-us/](doc/en-us/README.md) is the authoritative description of how everything behaves, with
a Simplified Chinese mirror in [doc/zh-cn/](doc/zh-cn/README.md). Start with
[options.md](doc/en-us/options.md) for the settings, [quality.md](doc/en-us/quality.md) for how
quality is decided and measured, and [review.md](doc/en-us/review.md) for what happens when
MyVidComp will not decide for you.

## Licence

The bundled FFmpeg builds include GPL-licensed components, so the packages are distributed under
the GPL. The media tools' own sources are published by their upstream projects.
