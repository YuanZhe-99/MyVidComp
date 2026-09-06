# Function index

Total documented declarations in `src/`: **368**.

Every declaration carries an `AI-FUNC-SUMMARY` comment in the source, and every one of those
appears on exactly one page below. The pages that mirror a module are the module; the pages that
split `src/lib.rs` group it by the stage of the pipeline the code belongs to, because the pipeline
itself is one file.

| Page | Covers | Declarations |
|---|---|---|
| [options.md](options.md) | `src/options.rs` | 29 |
| [codecs.md](codecs.md) | `src/codec.rs` | 12 |
| [quality.md](quality.md) | `src/vmaf.rs`, `src/search.rs` | 24 |
| [review.md](review.md) | `src/review.rs` | 20 |
| [ffi-abi.md](ffi-abi.md) | `src/ffi.rs` | 19 |
| [workflow-events.md](workflow-events.md) | the run, its events, progress, and commit | 79 |
| [cli-and-config.md](cli-and-config.md) | command line and settings file | 28 |
| [encoders-quality.md](encoders-quality.md) | encoder detection and quality estimation | 49 |
| [discovery-probing.md](discovery-probing.md) | finding and probing files | 20 |
| [transcoding.md](transcoding.md) | planning and running encodes | 55 |
| [validation-commit.md](validation-commit.md) | checking a finished file | 17 |
| [stream-policy.md](stream-policy.md) | stream mapping and container choice | 11 |
| [chapter-carrier-parsing.md](chapter-carrier-parsing.md) | ISO BMFF chapter evidence | 5 |

Adding, removing or changing a declaration means updating its page, the count beside it, the total
above, and the Simplified Chinese mirror, all in the same change. `scripts/check-doc-parity.ps1`
checks that the two language trees still line up.
