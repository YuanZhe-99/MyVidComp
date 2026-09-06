# Output Validation

Every converted temp output is validated against the source (or, for hardware-mode adaptations,
against the explicit adaptation target) before it may be committed or reused. Validation runs
`ffprobe` on the output and compares each property below.

## Primary video checks

| Check | Rule |
|---|---|
| Codec | output must be `av1` |
| Duration | must be valid (greater than zero) |
| Resolution | width/height must match the source exactly |
| Frame rate | a source/output rate pair must match within tolerance (see below) |
| Pixel format | must match the source (normalized), or the explicit adaptation target |
| Color metadata | range/space/transfer/primaries must match when known in the source |
| Display metadata | SAR/DAR compared numerically with tolerance; field order must match |
| Chroma location | advisory for AV1/MP4 when stronger metadata validates; explicit changes still fail |

Frame-rate validation accepts a pair match: source vs output nominal rates, or source vs output
average rates, compared with a small tolerance. This absorbs the drift between average and
nominal rates that muxing can introduce while still rejecting real frame-rate changes.

SAR/DAR are compared numerically with a small tolerance because FFmpeg may rewrite equivalent
MP4 rationals (e.g. near-1:1 SAR values).

## Stream signature checks

The output stream layout must match the expected mapped-stream signature:

- same stream count
- same type per position
- the first video stream must be `av1`
- every other stream must keep its exact source codec

A confirmed chapter carrier is excluded from the expected signature (see
[chapter-carrier.md](chapter-carrier.md)).

## Chapter semantics

Chapter validation applies only for confirmed carrier sources:

- `ConfirmedMeaningful`: output chapter count, start/end timestamps (0.05 s tolerance), and exact
  titles must match the source chapters.
- `ConfirmedEmpty`: the output must contain no chapters.

Ordinary sources have no forced chapter mapping and therefore no chapter validation.

## Repair and reuse classification

Validation failures are classified:

- **Remux-repairable** (missing AV1 color/chroma metadata): eligible for a metadata-repair remux
  that re-probes and re-maps the temp output's actual stream indexes.
- **Unusable temp**: no readable video stream or wrong codec — the temp output is deleted (only
  files matching MyVidComp temp naming) and never reused.
- **Mismatch**: retryable only inside the encoder-attempt loop for strict output mismatches.

Cached temp outputs may be reused only after the same validation path succeeds.

See [functions/validation-commit.md](functions/validation-commit.md) for the declarations.
