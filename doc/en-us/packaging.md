# Windows Packaging

## Dual-architecture GUI packaging

`scripts/package-gui-windows-all.ps1` is the supported dual-architecture release entry point. It
builds explicit `arm64`/`x64` build directories and validates the PE machine field of every
packaged EXE/DLL — never returning to "whichever Flutter build directory was modified most
recently", and never mixing x64 runtime binaries into ARM64 packages.

Architecture mapping:

| Package | Flutter target | Rust target |
|---|---|---|
| `MyVidComp-windows-arm64` | `windows-arm64` | `aarch64-pc-windows-msvc` |
| `MyVidComp-windows-x64` | `windows-x64` | `x86_64-pc-windows-msvc` |

The dual-package script builds the host target with Flutter and the other target with the same
Visual Studio CMake generator. Each package contains the Flutter release bundle, matching
`myvidcomp_core.dll`, `bin/ffmpeg.exe`, `bin/ffprobe.exe`, README files, and `SHA256SUMS.txt`.

## Installer

Each architecture also ships an installer, built with Inno Setup from one `installer.iss` that
switches on an `ARM64` symbol, exactly as the sibling projects here do. It wraps the staged package
folder rather than the raw Flutter output, so the installer and the zip carry the same tree.

It installs for the current user, into `{autopf}`, so nothing asks for an administrator. The `AppId`
GUID must never change: Windows recognises an upgrade by it, and a new one would install a second
copy beside the first.

The version is passed in rather than written into the script:

```powershell
scripts/package-gui-windows.ps1 -Platform x64 -DownloadFfmpeg
scripts/package-gui-windows-installer.ps1 -Platform x64
```

## FFmpeg bundling

- Package layout is relative: `bin/ffmpeg.exe` and `bin/ffprobe.exe`. Never write
  developer-machine absolute paths into package files.
- `-DownloadFfmpeg` auto-downloads matching BtbN FFmpeg builds. Download caches record their
  source URL; explicit URL changes invalidate the cached archive; partial downloads never replace
  a valid cache; `-RefreshFfmpeg` remains available for mutable `latest` URLs.
- Do not ship a GUI package until `MyVidComp.exe`, `myvidcomp_core.dll`, `flutter_windows.dll`,
  FFmpeg/FFprobe, and any runtime DLLs all pass the package script's PE machine check for the
  selected architecture.

## CLI packaging

`scripts/package.sh` builds the CLI (`myvidcomp.exe`) for `x86_64-pc-windows-gnu` or, for Windows
ARM64, `aarch64-pc-windows-gnullvm` with an ARM64 FFmpeg/FFprobe bundle plus matching llvm-mingw
ARM64 runtime DLLs such as `libunwind.dll`. Do not ship Windows ARM64 `myvidcomp.exe` without checking
imported DLLs — llvm-mingw builds may need runtime DLLs beside the executable. Release packages
include `myvidcomp.exe`, optional `bin/` runtime tools, `config.yaml`, `README.md`, and
`SHA256SUMS.txt`.

## Package config template

`config.example.yaml` is copied to release `config.yaml`. It keeps `conversion_mode` and
`output_format` commented because older MyVidComp builds reject unknown config keys; only uncomment
them for builds that support the keys.

## Build marker

Every build embeds a `BUILD_MARKER` shown in `--version` output (e.g.
`chapter-carrier.20260721a`). It identifies the behavior generation of a packaged build.
