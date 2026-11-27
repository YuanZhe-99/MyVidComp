# Review

What happens when a conversion worked but MyVidComp is not willing to decide for
you.

## When a conversion goes to review

Under flexible preservation a conversion is kept for review when any of these
hold:

- it changed something recoverable, such as a colour note the chosen codec
  cannot record, or a pixel format a codec requires,
- it scored below the quality target minus the review margin,
- its quality could not be measured at all and it changed something,
- `--size-guard review` is set and the output is not smaller than the source.

Under strict preservation none of these arise: a file that would need a change
is left untouched instead, and a low score is the only route to review.

## What is written

Nothing is deleted, and the original is never touched. The converted file is
written beside it:

```text
holiday.mkv                          the original, untouched
holiday.myvidcomp-review.mp4         the conversion
holiday.myvidcomp-review.mp4.json    what changed and how it scored
```

The record beside it holds the two paths, the name the conversion takes if it is
kept, both file sizes, the encoder and quality setting used, the measured score
and the target, optional `duration_seconds` / `worst_frame_seconds` hints for
preview seeking, and one plain sentence per change.

## Scanning again

A later run skips both halves of a pending pair, and reports the reason. Without
that, the original would be converted a second time and the review file would be
treated as a new source.

## Deciding

Three choices, each of which removes the record afterwards:

| Choice | Result |
|---|---|
| Keep the new one | The original is deleted and the conversion takes its name. |
| Keep the original | The conversion is deleted. Nothing else changes. |
| Keep both | The conversion is renamed to `<name>.myvidcomp.<ext>` and both files remain. |

In the application these are the three buttons on each card in Review, which also
shows the score, both sizes, every recorded change, and a **Compare** button.
Opening Compare launches an interactive dual-mode inspector backed by the
bundled FFmpeg tools (`src/preview.rs`): synchronized high-resolution still-frame
inspection (Side-by-Side, Wipe slider, A/B toggle, Difference heatmap, zoom, and
worst-frame jump), in-app synchronized clip playback from short 720p JPEG bursts
(at 1x or 0.5x speed across Side-by-Side, Wipe, and A/B modes), or exporting a
6-second side-by-side MP4 / opening both files in the system player. The whole
review list can also be resolved at once.

From the command line:

```sh
myvidcomp /path/to/videos --list-reviews
myvidcomp /path/to/videos --resolve-reviews keep-new
```

Through the embedding interface, `myvidcomp_review_list` returns the pending
records and `myvidcomp_review_resolve` applies one decision.

## Safety

A decision never overwrites anything. Keeping the new file when a different
file already has that name is refused before anything is deleted, so the
original, the conversion and the other file all stay as they were. A name that
is the original itself, such as `clip.MP4` kept as `clip.mp4` on a disk that
ignores letter case, is not a collision. The record is only removed once the
files are where the decision says they should be. A record whose files have
moved or been deleted is ignored.

Listing pending reviews walks real folders only and never follows a folder
link, so a link that points back up the tree cannot make it loop forever. Each
folder is read once per scan to find the originals that already have a pending
conversion.
