# Safety Model

MyVidComp never replaces a source video until conversion and validation have succeeded, and it never
touches the destination folder before the final commit.

## Temp files

- Long-running ffmpeg writes go to MyVidComp-owned temp files named
  `.myvidcomp-<stem>-<timestamp>.tmp.mp4` or `.tmp.mkv`.
- When `tmp_dir` is set, temp files live in that local directory (useful for SMB/NAS destinations);
  otherwise they sit next to the source.
- Cached `.myvidcomp-<stem>-*.tmp.mp4` / `.tmp.mkv` files may be reused only after the same validation
  path succeeds — never merely because the file exists.
- Unreadable MyVidComp temp outputs, especially files with no readable video stream, are deleted and
  not reused. Only files matching MyVidComp temp naming are ever deleted.

## Conflict detection before encoding

A file is skipped with a conflict reason when its output path, `.old` path, temp path, or
`.myvidcomp.recover` path already exists. Existing output files and `.old` files are never overwritten
silently.

## Commit

- Commit happens only after full validation.
- The validated temp output replaces the source via same-directory rename; cross-device moves
  fall back to copy-with-progress.
- With `--keep-original`, the original is renamed to `<name>.<ext>.old` first.
- The final commit temp path stays next to the destination so the last rename is same-volume.
- Recovery path naming (`.myvidcomp.recover`) supports manual recovery of interrupted commits.

## Originals

Real user videos are never overwritten, deleted, or renamed by tests — tests use synthetic
fixtures. Originals are preserved until conversion and validation have succeeded.

## Graceful stop

- CLI: type `q`, press Enter, then confirm with `y`.
- Progress refresh pauses while the confirmation prompt is active.
- The active ffmpeg process is never interrupted; MyVidComp finishes the current file and stops before
  starting the next one.
- The embedded API cancels through `CancellationToken::request_stop` with the same semantics.

## Config

`config.yaml` is user-owned input: MyVidComp reads it when needed but never rewrites or normalizes it.
The GUI stores its own preferences in the user's profile and must not rewrite `config.yaml`.
