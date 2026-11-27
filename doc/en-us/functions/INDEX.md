# Function index

GUI declarations delegate shared theme, navigation, inline choices, settings rows
and pane layout; SectionCard, LabelledField, CountTile and PageBody inherit the
shared presentation. See [shared-ui.md](../shared-ui.md) for their responsibilities.

GUI theme facade: [shared UI](../shared-ui.md) documents `gui/lib/app_theme.dart`:
style-name parsing, branded shared-theme construction, light/dark methods and exports.
The Rust declaration totals below are unaffected by this extraction.

Total documented declarations in `src/`: **452**.

Every declaration carries an `AI-FUNC-SUMMARY` comment in the source, and every one of those
appears on exactly one page below. The pages that mirror a module are the module; the pages that
split `src/lib.rs` group it by the stage of the pipeline the code belongs to, because the pipeline
itself is one file.

| Page | Covers | Declarations |
|---|---|---|
| [options.md](options.md) | `src/options.rs` | 52 |
| [codecs.md](codecs.md) | `src/codec.rs` | 12 |
| [quality.md](quality.md) | `src/vmaf.rs`, `src/search.rs` | 37 |
| [review.md](review.md) | `src/review.rs` | 23 |
| [preview.md](preview.md) | `src/preview.rs` | 14 |
| [ffi-abi.md](ffi-abi.md) | `src/ffi.rs` | 32 |
| [workflow-events.md](workflow-events.md) | the run, its events, progress, and commit | 81 |
| [cli-and-config.md](cli-and-config.md) | command line and settings file | 28 |
| [encoders-quality.md](encoders-quality.md) | encoder detection and quality estimation | 51 |
| [discovery-probing.md](discovery-probing.md) | finding and probing files | 20 |
| [transcoding.md](transcoding.md) | planning and running encodes | 62 |
| [validation-commit.md](validation-commit.md) | checking a finished file | 22 |
| [stream-policy.md](stream-policy.md) | stream mapping and container choice | 13 |
| [chapter-carrier-parsing.md](chapter-carrier-parsing.md) | ISO BMFF chapter evidence | 5 |

Adding, removing or changing a declaration means updating its page, the count beside it, the total
above, and the Simplified Chinese mirror, all in the same change. `scripts/check-doc-parity.ps1`
checks that the two language trees still line up.
