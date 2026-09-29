# Safety Model

MyVidComp never replaces a source video until conversion and validation have succeeded, and it never
touches the destination folder before the final commit.

## Temp files

- Long-running ffmpeg writes go to MyVidComp-owned temp files named
  `.myvidcomp-<stem>-<timestamp>.tmp.mp4` or `.tmp.mkv`.
- When `tmp_dir` is set, temp files live in that local directory (useful for SMB/NAS destinations);
  otherwise they sit next to the source.
- A cached temp is recognized only by its exact name, `.myvidcomp-<stem>-<digits>.tmp.mp4` or
  `.tmp.mkv` (or the legacy `.pvac-` prefix), so `clip` never claims a temp of `clip-b` and a
  `.repairing` leftover is never taken for a finished temp.
- Cached temps may be reused only after the same validation path succeeds — never merely because
  the file exists — against the codec this run asked for and with a duration tolerance ten times
  tighter than a fresh encode's (see [validation.md](validation.md)). A reused temp is still
  measured and can still go to review.
- A stale `.repairing` file from a stopped metadata repair is removed before the next repair
  starts, so it cannot block that temp's repair forever.
- Unreadable MyVidComp temp outputs, especially files with no readable video stream or the wrong
  codec, are deleted and not reused. Only files matching MyVidComp temp naming are ever deleted.
- If the engine loses track of a running ffmpeg (its progress can no longer be read), it kills and
  reaps that ffmpeg before reporting the error, so no orphan keeps writing a temp file.

## Conflict detection before encoding

A file is skipped with a conflict reason when its output path, `.old` path, temp path, or
`.myvidcomp.recover` path already exists. Existing output files and `.old` files are never overwritten
silently.

An output name that differs from the source only in letter case (`clip.MP4` becomes `clip.mp4`)
is not a conflict when both names are the same file, as they are on a disk that ignores letter
case. Two paths count as the same file only when both exist, neither is a symbolic link, and
(on Linux, macOS and Android) they share device and inode with a single link, or (on Windows)
they resolve to the same full path. On a disk that tells case apart, a separate `clip.mp4`
beside `clip.MP4` is still a conflict.

## Commit

- Commit happens only after full validation.
- The validated temp output replaces the source via same-directory rename; cross-device moves
  fall back to copy-with-progress.
- With `--keep-original`, the original is renamed to `<name>.<ext>.old` first.
- The final commit temp path stays next to the destination so the last rename is same-volume.
- Recovery path naming (`.myvidcomp.recover`) supports manual recovery of interrupted commits.
- An output name that is the source itself, including the differently-cased `clip.mp4` for
  `clip.MP4`, is committed through the recovery rename: the source moves to its recovery name,
  the converted file takes the output name, and only then is the recovery copy removed. Whether
  the two names are one file is decided before anything moves, because once the source is renamed
  aside the check can no longer see it; treating the names as separate would delete the converted
  file right after writing it.
- When the copy fallback fails part-way, the partial staging file is removed.

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
