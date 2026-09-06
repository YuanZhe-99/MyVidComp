#!/usr/bin/env bash
#
# Builds the FFmpeg tools MyVidComp needs on Android, from source.
#
# No published FFmpeg build for Android carries everything this project depends
# on, so the shipped binaries are built here. The result is two self-contained
# programs installed into the application's native library folder as
# libffmpeg.so and libffprobe.so, which is the only place Android will run a
# program from.
#
# The build needs a Linux host with clang, cmake, ninja, meson and make, plus an
# Android NDK sysroot. It does not need the NDK's own compiler: any clang recent
# enough to target Android will do, which is what makes this work on an Arm
# Linux host, where Google publishes no NDK at all.
#
# Usage:
#   scripts/build-android-ffmpeg.sh --sysroot <ndk>/toolchains/llvm/prebuilt/<host>/sysroot
#
set -euo pipefail

ABI="arm64-v8a"
API=29
JOBS="$(nproc 2>/dev/null || echo 4)"
SYSROOT=""
RUNTIME_LIBS=""
OUT=""

while [ $# -gt 0 ]; do
    case "$1" in
        --sysroot) SYSROOT="$2"; shift 2 ;;
        --runtime-libs) RUNTIME_LIBS="$2"; shift 2 ;;
        --abi) ABI="$2"; shift 2 ;;
        --api) API="$2"; shift 2 ;;
        --jobs) JOBS="$2"; shift 2 ;;
        --out) OUT="$2"; shift 2 ;;
        -h|--help) sed -n '2,18p' "$0"; exit 0 ;;
        *) echo "Unknown argument: $1" >&2; exit 2 ;;
    esac
done

case "$ABI" in
    arm64-v8a) TRIPLE="aarch64-linux-android"; FF_ARCH="aarch64"; FF_CPU="armv8-a" ;;
    x86_64)    TRIPLE="x86_64-linux-android";  FF_ARCH="x86_64";  FF_CPU="x86-64" ;;
    *) echo "Unsupported ABI: $ABI" >&2; exit 2 ;;
esac

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
WORK="${MYVIDCOMP_ANDROID_BUILD_DIR:-$HOME/.cache/myvidcomp-android-ffmpeg}/$ABI"
SRC="$WORK/src"
PREFIX="$WORK/prefix"
BIN="$WORK/bin"
[ -n "$OUT" ] || OUT="$REPO_ROOT/gui/android/app/src/main/jniLibs/$ABI"

# Pinned, so that rebuilding produces the same tools. Update deliberately.
FFMPEG_VERSION="9.0.1"
VMAF_VERSION="v3.2.0"
X265_VERSION="4.1"
SVTAV1_VERSION="v3.1.2"
VVENC_VERSION="v1.14.0"
DAV1D_VERSION="1.5.4"

if [ -z "$SYSROOT" ]; then
    for guess in "${ANDROID_NDK_HOME:-}" "${ANDROID_NDK_ROOT:-}" "$HOME/android-ffmpeg"; do
        [ -n "$guess" ] || continue
        if [ -d "$guess/sysroot/usr/include" ]; then SYSROOT="$guess/sysroot"; break; fi
        for candidate in "$guess"/toolchains/llvm/prebuilt/*/sysroot; do
            if [ -d "$candidate/usr/include" ]; then SYSROOT="$candidate"; break 2; fi
        done
    done
fi
if [ -z "$SYSROOT" ] || [ ! -d "$SYSROOT/usr/include" ]; then
    echo "Could not find the Android sysroot. Pass --sysroot <ndk>/toolchains/llvm/prebuilt/<host>/sysroot." >&2
    exit 1
fi

if [ -f "$OUT/libffmpeg.so" ] && [ -f "$OUT/libffprobe.so" ] &&
   [ -z "${MYVIDCOMP_FORCE_REBUILD:-}" ]; then
    echo "The tools are already installed in $OUT."
    echo "Set MYVIDCOMP_FORCE_REBUILD=1 to build them again."
    exit 0
fi

mkdir -p "$SRC" "$PREFIX/lib/pkgconfig" "$BIN"

# Android programs are linked against compiler-rt and LLVM's unwinder, not
# libgcc. A clang that was not built for Android reaches for libgcc and fails at
# the first link, so it is given the NDK's runtime libraries instead. They are
# merged with the host compiler's own resource folder, which keeps that
# compiler's headers and only replaces what has to come from the NDK.
if [ -z "$RUNTIME_LIBS" ]; then
    for candidate in "$SYSROOT"/../lib/clang/*/lib/linux; do
        if [ -d "$candidate" ]; then RUNTIME_LIBS="$candidate"; break; fi
    done
fi
if [ -z "$RUNTIME_LIBS" ] || [ ! -d "$RUNTIME_LIBS" ]; then
    echo "Could not find the NDK runtime libraries. Pass --runtime-libs <ndk>/toolchains/llvm/prebuilt/<host>/lib/clang/<version>/lib/linux." >&2
    exit 1
fi

RESOURCE_DIR="$WORK/resource"
if [ ! -f "$RESOURCE_DIR/lib/linux/libclang_rt.builtins-$FF_ARCH-android.a" ]; then
    rm -rf "$RESOURCE_DIR"
    mkdir -p "$RESOURCE_DIR"
    cp -r "$(clang -print-resource-dir)/." "$RESOURCE_DIR/"
    mkdir -p "$RESOURCE_DIR/lib"
    cp -r "$RUNTIME_LIBS/." "$RESOURCE_DIR/lib/linux/"
fi

# -Qunused-arguments: the two runtime choices matter only when linking, and
# several build systems compile their probe files with unused arguments treated
# as an error.
RUNTIME_FLAGS="-resource-dir=$RESOURCE_DIR -rtlib=compiler-rt --unwindlib=libunwind -Qunused-arguments"

# Small wrapper scripts, so cmake, meson and FFmpeg's configure can all be
# pointed at one compiler name and get the same target, sysroot and runtime.
write_wrapper() {
    local name="$1" tool="$2" extra="$3"
    {
        echo '#!/bin/sh'
        echo "exec $tool --target=$TRIPLE$API --sysroot=$SYSROOT $RUNTIME_FLAGS $extra \"\$@\""
    } > "$BIN/$name"
    chmod +x "$BIN/$name"
}
write_wrapper "$TRIPLE$API-clang" clang ""
# Android ships no C++ runtime for a program to link against, so the one the
# encoders need is built into the binaries.
write_wrapper "$TRIPLE$API-clang++" clang++ "--stdlib=libc++ -static-libstdc++"
ln -sf "$BIN/$TRIPLE$API-clang" "$BIN/$TRIPLE-gcc"
ln -sf "$BIN/$TRIPLE$API-clang" "$BIN/$TRIPLE-clang"
ln -sf "$BIN/$TRIPLE$API-clang++" "$BIN/$TRIPLE-clang++"
for tool in ar ranlib nm strip; do
    ln -sf "$(command -v "llvm-$tool")" "$BIN/$TRIPLE-$tool"
done

export PATH="$BIN:$PATH"
export PKG_CONFIG_LIBDIR="$PREFIX/lib/pkgconfig"
export PKG_CONFIG_PATH="$PREFIX/lib/pkgconfig"

CC="$TRIPLE$API-clang"
CXX="$TRIPLE$API-clang++"
CFLAGS="-O3 -fPIC -DANDROID -I$PREFIX/include"
CXXFLAGS="$CFLAGS"
# Android 15 and later run 64-bit applications on 16 KB memory pages. A program
# linked for 4 KB pages will not load at all on those devices.
LDFLAGS="-fuse-ld=lld -Wl,-z,max-page-size=16384 -L$PREFIX/lib"

fetch() {
    local url="$1" name="$2"
    if [ ! -f "$SRC/$name" ]; then
        echo "==> downloading $name"
        curl -fsSL "$url" -o "$SRC/$name.partial"
        mv "$SRC/$name.partial" "$SRC/$name"
    fi
}

unpack() {
    local archive="$SRC/$1" dir="$SRC/$2"
    if [ ! -d "$dir" ]; then
        echo "==> unpacking $1"
        mkdir -p "$dir"
        tar -xf "$archive" -C "$dir" --strip-components=1
    fi
}

CMAKE_TOOLCHAIN="$WORK/android.cmake"
{
    echo "set(CMAKE_SYSTEM_NAME Android)"
    echo "set(CMAKE_SYSTEM_VERSION $API)"
    echo "set(CMAKE_SYSTEM_PROCESSOR $FF_ARCH)"
    echo "set(CMAKE_ANDROID_ARCH_ABI $ABI)"
    echo "set(CMAKE_SYSROOT $SYSROOT)"
    echo "set(CMAKE_C_COMPILER $BIN/$TRIPLE$API-clang)"
    echo "set(CMAKE_CXX_COMPILER $BIN/$TRIPLE$API-clang++)"
    echo "set(CMAKE_AR $(command -v llvm-ar))"
    echo "set(CMAKE_RANLIB $(command -v llvm-ranlib))"
    echo "set(CMAKE_C_COMPILER_TARGET $TRIPLE$API)"
    echo "set(CMAKE_CXX_COMPILER_TARGET $TRIPLE$API)"
    echo "set(CMAKE_FIND_ROOT_PATH $PREFIX $SYSROOT)"
    echo "set(CMAKE_FIND_ROOT_PATH_MODE_PROGRAM NEVER)"
    echo "set(CMAKE_FIND_ROOT_PATH_MODE_LIBRARY ONLY)"
    echo "set(CMAKE_FIND_ROOT_PATH_MODE_INCLUDE ONLY)"
    echo "set(CMAKE_POSITION_INDEPENDENT_CODE ON)"
} > "$CMAKE_TOOLCHAIN"

MESON_CROSS="$WORK/android.meson"
{
    echo "[binaries]"
    echo "c = '$BIN/$TRIPLE$API-clang'"
    echo "cpp = '$BIN/$TRIPLE$API-clang++'"
    echo "ar = '$(command -v llvm-ar)'"
    echo "strip = '$(command -v llvm-strip)'"
    echo "pkg-config = '$(command -v pkg-config)'"
    echo ""
    echo "[built-in options]"
    echo "c_args = ['-O3', '-fPIC']"
    echo "cpp_args = ['-O3', '-fPIC']"
    echo "c_link_args = ['-fuse-ld=lld', '-Wl,-z,max-page-size=16384']"
    echo "cpp_link_args = ['-fuse-ld=lld', '-Wl,-z,max-page-size=16384']"
    echo ""
    echo "[host_machine]"
    echo "system = 'android'"
    echo "cpu_family = '$FF_ARCH'"
    echo "cpu = '$FF_ARCH'"
    echo "endian = 'little'"
} > "$MESON_CROSS"

build_vmaf() {
    [ -f "$PREFIX/lib/pkgconfig/libvmaf.pc" ] && return 0
    fetch "https://github.com/Netflix/vmaf/archive/refs/tags/$VMAF_VERSION.tar.gz" "vmaf.tar.gz"
    unpack "vmaf.tar.gz" "vmaf"
    echo "==> building libvmaf"
    rm -rf "$SRC/vmaf/libvmaf/build"
    meson setup "$SRC/vmaf/libvmaf/build" "$SRC/vmaf/libvmaf" \
        --cross-file "$MESON_CROSS" --prefix "$PREFIX" \
        --buildtype release --default-library static \
        -Denable_tests=false -Denable_docs=false -Dbuilt_in_models=true \
        -Denable_tools=false
    ninja -C "$SRC/vmaf/libvmaf/build" -j "$JOBS"
    ninja -C "$SRC/vmaf/libvmaf/build" install
}

build_dav1d() {
    [ -f "$PREFIX/lib/pkgconfig/dav1d.pc" ] && return 0
    fetch "https://code.videolan.org/videolan/dav1d/-/archive/$DAV1D_VERSION/dav1d-$DAV1D_VERSION.tar.gz" "dav1d.tar.gz"
    unpack "dav1d.tar.gz" "dav1d"
    echo "==> building dav1d"
    rm -rf "$SRC/dav1d/build"
    meson setup "$SRC/dav1d/build" "$SRC/dav1d" \
        --cross-file "$MESON_CROSS" --prefix "$PREFIX" \
        --buildtype release --default-library static \
        -Denable_tools=false -Denable_tests=false
    ninja -C "$SRC/dav1d/build" -j "$JOBS"
    ninja -C "$SRC/dav1d/build" install
}

# x265 holds one bit depth per library, so encoding 10-bit sources needs the 10-
# and 12-bit builds linked into the 8-bit one. Every distribution does this; the
# steps below reproduce it.
build_x265() {
    [ -f "$PREFIX/lib/pkgconfig/x265.pc" ] && return 0
    fetch "https://bitbucket.org/multicoreware/x265_git/downloads/x265_$X265_VERSION.tar.gz" "x265.tar.gz"
    unpack "x265.tar.gz" "x265"
    # x265 asks for the pre-3.0 behaviour of two CMake policies that CMake 4
    # removed outright. Dropping the requests leaves the current behaviour,
    # which is what every distribution shipping x265 on CMake 4 does.
    sed -i '/cmake_policy(SET CMP0025 OLD)/d;/cmake_policy(SET CMP0054 OLD)/d' \
        "$SRC/x265/source/CMakeLists.txt"
    echo "==> building x265"
    local common=(
        -G Ninja
        -DCMAKE_TOOLCHAIN_FILE="$CMAKE_TOOLCHAIN"
        # x265 still declares support for CMake versions that current CMake
        # refuses to emulate, and stops before configuring anything without
        # this.
        -DCMAKE_POLICY_VERSION_MINIMUM=3.5
        -DCMAKE_BUILD_TYPE=Release
        -DCMAKE_INSTALL_PREFIX="$PREFIX"
        -DENABLE_SHARED=OFF
        -DENABLE_CLI=OFF
    )
    for depth in 12 10; do
        local main12=OFF
        [ "$depth" = 12 ] && main12=ON
        rm -rf "$SRC/x265/build$depth"
        cmake -S "$SRC/x265/source" -B "$SRC/x265/build$depth" "${common[@]}" \
            -DHIGH_BIT_DEPTH=ON -DMAIN12="$main12" -DEXPORT_C_API=OFF
        ninja -C "$SRC/x265/build$depth" -j "$JOBS"
        cp "$SRC/x265/build$depth/libx265.a" "$SRC/x265/libx265_main$depth.a"
    done
    rm -rf "$SRC/x265/build8"
    cmake -S "$SRC/x265/source" -B "$SRC/x265/build8" "${common[@]}" \
        -DEXTRA_LIB="$SRC/x265/libx265_main10.a;$SRC/x265/libx265_main12.a" \
        -DEXTRA_LINK_FLAGS=-L. -DLINKED_10BIT=ON -DLINKED_12BIT=ON
    ninja -C "$SRC/x265/build8" -j "$JOBS"
    ninja -C "$SRC/x265/build8" install
    # What gets installed is only the 8-bit half until the other two archives
    # are merged into it.
    local script="$WORK/x265-merge.mri"
    {
        echo "CREATE $SRC/x265/libx265_full.a"
        echo "ADDLIB $PREFIX/lib/libx265.a"
        echo "ADDLIB $SRC/x265/libx265_main10.a"
        echo "ADDLIB $SRC/x265/libx265_main12.a"
        echo "SAVE"
        echo "END"
    } > "$script"
    llvm-ar -M < "$script"
    mv "$SRC/x265/libx265_full.a" "$PREFIX/lib/libx265.a"
    llvm-ranlib "$PREFIX/lib/libx265.a"
}

build_svtav1() {
    [ -f "$PREFIX/lib/pkgconfig/SvtAv1Enc.pc" ] && return 0
    fetch "https://gitlab.com/AOMediaCodec/SVT-AV1/-/archive/$SVTAV1_VERSION/SVT-AV1-$SVTAV1_VERSION.tar.gz" "svtav1.tar.gz"
    unpack "svtav1.tar.gz" "svtav1"
    echo "==> building SVT-AV1"
    rm -rf "$SRC/svtav1/build"
    cmake -S "$SRC/svtav1" -B "$SRC/svtav1/build" -G Ninja \
        -DCMAKE_TOOLCHAIN_FILE="$CMAKE_TOOLCHAIN" \
        -DCMAKE_BUILD_TYPE=Release -DCMAKE_INSTALL_PREFIX="$PREFIX" \
        -DBUILD_SHARED_LIBS=OFF -DBUILD_APPS=OFF -DBUILD_TESTING=OFF
    ninja -C "$SRC/svtav1/build" -j "$JOBS"
    ninja -C "$SRC/svtav1/build" install
}

build_vvenc() {
    [ -f "$PREFIX/lib/pkgconfig/libvvenc.pc" ] && return 0
    fetch "https://github.com/fraunhoferhhi/vvenc/archive/refs/tags/$VVENC_VERSION.tar.gz" "vvenc.tar.gz"
    unpack "vvenc.tar.gz" "vvenc"
    echo "==> building vvenc"
    rm -rf "$SRC/vvenc/build"
    cmake -S "$SRC/vvenc" -B "$SRC/vvenc/build" -G Ninja \
        -DCMAKE_TOOLCHAIN_FILE="$CMAKE_TOOLCHAIN" \
        -DCMAKE_BUILD_TYPE=Release -DCMAKE_INSTALL_PREFIX="$PREFIX" \
        -DBUILD_SHARED_LIBS=OFF -DVVENC_ENABLE_LINK_TIME_OPT=OFF \
        -DVVENC_ENABLE_INSTALL=ON
    ninja -C "$SRC/vvenc/build" -j "$JOBS"
    ninja -C "$SRC/vvenc/build" install
}

# Two things CMake writes into its pkg-config files have to go before FFmpeg
# reads them. It records the unwinder as a file name rather than a library name,
# which then gets a second "-l" in front of it and makes every library look
# broken. And it asks for the shared C++ runtime, which no Android device has;
# the static one is linked in instead, further down.
sanitize_pkgconfig() {
    sed -i 's/-l-l:libunwind\.a//g; s/-lc++\([^_a-z]\|$\)/\1/g' \
        "$PREFIX"/lib/pkgconfig/*.pc
}

build_ffmpeg() {
    fetch "https://ffmpeg.org/releases/ffmpeg-$FFMPEG_VERSION.tar.xz" "ffmpeg.tar.xz"
    unpack "ffmpeg.tar.xz" "ffmpeg"
    echo "==> building ffmpeg"
    rm -rf "$SRC/ffmpeg/build"
    mkdir -p "$SRC/ffmpeg/build"
    cd "$SRC/ffmpeg/build"
    # MediaCodec is what reaches the phone's own video hardware, whoever made
    # it. Qualcomm, MediaTek, Samsung and Google all expose their encoders and
    # decoders through it and through nothing else.
    ../configure \
        --prefix="$PREFIX" \
        --enable-cross-compile \
        --cross-prefix="$TRIPLE-" \
        --cc="$CC" --cxx="$CXX" \
        `# Left alone, configure looks for a pkg-config named after the cross`\
        `# prefix, which does not exist. Everything it needs is static, so it`\
        `# also has to be told to ask for the libraries those libraries need.`\
        --pkg-config=pkg-config \
        --pkg-config-flags=--static \
        --target-os=android \
        --arch="$FF_ARCH" \
        --cpu="$FF_CPU" \
        --sysroot="$SYSROOT" \
        --enable-jni \
        --enable-mediacodec \
        --enable-gpl \
        --enable-version3 \
        --enable-libvmaf \
        --enable-libx265 \
        --enable-libsvtav1 \
        --enable-libvvenc \
        --enable-libdav1d \
        --enable-pic \
        --enable-static --disable-shared \
        --disable-doc --disable-debug --disable-ffplay \
        --extra-cflags="$CFLAGS" \
        --extra-cxxflags="$CXXFLAGS" \
        --extra-ldflags="$LDFLAGS" \
        --extra-libs="-lc++_static -lc++abi -lm -ldl -llog"
    make -j "$JOBS"
    cd "$REPO_ROOT"
}

build_vmaf
build_dav1d
build_x265
build_svtav1
build_vvenc
sanitize_pkgconfig
build_ffmpeg

mkdir -p "$OUT"
for tool in ffmpeg ffprobe; do
    built="$SRC/ffmpeg/build/$tool"
    [ -f "$built" ] || { echo "$tool was not built" >&2; exit 1; }
    # The library name is what lets Android unpack the file and run it.
    llvm-strip -o "$OUT/lib$tool.so" "$built"
    printf '%-8s %s MB\n' "$tool" "$(( $(stat -c %s "$OUT/lib$tool.so") / 1048576 ))"
done

echo
echo "Installed into $OUT"
