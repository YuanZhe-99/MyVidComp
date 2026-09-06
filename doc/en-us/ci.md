# Automated checks

What runs on every change, and how to run the same checks locally.

## On every push

`.github/workflows/ci.yml` runs four jobs.

| Job | Checks |
|---|---|
| Conversion engine | Formatting, lints, and tests on Linux and Windows. Also refuses any reappearance of the old project name outside the one constant that recognises files an older install left behind. |
| Interface | Dart formatting, lints, tests, and a Windows build. |
| Android | Builds the engine for `arm64-v8a`, builds the media tools from source, and builds the application around both, then confirms all three really are inside the package. |
| Documentation | Confirms both language trees still describe the same thing. |

The engine job installs the media tools so the tests that need them actually run instead of quietly
skipping.

## On a version tag

`.github/workflows/release.yml` runs when a `v*` tag is pushed. It first refuses to continue unless
the tag matches the version in `Cargo.toml`. Then it builds the two Windows packages, the
command-line package, and the Android package, writes one checksum file covering all of them, and
publishes them.

The media-tool download URLs are pinned to a dated build rather than the rolling one, so
re-running the workflow for the same version produces the same bundle and the same checksums.
Those builds are removed by their publisher after a few months; when a pinned one is gone the
workflow takes the newest build of the same FFmpeg series instead and says so in the run, so a
release is never blocked by a download that expired.

Starting the workflow by hand builds every package and stops there, leaving them as run
artifacts. Only a tag publishes.

The Android media tools are compiled rather than downloaded, which takes far longer than
everything else in the workflow. The result is kept between runs and rebuilt only when
`scripts/build-android-ffmpeg.sh` changes, which is also the only thing that changes what it
produces, because every source version in it is pinned.

## Cutting a release

Set the new version in `Cargo.toml`, `gui/pubspec.yaml` and `gui/windows/runner/Runner.rc`, say
what changed in `version-history.md` in both languages, then:

```sh
git commit -am "Release 0.1.2"
git tag -a v0.1.2 -m "MyVidComp 0.1.2"
git push github main
git push github v0.1.2
```

Pushing the tag is the whole trigger: nothing else has to be started by hand. The tag has to reach
the GitHub remote, so a tag pushed only to another remote builds nothing there.

## Running the same checks locally

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

## Runner images

The release builds each Windows package on a runner of its own architecture: `windows-2025` for
x64 and `windows-11-arm` for Arm64. Flutter builds a Windows application only for the machine it
is running on, so no single runner can produce both. `windows-2025` is pinned rather than
`windows-latest` because the newer image is missing build tools the packaging needs.

Flutter is pinned to an exact version rather than a channel, so a Flutter release never changes the
build underneath a working checkout.

## Running elsewhere

Self-hosted forge software that implements the same workflow format can run these files, either by
reading `.github/workflows` directly or from a copy under its own directory. The Linux jobs run
anywhere; the Windows and Android jobs need runners with those toolchains, so a Linux-only runner
will only be able to run the engine and documentation jobs.
