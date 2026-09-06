# Automated checks

What runs on every change, and how to run the same checks locally.

## On every push

`.github/workflows/ci.yml` runs four jobs.

| Job | Checks |
|---|---|
| Conversion engine | Formatting, lints, and tests on Linux and Windows. Also refuses any reappearance of the old project name outside the one constant that recognises files an older install left behind. |
| Interface | Dart formatting, lints, tests, and a Windows build. |
| Android | Builds the engine for `arm64-v8a` and the application around it, then confirms the engine really is inside the package. |
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

The Windows jobs pin `windows-2025` rather than `windows-latest`. The newer image is missing the
Arm64 build tools the dual-architecture packaging needs, and pinning keeps both workflows on the
image that has them. Revisit this once that is fixed upstream.

Flutter is pinned to an exact version rather than a channel, so a Flutter release never changes the
build underneath a working checkout.

## Running elsewhere

Self-hosted forge software that implements the same workflow format can run these files, either by
reading `.github/workflows` directly or from a copy under its own directory. The Linux jobs run
anywhere; the Windows and Android jobs need runners with those toolchains, so a Linux-only runner
will only be able to run the engine and documentation jobs.
