# Options

Every setting a person can change, what it does, and where it can be set.

Each option has one stable wire value used by the command line, the config file
and the embedding interface, plus a label and a sentence shown in the
application. The wire values never appear on screen.

## Output codec

| Wire value | Shown as | Meaning |
|---|---|---|
| `av1` (default) | AV1 | Smallest files. Slow to encode in software. |
| `hevc` | H.265 | Slightly larger, plays almost everywhere, and records colour metadata AV1 cannot. |
| `vvc` | H.266 | Experimental. Software only, very slow, few players. |

Set with `--codec`, the `target_codec` config key, or the codec picker.

H.266 accepts only 10-bit 4:2:0 video, so every other source is converted before
encoding. That conversion is a change, which means H.266 needs flexible
preservation for most files. See [codecs.md](codecs.md).

## Quality target

A VMAF score from 50 to 100, stored internally in hundredths. The default is
95.0. Set with `--quality`, the `quality_target` config key, or the quality
picker, which offers Highest (97), High (95), Balanced (93) and Smallest (90).

## Quality strategy

| Wire value | Shown as | Meaning |
|---|---|---|
| `search` (default) | Measure and tune | Encodes short samples at several settings and picks the smallest that reaches the target. |
| `estimate` | Quick estimate | Derives a setting from the source without test encodes. |

Set with `--quality-mode` or the `quality_mode` config key. A search needs the
media tools to support quality measurement; without that it falls back to an
estimate and says so. Files shorter than 90 seconds are always estimated,
because the trials would cost more than the conversion.

## Quality check

| Wire value | Shown as | Meaning |
|---|---|---|
| `sampled` (default) | Spot check | Compares one frame in five. |
| `full` | Every frame | Compares every frame. Slowest and most accurate. |
| `off` | Skip | No comparison. |

Set with `--quality-check` or the `quality_check` config key. A conversion that
changed something recoverable is measured even under `off`, because its review
decision depends on a score.

## Preservation

| Wire value | Shown as | Meaning |
|---|---|---|
| `flexible` (default) | Balanced | Converts when only recoverable details would change, records each change, and keeps both files. |
| `strict` | Exact | Converts only when every detail is reproduced. Leaves the rest untouched. |

Set with `--preservation` or the `preservation` config key.

Flexible preservation relaxes exactly these, and nothing else:

- colour metadata the chosen codec cannot record,
- a pixel-format conversion a codec or graphics card requires,
- an MKV container when a stream will not fit in MP4.

It never changes resolution, frame rate, HDR handling, or anything about the
audio. Interlaced sources are still left alone, because deinterlacing is a
change to the picture rather than to how it is described. See
[review.md](review.md).

## Encoders

| Wire value | Shown as | Meaning |
|---|---|---|
| `auto` (default) | Balanced | Graphics card when it reproduces the source exactly, processor otherwise. |
| `gpu` | Prefer graphics card | Graphics card first, even when it means converting the pixel format. |
| `cpu` | Processor only | No graphics card. Slowest, usually smallest. |

Set with `--encoder-preference` or the `encoder_preference` config key. The
retired name `--conversion-mode` is still accepted, and its old values map
across: `consistency` becomes `auto`, `hardware` becomes `gpu`.

`--encoder NAME` pins one specific encoder. It is never swapped for another,
although it may retry with a converted pixel format.

## Decoding

| Wire value | Shown as | Meaning |
|---|---|---|
| `auto` (default) | Graphics card when it works | Graphics card once it is proven on the file, processor otherwise. |
| `gpu` | Graphics card | Asks for one and says so when none can be used. |
| `cpu` | Processor | Never uses a graphics card. Scores match on any machine. |

Set with `--decoder` or the `decoder_preference` config key. A method name such
as `d3d11va`, `dxva2`, `cuda`, `qsv`, `vaapi`, `videotoolbox` or `mediacodec`
picks exactly one and nothing else.

This is about reading the video, not encoding it; the two are separate settings
because a machine can be good at one and not the other. See
[quality.md](quality.md) for how a method is proven and what happens when one
fails.

## Container

| Wire value | Shown as | Meaning |
|---|---|---|
| `mp4` (default) | MP4 only | Files whose streams do not fit MP4 are left alone. |
| `mkv-fallback` | MP4, or MKV when needed | Uses MKV when a stream cannot go in MP4. |

Set with `--output-format` or the `output_format` config key.

## Originals

`--keep-original` renames each original to `<name>.old` once the result is
verified. `--no-keep-original` replaces it. Neither applies to a conversion kept
for review: both files always survive until the user chooses.

## Everything else

| Option | Default | Purpose |
|---|---|---|
| `--count N` | all | Convert at most N files. |
| `--dry-run` | off | List what would happen, change nothing. |
| `--tmp-dir PATH` | beside each video | Where working files go. |
| `--review-margin N` | 2.0 | How far under the target a result may land before both files are kept. |
| `--quality-threads N` | one per core | Threads used for quality measurement. |
| `--ffmpeg PATH`, `--ffprobe PATH` | bundled | Media tools to use. |
| `--config PATH` | `config.yaml` | Settings file to read. |

## Precedence

Command line, then the config file, then the built-in default. A blank or
absent config value means "use the default" rather than being an error, so a
shipped template can list a key without committing to a value.
