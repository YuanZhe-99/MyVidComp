# Output Validation

Every converted temp output is validated against the source (or, for hardware-mode adaptations,
against the explicit adaptation target) before it may be committed or reused. Validation runs
`ffprobe` on the output and compares each property below.

## Primary video checks

| Check | Rule |
|---|---|
| Codec | output must be the codec the run asked for (`av1`, `hevc` or `vvc`) |
| Duration | must be valid (greater than zero) and match the source: within the larger of 0.5 s and 1% for a fresh encode, or the larger of 0.5 s and 0.1% for a reused temp |
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
- the first video stream must be the codec the run asked for
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
- **Unusable temp**: no readable video stream, a duration of zero, or a codec other than the one
  the run asked for — the temp output is deleted (only files matching MyVidComp temp naming) and
  never reused. A duration mismatch is not in this class: the temp is skipped, not deleted.
- **Mismatch**: retryable only inside the encoder-attempt loop for strict output mismatches.

Cached temp outputs may be reused only after the same validation path succeeds, checked against
the codec this run asked for and with the stricter reused-temp duration tolerance, because a run
that was stopped part-way leaves a temp that is only a little short. A reused temp that matched
the hardware-mode adaptation reports the same `pixel_format` difference a fresh adapted encode
would, so it is measured and can go to review instead of being committed silently.

See [functions/validation-commit.md](functions/validation-commit.md) for the declarations.
