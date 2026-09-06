# Automated checks

What runs on every change, and how to run the same checks locally.

## On every push

`.github/workflows/ci.yml` runs four jobs.

| Job | Checks |
|---|---|
| Conversion engine | Formatting, lints, and tests on Linux and Windows. |
| Interface | Dart formatting, lints, and tests. |
| Android | Builds the engine for `arm64-v8a`, builds the media tools from source, and builds the application around both, then confirms all three really are inside the package. |
| Documentation | Confirms both language trees still describe the same thing, and refuses any reappearance of the old project name outside the constant that recognises files an older install left behind. |

The engine job installs the media tools so the tests that need them actually run instead of quietly
skipping.

## On a version tag

`.github/workflows/release.yml` runs when a `v*` tag is pushed. It builds the two Windows
packages, the command-line package and the Android package, then refuses to publish unless the tag
matches the version in `Cargo.toml`, writes one checksum file covering every download, and
publishes them.

The Windows packages bundle the rolling build of the FFmpeg 9.0 series. Dated builds were used
before, until one was removed by its publisher and stopped a release; the rolling address of a
fixed series does not expire.

Starting the workflow by hand builds every package and stops there, leaving them as run
artifacts. Only a tag publishes.

The Android media tools are compiled rather than downloaded, which takes far longer than
everything else in the workflow. The result is kept between runs and rebuilt only when
`scripts/build-android-ffmpeg.sh` changes, which is also the only thing that changes what it
produces, because every source version in it is pinned.

## Signing the Android package

The package is signed with the upload key when all four of these repository secrets are set, and
with the debug key when they are not. A debug-signed package installs, but nothing signed with the
real key can ever replace it, so a release meant for people has to carry the real signature.

```
KEYSTORE_BASE64
STORE_PASSWORD
KEY_ALIAS
KEY_PASSWORD
```

`KEYSTORE_BASE64` is the keystore file itself, base64-encoded. The workflow decodes it to
`gui/android/app/upload-keystore.jks` and writes the other three into `gui/android/key.properties`;
Git ignores both. Writing that same properties file by hand is what signs a build made here.

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
