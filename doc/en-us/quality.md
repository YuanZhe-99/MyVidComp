# Quality

How MyVidComp decides what a conversion should look like, and how it checks
whether it got there.

## What is measured

Quality is measured with VMAF by default — a model Netflix trained on how people
actually rate video. It produces one number from 0 to 100 comparing a converted
file against its source. Roughly six points is one just-noticeable difference,
and a score around 93 to 95 is where most viewers stop being able to tell the
two apart. That is why the default target is 95.

MyVidComp also supports FFmpeg's built-in `xpsnr` metric (`--quality-metric
xpsnr` or `auto`), which measures perceptually weighted PSNR across luma and
chroma much faster on CPU and maps the decibel score onto the same 0–100 scale
for target and review comparison. Under `auto`, a fast `xpsnr` pass screens
obvious passes or misses and confirms borderline results with `libvmaf`.

Scores are kept in hundredths everywhere inside the program, so they cross the
embedding boundary as whole numbers.

## Measuring

Measurement runs FFmpeg's `libvmaf` (or `xpsnr`) filter with the converted file
as the first input and the source as the second. That order matters: reversed,
the filter still produces a number, just a different one.

Three things are done to every comparison:

- Both inputs are converted to the same planar YUV format, because the filter
  refuses inputs whose format, width or height differ. The format is chosen from
  the source's own chroma layout and bit depth.
- Both inputs get their timebase and start time reset. A source that does not
  begin at zero otherwise scores far too low, because the filter compares
  mismatched frames.
- Decoding may happen on a graphics card, but the comparison itself never does.
  The two files are decoded separately, because they are in different codecs and
  hardware that reads one may refuse the other, and the decoded frames are
  brought back into ordinary memory for the filter.
- When scoring 10-bit portrait sources (`height > width`) with `xpsnr`, both
  inputs receive `transpose=1` before `xpsnr` to avoid an upstream FFmpeg
  `vf_xpsnr` 10-bit portrait stride bug.

For whole-file checks (`sampled` / `full`), the check runs once across the stream (still collecting per-frame `libvmaf` JSON logs when CPU `libvmaf` runs).
For the default `segmented` check on files 90 seconds or longer, a zero-decode `ffprobe` packet scan (`packet=pts_time,size,flags`) builds an isolated Dual-Pool plan of 6-second keyframe-snapped windows: up to 6 complexity-stratified windows in Pool A (sampling low, medium, and high source-bitrate tertiles) estimate unbiased Mean and Harmonic Mean, while up to 4 isolated Pool B risk probes target bitrate-starvation lows (`B_enc / B_src`) and source-bitrate spikes to catch P05 and Min collapses without skewing Pool A. Files between 60 and 90 seconds (or when packet headers are unavailable) fall back to up to 6 uniformly spaced, keyframe-aligned 6-second windows. Both paths use odd subsampling (`n_subsample=3`) to avoid hierarchical B-frame phase-locking, fast `-ss` on the distorted input, and two-stage hybrid seek (`-ss` 5 s earlier plus `trim`) on the reference input.

Per-frame `libvmaf` JSON output is parsed to compute arithmetic Mean, Harmonic Mean, 5th percentile (P05), Minimum, and the exact `worst_frame_seconds` timestamp from the lowest-scoring frame's index and source frame rate. Under flexible preservation, even when the global Mean passes the review threshold, a sharp local drop (`Harmonic Mean < threshold - 3` or `P05 < threshold - 6`) flags a `local_quality` deviation and holds the conversion for review.

The model is chosen dynamically from resolution, HDR transfer, and build capabilities: when FFmpeg's `libvmaf` supports `hdr_vmaf_v1.1.8` (`libvmaf >= 3.2` with `-Denable_zimg=true`), PQ (`smpte2084`) sources automatically use `hdr_vmaf_v1.1.8_1d5h_2160` (> 1440p) or `hdr_vmaf_v1.1.8_3d0h` (<= 1440p). Otherwise, SDR and unsupported builds use `vmaf_4k_v0.6.1` above 1440p and `vmaf_v0.6.1` below it.

`n_threads` defaults to one thread in FFmpeg itself, which is far too slow for
whole files, so MyVidComp sets it to the number of cores unless told otherwise.

## Where it is inaccurate

Standard `vmaf_v0.6.1` models were trained on standard-range video. When an FFmpeg build supports `hdr_vmaf_v1.1.8`, PQ (`smpte2084`) HDR sources are scored natively without an HDR inaccuracy caveat; HLG (`arib-std-b67`) sources and builds without HDR VMAF v1 still carry a note saying the score is indicative only and should not be used to reject an HDR conversion on its own.

## Choosing a setting

Encoders disagree about what a given quality number looks like, and the same
number behaves differently on different footage. There are two ways to pick one.

### Measure and tune

Short samples are copied out of the source without re-encoding, spread through
the middle of the file, away from titles and credits. One sample per twelve
minutes, twelve seconds each, at most six.

Each trial encodes every sample at one setting and measures the result with
`n_subsample=5` (odd subsampling avoids B-frame phase-locking while cutting trial
scoring cost), then interpolates in `logit(VMAF/100)` secant space (seeded by an
initial estimate when available) to linearize the CRF/QP S-curve towards the
setting just above the target. The search stops when the bracket closes to one step, when a setting repeats, or after
six trials. The chosen setting is the largest passing value, which is the
smallest file that still reaches the target.

If nothing reaches the target, the best trial is used and the run says so.

Trials use each encoder's fastest preset. A preset changes file size far more
than it changes quality, so the trial score still predicts the final result
closely enough to steer the search.

Graphics-card encoders have no usable constant-quality control, so they are
searched over a ladder of bitrates instead: 15, 25, 40, 60 and 85 percent of the
source bitrate, stopping at the first rung that reaches the target.

### Quick estimate

A setting is derived from the source's own encoder metadata when it has any, and
from bitrate per pixel per frame when it does not, then converted onto the
target encoder's scale. It costs nothing and is less precise. Files under 90
seconds always use it.

### Fixed quality

Passes the user's explicit `--fixed-quality` (`fixed_quality`) CRF/QP value on
to the encoder without search or estimation, or falls back to the encoder's
built-in default when none is given.

## After the conversion

Every finished conversion is compared against its source unless the quality
check is switched off, and a conversion that changed something recoverable is
always compared regardless.

A result at or above the target minus the review margin is committed normally. A
result below it is kept beside its original for the user to decide, along with
the score. See [review.md](review.md).

Cached working files from an interrupted run are measured too, so reusing one
never skips the check.

## Cost

Measurement is not free. Comparing every frame of a 1080p file takes about as
long as encoding it; 4K is several times worse. `segmented` (the default) seeks
to up to six 6-second keyframe-aligned windows across files 60 s or longer with
`n_subsample=3`, keeping CPU measurement bounded to seconds even on long 4K
inputs. `sampled` compares one frame in five across the full stream, and
`--quality-metric xpsnr` (or `auto`) replaces or screens slow `libvmaf` passes. A tuning search adds a
handful of short 12-second sample encodes on top of that.

## Reading the video

Decoding happens twice for every file: once to convert it, and once more to
compare the result against it. The `--decoder` setting decides where that
happens.

`auto`, the default, uses a graphics card when one is proven to work. Proving it
takes two steps, because neither answers the other's question: creating the
device says whether the driver is there, and decoding two frames of the real
file says whether this codec is supported at all. Hardware with no AV1 decoder
handles H.265 perfectly and refuses AV1, and the answer is remembered per codec,
so a folder of one codec is asked once.

Any failure during real work is treated the same way: one warning, the work is
repeated on the processor, and nothing else in the run uses the card again. A
crash therefore costs one encode rather than the file. This is why `auto` is
safe to have on; an earlier version asked FFmpeg to choose a method by itself,
which crashed it on roughly one run in six.

`cpu` never uses a graphics card. It is the setting to choose when a score has
to come out the same on another machine. `gpu` asks for one and says so when
none can be used. A method name such as `d3d11va`, `cuda`, `vaapi` or
`videotoolbox` picks exactly one and nothing else.

What this speeds up is reading the video, not the measurement: VMAF itself runs
on the processor in every case. On a 4K file that still helps, because it leaves
the processor free for the comparison.

### Measuring on the graphics card

FFmpeg has a second comparison filter, `libvmaf_cuda`, which keeps the frames on
an NVIDIA card from decoding all the way through scoring. MyVidComp uses it when
it is there, the source is eight-bit 4:2:0, and `--decoder cuda` was asked for by
name. It is never chosen automatically, because no build MyVidComp can test with
has it.

No build anyone ships has it, and that is a licensing wall rather than an
oversight. The filter needs libvmaf compiled with CUDA and FFmpeg configured
like this:

```sh
./configure --enable-nonfree --enable-ffnvcodec --enable-libvmaf
```

`--enable-nonfree` produces a binary that may not be redistributed, so neither
BtbN, whose builds MyVidComp bundles, nor MyVidComp itself can hand one out.
Building it yourself works on Linux; on Windows it is an open question, with no
published recipe.

For an NVIDIA machine the useful setting is therefore `--decoder cuda`, which
works with the bundled FFmpeg today and moves the decoding, though not the
scoring, off the processor. Point `--ffmpeg` at your own build if you have one
with the CUDA filter; MyVidComp will notice and say so.
