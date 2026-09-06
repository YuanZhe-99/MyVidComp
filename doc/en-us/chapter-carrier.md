# QuickTime Chapter Carrier Handling

Some MP4/MOV sources carry chapter metadata in a dedicated data track — typically a `bin_data`
stream with FourCC `text` — that audio/video tracks reference through `tref/chap` track
references. FFmpeg cannot copy such a track into MP4 (it would be rewritten as an unrelated
`gpmd`-style tag) or into Matroska, and it must not be silently dropped based on the codec name
alone. MyVidComp handles it evidence-based.

## Candidate criteria

A stream is a possible chapter carrier only when **all** of these hold:

- `codec_type` is `data` and `codec_name` is `bin_data`
- `codec_tag_string` is `text`
- it has a numeric track ID (`id` parsed from decimal or `0x` hex)

Anything else — `timed_id3`, attachments, telemetry, unknown streams — is never treated as a
carrier and falls through to the ordinary container-selection path.

## Confirmation: bounded ISO BMFF reading

For a file with carrier candidates, MyVidComp reads the source's ISO BMFF structure with a bounded,
standard-library-only reader to collect track IDs explicitly targeted by `tref/chap`:

- Reads box headers through `moov/trak/tkhd/tref/chap` only; **never reads `mdat`** and never
  loads media payload.
- Supports ordinary 32-bit sizes, extended 64-bit sizes, and parent-ending `size = 0` boxes.
- Validates every box against its parent bound and a global box-count budget.
- Reads `tkhd` track IDs (version 0 and 1) and the `chap` target-ID list.

A stream is a **confirmed carrier** when its probed track ID is targeted by `tref/chap`. The
parser is strict: malformed or ambiguous atoms, multiple `moov`/`tkhd` boxes, zero or invalid
target IDs, or budget overflow produce a specific per-file skip — never a silent drop, never an
abort of the batch.

## Policies for a confirmed carrier

The original carrier track is excluded from mapped streams and from the expected stream
signature. Chapter handling then follows the probed chapter metadata:

| Situation | Policy | FFmpeg | Validation |
|---|---|---|---|
| Meaningful chapters | `ConfirmedMeaningful` | `-map_chapters 0` | count, timestamps (0.05 s tolerance), exact titles |
| Empty or one unnamed full-duration placeholder | `ConfirmedEmpty` | `-map_chapters -1` | no chapters present |
| Ordinary sources (no carrier) | `Ordinary` | `-map_chapters -1` | none (no forced chapter mapping) |

Placeholder detection: chapter list empty, or exactly one chapter whose title is empty/whitespace,
whose start is within 0.05 s of zero, and whose end matches the source duration within
`max(0.1 s, duration × 0.001)`. This tolerance absorbs practical muxer timestamp rewrites.

Meaningful chapters are semantically validated after encoding: chapter count, start/end
timestamps with tolerance, and exact title text. Incomplete or malformed ffprobe chapter records
(reversed or missing timestamps) cause a detailed per-file skip rather than being mistaken for an
empty placeholder.

## Interaction with other policies

- Carrier exclusion happens **before** container selection, so a confirmed carrier never triggers
  MKV fallback; the file stays MP4.
- Metadata-repair remuxes probe their own input and apply the same chapter policy and the same
  actual-index mapping.
- Multiple confirmed carriers for one file are ambiguous and produce a specific safe skip.
- Ordinary sources keep the no-forced-chapter policy — chapters are never mapped on guesswork.

See [functions/chapter-carrier-parsing.md](functions/chapter-carrier-parsing.md) for the parser
declarations.
