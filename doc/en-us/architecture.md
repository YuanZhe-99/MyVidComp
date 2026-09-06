# Architecture

## What MyVidComp is

MyVidComp (`myvidcomp`) is a Rust command-line tool and embeddable engine that makes a folder of
videos smaller, measures how close each result looks to its source, and never deletes anything it
is not sure about. The application under `gui/` embeds the same engine and consumes structured
events rather than parsing terminal output. The engine runs `ffmpeg` and `ffprobe` as separate
programs; they are bundled or supplied by the caller.

## The conversion pipeline

Each candidate file passes through one ordered pipeline:

1. **Discovery** — directory entries and filename extensions only; no probing. Scans of network
   drives stay lightweight. Files waiting for a review decision are skipped here.
2. **Probing** — `ffprobe` reads the primary video properties (`VideoInfo`), the stream layout
   (`StreamInfo` with actual index, codec, codec tag, track ID, handler), and chapter metadata.
   Files already in the target codec are skipped here.
3. **Stream policy** — possible QuickTime chapter carriers are classified against bounded ISO BMFF
   `tref/chap` evidence (see [chapter-carrier.md](chapter-carrier.md)); the mapped stream set and
   the chapter policy are fixed before anything else.
4. **Container selection** — strict MP4 or conservative MKV fallback (see
   [output-format.md](output-format.md)).
5. **Planning** — the ordered list of encode attempts for this file, built from the encoder
   preference, the preservation policy and the target codec (see [options.md](options.md)).
6. **Quality selection** — either a tuning search over short samples, or an estimate derived from
   the source (see [quality.md](quality.md)).
7. **Transcode** — one `ffmpeg` run per attempt with explicit `-map 0:<index>` arguments, progress
   parsing, and hidden Windows child processes (see [encoders.md](encoders.md)).
8. **Validation** — probed output is compared against the expected video properties, duration,
   stream signature, and chapter semantics (see [validation.md](validation.md)). Optional metadata
   repair runs before final validation when classified as remux-repairable.
9. **Measurement** — the result is compared against its source (see [quality.md](quality.md)).
10. **Commit** — a result that changed nothing and met its target replaces the source, with `.old`
    backups and cross-device copy fallback. Anything else is written beside the original for the
    user to decide about (see [review.md](review.md) and [safety.md](safety.md)).

## Repository layout

```text
src/main.rs      Minimal binary entry point.
src/lib.rs       The pipeline: discovery, probing, planning, transcoding, validation, commit.
src/options.rs   Every user-facing option and its wire value.
src/codec.rs     What differs between AV1, H.265 and H.266.
src/vmaf.rs      Quality measurement.
src/search.rs    Choosing a quality setting by sampling.
src/review.rs    Conversions kept for the user to decide about.
src/ffi.rs       The embedding ABI.
gui/             The application, for Windows, macOS, Linux and Android.
scripts/         Packaging and build helpers.
config.example.yaml  Default settings template copied into release packages.
doc/en-us/       Authoritative documentation (this tree).
doc/zh-cn/       Simplified Chinese mirror.
README.md        User-facing quickstart, build, runtime, and output rules.
AGENTS.md        Agent working rules only; behaviour lives in doc/.
```

The command line and the embedded interface share the same workflow through `run_with_events`. The
pipeline itself is one file, so the function documentation under [functions/](functions/INDEX.md)
is grouped by behaviour domain rather than mirroring source paths.

## Media-tool lookup

MyVidComp needs `ffmpeg` and `ffprobe`. It searches next to the executable, then a sibling `bin/`
directory, then `PATH`. macOS and Linux use `ffmpeg` and `ffprobe`; Windows uses `ffmpeg.exe` and
`ffprobe.exe`; Android runs them from the folder it unpacks native libraries into, where they are
named `libffmpeg.so` and `libffprobe.so` (see [android.md](android.md)).

Optional capabilities are detected dynamically and never trusted from the advertised encoder list
alone: every encoder must also pass a synthetic runtime encode check, and quality measurement is
only offered when the media tools actually provide it.

## Config

`config.yaml` is user-owned input: MyVidComp reads it when needed but never rewrites or normalizes
it. Command-line values override config values. A key present with a blank value means "use the
default", so a shipped template can list every key without committing to any of them.

## Safety model

Originals are preserved until conversion and validation have succeeded. Long-running ffmpeg writes
go to temp files this tool owns (`tmp_dir` when configured), and the destination folder is touched
only during final commit. Validated temp outputs are reused from cache, including ones an older
install left behind; unreadable ones are removed. A conversion the tool is not confident about is
kept beside its original rather than replacing it. Details and invariants are in
[safety.md](safety.md).

## Documentation maintenance

Each new behaviour area must gain `doc/en-us/` pages, and the matching `doc/zh-cn/` translations,
in the same change set. See this repository's `AGENTS.md` for the exact rule.
