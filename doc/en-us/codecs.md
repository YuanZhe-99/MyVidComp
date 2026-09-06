# Codecs

What differs between the three output codecs, and why.

## Encoders

Candidates are tried in this order. Every one is confirmed to work by encoding a
single synthetic frame before a real file is given to it, so an encoder that is
advertised but broken is skipped rather than failing a conversion.

| Codec | Graphics card | Processor |
|---|---|---|
| AV1 | `av1_videotoolbox`, `av1_nvenc`, `av1_qsv`, `av1_amf`, `av1_mf`, `av1_mediacodec`, `av1_vulkan` | `libsvtav1`, `libaom-av1`, `librav1e` |
| H.265 | `hevc_videotoolbox`, `hevc_nvenc`, `hevc_qsv`, `hevc_amf`, `hevc_mf`, `hevc_mediacodec`, `hevc_vulkan` | `libx265` |
| H.266 | none exist | `libvvenc` |

Short names resolve against the chosen codec, so `--encoder nvenc` means
`av1_nvenc` for AV1 and `hevc_nvenc` for H.265.

## Quality controls

| Family | Control | Range |
|---|---|---|
| `libsvtav1`, `libaom-av1`, Vulkan | CRF | 10 to 55 |
| `libx265` | CRF, fractional | 10 to 46 |
| `librav1e` | quantizer | 20 to 255 |
| `libvvenc` | quantizer | 15 to 55 |
| graphics card, phone | bitrate | derived from the source |

Graphics-card encoders either have no constant-quality mode or implement one
that behaves differently per driver, so they are all given an explicit bitrate.

A quality value chosen for AV1 is converted onto the target encoder's own scale
before use. Those conversions are working approximations, not measured
equivalences: they set a starting point that a tuning search then corrects.

## Colour metadata

Source colour metadata is carried into the encoded stream with a bitstream
filter. Each codec names its options differently, and they do not cover the same
ground.

| Codec | Filter | Chroma position |
|---|---|---|
| AV1 | `av1_metadata` | two of the six positions |
| H.265 | `hevc_metadata` | all six |
| H.266 | none usable | none |

This is the practical difference between the codecs. The chroma position most
MPEG-derived sources report cannot be written into an AV1 stream at all, so
under strict preservation those files are left alone. The same files convert
exactly to H.265, because its filter can record every position the source
formats define.

H.266 has a bitstream filter, but it only handles access unit delimiters, so an
H.266 conversion cannot carry any colour metadata. Every such conversion records
that loss.

## Containers

MP4 is the default. H.265 in MP4 is tagged `hvc1`, because FFmpeg writes the
other tag by default and Apple players refuse to decode it. H.266 needs no tag.

MKV is used when a stream will not fit in MP4 and the container option allows it.

## Pixel formats

`libvvenc` accepts only 10-bit 4:2:0, so every other source is converted before
encoding. That is a recorded change. `libx265` accepts 8, 10 and 12-bit 4:2:0,
4:2:2 and 4:4:4, greyscale and alpha, which covers everything the tool is likely
to meet; AV1 software encoders accept only 4:2:0 at 8 or 10 bits.

## Speed

AV1 and H.265 are both usable for batch work. H.266 is not: even its fastest
preset is far slower than the others, and almost nothing can play the result. It
is offered because it produces the smallest files at a given quality, not
because it is a sensible default.
