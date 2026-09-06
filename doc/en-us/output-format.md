# Output Format and Container Selection

The `OutputFormat` policy is the typed container-policy boundary. MyVidComp outputs AV1 MP4 by default
and never leaves the MP4 path unless the user opts into conservative MKV fallback.

## The two formats

| Format | Wire value | Behavior |
|---|---|---|
| Strict MP4 (default) | `mp4` | Skips files whose mapped non-primary streams are outside the conservative MP4 copy whitelist, with a specific reason. |
| MP4 with MKV fallback | `mkv-fallback` | For such files, switches the output container to MKV only when every mapped non-primary stream type is conservatively Matroska-copy-compatible. |

Selection is made **after** confirmed QuickTime chapter carriers have been excluded from the
mapped stream set, so a chapter carrier never triggers MKV fallback — such files stay MP4.

## Conservative MP4 copy whitelist

The primary video stream is re-encoded to AV1, so it is not gated by this whitelist. Every other
mapped stream must be copy-compatible:

| Stream type | Whitelisted codecs |
|---|---|
| Video (non-primary) | `av1`, `h264`, `hevc`, `mpeg4`, `mjpeg`, `jpeg2000` |
| Audio | `aac`, `mp3`, `alac`, `ac3`, `eac3`, `flac`, `opus` |
| Subtitle | `mov_text` |

This is a deliberately conservative static baseline. Streams outside it — including ASS/SSA and
subrip subtitles, DTS/TrueHD/PCM audio, `timed_id3` data, attachments, and unknown stream types —
produce a specific skip reason with the stream index, codec, and (when known) handler name.

## MKV fallback rules

`mkv-fallback` does not mean "put anything into MKV". Fallback is selected only when **every**
mapped non-primary stream type is conservatively Matroska-copy-compatible, which today means:

| Stream type | Matroska fallback |
|---|---|
| `video` / `audio` / `subtitle` | allowed (e.g. ASS, subrip, DTS, TrueHD audio) |
| `data` / `attachment` / unknown | never — detailed safe skip |

Matroska capability is not proof that an arbitrary stream can be copied safely: arbitrary data,
attachment, and unknown streams remain detailed safe skips even with fallback enabled. When both
MP4 and MKV are unsafe, the skip reason reports the MP4 reason followed by the Matroska reason.

## Skip reasons

`SkipReason::UnsafeReplacement` carries a specific detail message:

- Incompatible streams: stream index, type, codec, and handler when available.
- Interlaced sources: the probed field order; MyVidComp does not deinterlace.
- Unmappable AV1 metadata: the specific metadata field.
- Chapter carrier classification failures: the atom-level or chapter-record problem.

## Selection surface

- CLI: `--output-format mp4|mkv-fallback`
- Config: `output_format` (commented in package templates for older builds)
- Embedded API: `RunOptions.output_format`
- GUI: the localized Output format selector, persisted in GUI settings (schema v3+)
- The embedding ABI: the `output_format` field of `FfiRunOptionsV1` (blank maps to `mp4`)

The selected format is reported through the `output_format_selected` event and the terminal
"Output format" line.
