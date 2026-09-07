# AGENTS.md — MyVidComp

Operating guide for agents working on this repository. This file holds **only** rules about how to
work here. Everything describing what the code *is* or *does* lives in `doc/en-us/` — see
[Where to read what](#where-to-read-what).

MyVidComp (`myvidcomp`) is a Rust command-line tool and embeddable engine that makes a folder of
videos smaller, measures how close each result looks to its source, and never deletes anything it
is not sure about. The application under `gui/` embeds the same engine and builds for Windows,
macOS, Linux and Android.

## Reading order

When you need to understand code, read in this order and stop as soon as you have what you need:

1. **`doc/en-us/`** — start here, always. `architecture.md` for shape and pipeline;
   `functions/<domain>.md` for declarations; `functions/INDEX.md` to find the right page; the
   concept docs for behavior.
2. **Comments in the source** — the `AI-FUNC-SUMMARY` layer above each declaration.
3. **The implementation** — only when the docs and comments are insufficient, or when you must
   confirm actual behavior before changing it.

Do not jump straight to reading source bodies. Where docs and code disagree on something you are
about to change, verify against the code, then fix the docs in the same commit.

## Where to read what

| Question | Read |
|---|---|
| What MyVidComp is, the pipeline, layout, media-tool lookup | `doc/en-us/architecture.md` |
| What does a declaration do | `doc/en-us/functions/<domain>.md` |
| Which page covers which domain | `doc/en-us/functions/INDEX.md` |
| Every setting and where it can be set | `doc/en-us/options.md` |
| How a quality setting is chosen and measured | `doc/en-us/quality.md` |
| Conversions kept for the user to decide about | `doc/en-us/review.md` |
| What differs between AV1, H.265 and H.266 | `doc/en-us/codecs.md` |
| MP4 whitelist, MKV fallback, skip reasons | `doc/en-us/output-format.md` |
| QuickTime chapter-carrier policy | `doc/en-us/chapter-carrier.md` |
| Encoder detection, retries, hidden processes | `doc/en-us/encoders.md` |
| Output validation rules | `doc/en-us/validation.md` |
| Temp files, cache reuse, commit, graceful stop | `doc/en-us/safety.md` |
| The embedding ABI and event JSON | `doc/en-us/ffi-abi.md` |
| Windows packaging and media-tool bundling | `doc/en-us/packaging.md` |
| The Android build and what it needs | `doc/en-us/android.md` |
| What runs automatically on each change | `doc/en-us/ci.md` |
| The application, its layout and its wording | `doc/en-us/gui.md` |
| Why a behavior exists; past changes | `doc/en-us/version-history.md` |
| English→Chinese terminology | `doc/en-us/translation-guide.md` |

## Required workflow

1. Treat the user's message as the modification request.
2. Read per [Reading order](#reading-order).
3. Plan when the work is non-trivial, then implement it in this workspace.
4. Keep changes scoped; do not revert unrelated work in the tree.
5. Update documentation in the same change set — see [Documentation maintenance](#documentation-maintenance).
6. Verify with the narrowest meaningful checks, usually `cargo fmt --check`,
   `cargo clippy --all-targets -- -D warnings`, and `cargo test`; add `flutter analyze`,
   `flutter test`, and `flutter build windows` for GUI changes.
7. Report briefly, in English and Chinese: what changed, what was verified, and anything that
   could not be done.
8. Ask before committing or pushing. The user decides when to release.

## Documentation maintenance

**Docs are the primary artifact. Update them first, and never let them drift.**

Any change that adds, removes, or changes the behavior or signature of a declaration, a policy, a
safety rule, or a packaging rule must update, in the same commit:

- the per-domain page under `doc/en-us/functions/` and its `INDEX.md` count,
- every affected concept doc under `doc/en-us/`.

Every language directory under `doc/` (currently `en-us` and `zh-cn`) mirrors the others exactly —
same files, headings, tables, and examples. `doc/en-us/` is authoritative: any documentation
change updates **all** language directories in the same commit, translated per
`translation-guide.md`. Adding a new language means creating a complete mirror of `doc/en-us/` in
the same change. New terminology goes into `translation-guide.md`'s glossary.

**Put explanation in the docs, not here.** This file is for agent instructions only. If you are
about to add a paragraph describing how the code works, it belongs in `doc/en-us/`. Only add to
this file when the rule is about how an agent should behave.

## Authoring rules

**Function Explanation Layer.** Every function carries an `AI-FUNC-SUMMARY` comment immediately
above it, written in English:

- `Purpose: <one short sentence describing what the declaration is responsible for>`
- `Inputs: <important parameters only; omit obvious ones if trivial>`
- `Returns: <what the caller receives, or "None">`
- `Side effects: <state changes, file/process/UI effects, logging, mutation, or "None">`
- `Notes: <important assumptions, edge cases, invariants, or when the declaration should be used;
  prefer none when there is nothing special to add>`

Use the compact single-line format for small functions and the full block format when a function
has non-trivial invariants, file/process side effects, or subtle safety behavior. Add one when
adding a declaration, and update it in the same change when editing an existing one. Do not add
explanatory comments outside `AI-FUNC-SUMMARY` unless the code would otherwise be difficult to
maintain. Treat summaries as navigation aids, not a substitute for reading code before
behavior-changing edits.

Other conventions:

- Prefer the Rust standard library unless a dependency clearly reduces risk or complexity.
- Keep modules behavior-oriented: CLI parsing, discovery, probing, quality estimation, ffmpeg
  command construction, transcoding, validation, commit, and UI/progress.
- Use explicit error messages that include affected paths or commands when useful.
- Optional ffmpeg capabilities must be detected dynamically.
- Tests that depend on optional ffmpeg filters or encoders should skip cleanly when unavailable.
- Do not overwrite, delete, or rename real user videos during tests. Use synthetic fixtures for
  destructive or replacement behavior.

## Behavior contract

Do not change these without the user explicitly deciding to. Details live in the concept docs;
these are the load-bearing summary rules:

- Preserve source resolution and frame rate. Do not add scale, crop, frame-rate conversion, or
  timestamp-changing filters unless the user explicitly asks for that behavior.
- Preserve originals until conversion and validation have succeeded.
- Treat `config.yaml` as user-owned input. Read it when needed, but never rewrite or normalize it.
- Keep media-tool lookup cross-platform: macOS and Linux use `ffmpeg` and `ffprobe`, Windows
  uses `ffmpeg.exe` and `ffprobe.exe`, and Android runs `libffmpeg.so` and `libffprobe.so`
  from the folder it unpacks native libraries into.
- Never add a field to `FfiRunOptionsV1`. A caller built against the old layout would read
  the wrong bytes. Add `FfiRunOptionsV2` and `myvidcomp_run_blocking_v2` beside it instead
  (see `doc/en-us/ffi-abi.md`).
- Graceful exit requests must pause progress refresh during confirmation and stop before starting
  the next file, not interrupt the active ffmpeg process.
- All Rust FFmpeg/FFprobe child processes must use the shared command constructor so Windows
  applies `CREATE_NO_WINDOW`.
- `scripts/package-gui-windows-all.ps1` is the supported dual-architecture release entry point;
  keep explicit `arm64`/`x64` build-directory selection and PE machine validation.
- Hardware decoding is never asked for automatically by ffmpeg. Name one method, prove it by
  creating its device and decoding two frames of the real file, retry on the processor when it
  fails, and stop using it for the rest of the run. `-hwaccel auto` was what crashed ffmpeg on
  roughly one run in six in 2026-09 testing, and a crash must never cost more than one encode.
- Never pass `-hwaccel_output_format` to a quality measurement. The comparison filter needs
  frames in ordinary memory, and leaving that option out is what brings them back.
- A conversion the tool is not confident about is kept beside its original, never committed
  over it. Both files survive until the user decides.

## Verification

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

For interface changes, also run `cd gui && dart format --set-exit-if-changed lib test`,
`flutter analyze`, `flutter test`, and `flutter build windows` on Windows. For Android
changes, `flutter build apk --release --target-platform android-arm64`.

For packaging changes, run `bash -n scripts/package.sh`, parse both PowerShell package scripts,
run
`powershell -NoProfile -ExecutionPolicy Bypass -File scripts/package-gui-windows.ps1 -Platform <arm64|x64> -SkipBuild -NoArchive`,
and rebuild both Windows bundles with `scripts/package-gui-windows-all.ps1 -DownloadFfmpeg` or
explicit matching FFmpeg directories.

For documentation changes, run `scripts/check-doc-parity.ps1`, which enforces the rules in
`translation-guide.md`: the same files, heading counts, table-row counts, and code blocks that
are byte-identical across both trees.

## Never commit

Secrets, credentials, user videos, packaged binaries under `dist/`, build outputs, or
developer-machine absolute paths in package files. Documentation-only commits do not change
versions.
