#!/usr/bin/env bash
set -euo pipefail

target="x86_64-pc-windows-gnu"
out_dir="dist"
ffmpeg_dir=""
runtime_dir=""

# AI-FUNC-SUMMARY: Prints package script usage; returns none; side effects: writes help text to stdout.
usage() {
  printf 'Usage: %s [--target TARGET] [--out-dir DIR] [--with-ffmpeg DIR] [--with-runtime-dir DIR]\n' "$0"
  printf '\n'
  printf 'Examples:\n'
  printf '  %s --target x86_64-pc-windows-gnu\n' "$0"
  printf '  %s --target x86_64-pc-windows-gnu --with-ffmpeg /opt/ffmpeg/bin\n' "$0"
  printf '  %s --target aarch64-pc-windows-gnullvm --with-ffmpeg /opt/ffmpeg-winarm64/bin --with-runtime-dir /opt/llvm-mingw/aarch64-w64-mingw32/bin\n' "$0"
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --target)
      target="${2:?--target requires a value}"
      shift 2
      ;;
    --out-dir)
      out_dir="${2:?--out-dir requires a value}"
      shift 2
      ;;
    --with-ffmpeg)
      ffmpeg_dir="${2:?--with-ffmpeg requires a directory}"
      shift 2
      ;;
    --with-runtime-dir)
      runtime_dir="${2:?--with-runtime-dir requires a directory}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      printf 'unknown option: %s\n\n' "$1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

cargo_cmd=()
if [[ -n "${CARGO:-}" ]]; then
  cargo_cmd=("$CARGO")
elif command -v cargo >/dev/null 2>&1; then
  cargo_cmd=(cargo)
elif [[ -x "$HOME/.cargo/bin/rustup" ]]; then
  cargo_cmd=("$HOME/.cargo/bin/rustup" run stable cargo)
else
  printf 'cargo was not found. Install Rust with rustup, then source ~/.bashrc.\n' >&2
  exit 1
fi

if [[ "$target" == *windows* || "$target" == *-pc-windows-* ]]; then
  exe_name="myvidcomp.exe"
  ffmpeg_name="ffmpeg.exe"
  ffprobe_name="ffprobe.exe"
  archive_ext="zip"
else
  exe_name="myvidcomp"
  ffmpeg_name="ffmpeg"
  ffprobe_name="ffprobe"
  archive_ext="tar.gz"
fi

# AI-FUNC-SUMMARY: Finds a runtime tool under a root or bin child directory; returns tool path; side effects: exits on missing tool.
find_runtime_tool() {
  local root="$1"
  local name="$2"

  if [[ -f "$root/$name" ]]; then
    printf '%s\n' "$root/$name"
  elif [[ -f "$root/bin/$name" ]]; then
    printf '%s\n' "$root/bin/$name"
  else
    printf 'missing %s under %s or %s/bin\n' "$name" "$root" "$root" >&2
    exit 1
  fi
}

"${cargo_cmd[@]}" build --release --target "$target"

package="myvidcomp-cli-$target"
stage="$out_dir/$package"
binary="target/$target/release/$exe_name"

if [[ ! -f "$binary" ]]; then
  printf 'expected build output not found: %s\n' "$binary" >&2
  exit 1
fi

rm -rf "$stage" "$out_dir/$package.zip" "$out_dir/$package.tar.gz"
mkdir -p "$stage"
cp "$binary" "$stage/$exe_name"

if [[ -f README.md ]]; then
  cp README.md "$stage/README.md"
fi

if [[ -f config.example.yaml ]]; then
  cp config.example.yaml "$stage/config.yaml"
fi

if [[ -n "$ffmpeg_dir" ]]; then
  mkdir -p "$stage/bin"
  cp "$(find_runtime_tool "$ffmpeg_dir" "$ffmpeg_name")" "$stage/bin/$ffmpeg_name"
  cp "$(find_runtime_tool "$ffmpeg_dir" "$ffprobe_name")" "$stage/bin/$ffprobe_name"
fi

if [[ -n "$runtime_dir" ]]; then
  shopt -s nullglob
  runtime_dlls=("$runtime_dir"/*.dll)
  shopt -u nullglob
  if [[ ${#runtime_dlls[@]} -eq 0 ]]; then
    printf 'missing runtime DLLs under %s\n' "$runtime_dir" >&2
    exit 1
  fi
  for dll in "${runtime_dlls[@]}"; do
    cp "$dll" "$stage/$(basename "$dll")"
  done
fi

checksum_files=("$exe_name")
if [[ -f "$stage/README.md" ]]; then
  checksum_files+=("README.md")
fi
if [[ -f "$stage/config.yaml" ]]; then
  checksum_files+=("config.yaml")
fi
if [[ -f "$stage/bin/$ffmpeg_name" ]]; then
  checksum_files+=("bin/$ffmpeg_name")
fi
if [[ -f "$stage/bin/$ffprobe_name" ]]; then
  checksum_files+=("bin/$ffprobe_name")
fi
shopt -s nullglob
for dll in "$stage"/*.dll; do
  checksum_files+=("$(basename "$dll")")
done
shopt -u nullglob

(cd "$stage" && sha256sum "${checksum_files[@]}" > SHA256SUMS.txt)

if [[ "$archive_ext" == "zip" ]]; then
  (cd "$out_dir" && zip -qr "$package.zip" "$package")
  printf 'Wrote %s\n' "$out_dir/$package.zip"
else
  tar -C "$out_dir" -czf "$out_dir/$package.tar.gz" "$package"
  printf 'Wrote %s\n' "$out_dir/$package.tar.gz"
fi
