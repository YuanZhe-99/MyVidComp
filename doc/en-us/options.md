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
| `fixed` | Fixed quality | Uses the caller's `--fixed-quality` / `fixed_quality` CRF or QP value directly. |

Set with `--quality-mode` or the `quality_mode` config key. When set to `fixed`,
supply `--fixed-quality` (in integer or half-step points); if omitted, the run
falls back to the codec's default value. A search needs the media tools to
support quality measurement; without that it falls back to an estimate and says
so. Files shorter than 90 seconds are always estimated during `search`, because
the trials would cost more than the conversion.

## Quality check

| Wire value | Shown as | Meaning |
|---|---|---|
| `segmented` (default) | Segmented check | Compares up to six 6-second windows across files 60 s or longer (2x frame subsample). |
| `sampled` | Spot check | Compares one frame in five across the whole file. |
| `full` | Every frame | Compares every frame. Slowest and most accurate. |
| `off` | Skip | No comparison. |

Set with `--quality-check` or the `quality_check` config key. A conversion that
changed something recoverable is measured even under `off`, because its review
decision depends on a score.

## Quality metric

| Wire value | Shown as | Meaning |
|---|---|---|
| `vmaf` (default) | VMAF | Measures perceptual quality with `libvmaf`. |
| `xpsnr` | XPSNR | Measures weighted PSNR with FFmpeg's `xpsnr` filter (much faster on CPU) and maps dB onto the VMAF scale. |
| `auto` | Auto (XPSNR then VMAF) | Screens with `xpsnr` first and confirms borderline scores near the target with `libvmaf`. |

Set with `--quality-metric` or the `quality_metric` config key. Note that
`xpsnr` and `auto` require a `xpsnr`-capable FFmpeg build and automatically fall
back to VMAF if absent.

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

It never changes resolution or frame rate on its own. Interlaced sources are left
alone under the default `--deinterlace skip`, whereas `--deinterlace bwdif`
explicitly deinterlaces them with `bwdif`. Likewise, `--audio-policy copy`
keeps all audio verbatim, while `--audio-policy transcode-incompatible`
re-encodes MP4-incompatible audio tracks to AAC when outputting to MP4. See
[review.md](review.md).

## Encoding speed

| Wire value | Shown as | Meaning |
|---|---|---|
| `fast` | Fast | Faster encode preset at slightly larger file size. |
| `balanced` (default) | Balanced | Default encoder speed/efficiency trade-off. |
| `slow` | Slow | Slower encode preset for better compression. |

Set with `--preset` or the `encoder_preset` config key. Each encoder maps this
abstract preset onto its own native preset flag (for example SVT-AV1 `-preset`,
`libx265`/`libx264` presets, or NVENC `-preset`).

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
| `mp4` (default) | MP4 only | Files whose streams do not fit MP4 are left alone (unless `--audio-policy transcode-incompatible` converts incompatible audio to AAC). |
| `mkv-fallback` | MP4, or MKV when needed | Uses MKV when a stream cannot go in MP4. |
| `mkv` | Always MKV | Outputs Matroska (`.mkv`) for every converted file whose streams fit MKV. |
| `keep` | Keep source container | Uses `.mkv` when the source was `.mkv` and otherwise uses `.mp4` (or falls back per compatibility rules). |

Set with `--output-format` or the `output_format` config key.

## Originals

`--keep-original` renames each original to `<name>.old` once the result is
verified. `--no-keep-original` replaces it. Neither applies to a conversion kept
for review: both files always survive until the user chooses. When `--output-dir`
points elsewhere, originals in the source tree remain untouched and outputs are
written under the mirror tree.

## Everything else

| Option | Default | Purpose |
|---|---|---|
| `--count N` | all | Convert at most N files. |
| `--dry-run` | off | List what would happen, change nothing. |
| `--tmp-dir PATH` | beside each video | Where working files go. |
| `--output-dir PATH` | beside each video | Optional root directory mirroring the source folder layout. |
| `--recursive` / `--no-recursive` | `true` | Recurse into subdirectories when discovering videos. |
| `--review-margin N` | 2.0 | How far under the target a result may land before both files are kept. |
| `--quality-threads N` | one per core | Threads used for quality measurement. |
| `--fixed-quality N` | codec default | Explicit CRF/QP used when `--quality-mode fixed` is active. |
| `--size-guard MODE` | `off` | Action when output is not smaller than source: `off`, `review`, or `skip`. |
| `--audio-policy MODE` | `copy` | Audio handling: `copy`, or `transcode-incompatible` (AAC in MP4). |
| `--deinterlace MODE` | `skip` | Interlaced source handling: `skip`, or `bwdif`. |
| `--ffmpeg PATH`, `--ffprobe PATH` | bundled | Media tools to use. |
| `--config PATH` | `config.yaml` | Settings file to read. |

## Precedence

Command line, then the config file, then the built-in default. A blank or
absent config value means "use the default" rather than being an error, so a
shipped template can list a key without committing to a value.
