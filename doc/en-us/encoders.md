# Encoder Selection and Attempts

MyVidComp detects usable AV1 encoders at runtime and never trusts FFmpeg's advertised encoder list
alone: every candidate must also pass a synthetic runtime encode check.

## Candidate pool

| Encoder | Kind | Notes |
|---|---|---|
| `av1_nvenc` | NVIDIA GPU | |
| `av1_qsv` | Intel GPU | |
| `av1_amf` | AMD GPU | |
| `av1_mf` | Media Foundation GPU | preferred before `av1_vulkan` on Windows ARM64 (Snapdragon) |
| `av1_vulkan` | Vulkan GPU | |
| `libsvtav1` | CPU (SVT-AV1) | |
| `libaom-av1` | CPU | |
| `librav1e` | CPU | |

Short aliases are accepted for `--encoder`: `nvenc`, `qsv`, `amf`, `mf`, `vulkan`. `auto`
(default) ranks automatically per the conversion mode; an explicit override pins one encoder and
never switches to another.

## Detection requirements

An encoder is usable only when both hold:

1. FFmpeg advertises it in the encoder listing.
2. The synthetic runtime encode check succeeds (a tiny null-muxed encode probe).

Advertised-but-failing encoders are reported with their failure detail (e.g. missing
`nvcuda.dll`, no MFT for the media type, Vulkan conversion failure) and skipped. If no candidate
survives, the run fails with a terminal error rather than guessing.

## Retry classification

Per-file attempt failures are classified conservatively. Retryable:

- device/encoder/pixel-format failures
- strict output mismatches (validation failures against the plan)

Terminal — never retried:

- unknown errors, input problems, disk problems, process failures
- repair-infrastructure failures, commit errors

Cache lookup and commit stay outside the attempt loop entirely.

## Child-process creation

All Rust `ffmpeg`/`ffprobe` child processes go through the shared command constructor so Windows
applies `CREATE_NO_WINDOW` — no console window flashes during GUI runs. Direct Dart runtime
`-version` process checks that flash a console window must not be reintroduced.

See [functions/encoders-quality.md](functions/encoders-quality.md) for the detection/ranking
declarations and [functions/transcoding.md](functions/transcoding.md) for the attempt loop.
