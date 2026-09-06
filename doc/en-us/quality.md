# Quality

How MyVidComp decides what a conversion should look like, and how it checks
whether it got there.

## What is measured

Quality is measured with VMAF, a model Netflix trained on how people actually
rate video. It produces one number from 0 to 100 comparing a converted file
against its source. Roughly six points is one just-noticeable difference, and a
score around 93 to 95 is where most viewers stop being able to tell the two
apart. That is why the default target is 95.

Scores are kept in hundredths everywhere inside the program, so they cross the
embedding boundary as whole numbers.

## Measuring

Measurement runs FFmpeg's `libvmaf` filter with the converted file as the first
input and the source as the second. That order matters: reversed, the filter
still produces a number, just a different one.

Three things are done to every comparison:

- Both inputs are converted to the same planar YUV format, because the filter
  refuses inputs whose format, width or height differ. The format is chosen from
  the source's own chroma layout and bit depth.
- Both inputs get their timebase and start time reset. A source that does not
  begin at zero otherwise scores far too low, because the filter compares
  mismatched frames.
- Hardware decoding is not used. The filter needs frames in ordinary memory.

The model is chosen by resolution: the 4K model above 1440p, the standard model
below it.

`n_threads` defaults to one thread in FFmpeg itself, which is far too slow for
whole files, so MyVidComp sets it to the number of cores unless told otherwise.

## Where it is inaccurate

The published VMAF models were trained on standard-range video. HDR sources are
still scored, but the result carries a note saying it is indicative only, and it
should not be used to reject an HDR conversion on its own.

## Choosing a setting

Encoders disagree about what a given quality number looks like, and the same
number behaves differently on different footage. There are two ways to pick one.

### Measure and tune

Short samples are copied out of the source without re-encoding, spread through
the middle of the file, away from titles and credits. One sample per twelve
minutes, twenty seconds each, at most six.

Each trial encodes every sample at one setting and measures the result on every
frame, then interpolates towards the setting that lands just above the target.
The search stops when the bracket closes to one step, when a setting repeats, or
after six trials. The chosen setting is the largest passing value, which is the
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
long as encoding it; 4K is several times worse. The spot check compares one
frame in five, which is several times faster and accurate enough to decide with.
A tuning search adds a handful of short encodes on top of that.
