# MyVidComp Documentation (English)

This is the English documentation tree for **MyVidComp** (`myvidcomp`), a batch video converter
that makes a folder of videos smaller, checks how close each result looks to its source, and never
deletes anything it is not sure about. The Simplified Chinese mirror (`doc/zh-cn/`) is maintained
alongside it; see [translation-guide.md](translation-guide.md) for the translation and parity
rules.

**These docs are the authoritative description of the code.** The repository's
[AGENTS.md](../../AGENTS.md) is deliberately limited to instructions for agents: workflow,
authoring rules, the behaviour contract, and the release process. It points here for everything
else. When code changes, these pages are updated first; when docs and code disagree, verify against
the code and then fix the page.

## Contents

- [architecture.md](architecture.md) — what MyVidComp is, the conversion pipeline, repository
  layout, media-tool lookup, and the safety model.
- [options.md](options.md) — every setting, its wire value, and where it can be set.
- [quality.md](quality.md) — how a quality setting is chosen and how the result is measured.
- [review.md](review.md) — conversions kept for the user to decide about, and how to resolve them.
- [codecs.md](codecs.md) — what differs between AV1, H.265 and H.266.
- [output-format.md](output-format.md) — the container policy, the conservative MP4 copy
  whitelist, MKV fallback rules, and skip reasons.
- [chapter-carrier.md](chapter-carrier.md) — evidence-based QuickTime chapter-carrier handling:
  ISO BMFF `tref/chap` detection, placeholder suppression, and meaningful chapter re-authoring.
- [encoders.md](encoders.md) — encoder detection and ranking, runtime encode checks, retry
  classification, and Windows hidden child-process creation.
- [validation.md](validation.md) — post-encode output validation rules: stream signature, video
  properties, metadata, duration, and chapter semantics.
- [safety.md](safety.md) — temp files, cached-temp reuse, commit/recovery, and graceful stop.
- [ffi-abi.md](ffi-abi.md) — the versioned embedding ABI, its entry points, and the event JSON.
- [packaging.md](packaging.md) — Windows dual-architecture packaging, media-tool bundling, and
  package verification.
- [gui.md](gui.md) — the application's structure, adaptive layout, settings and wording rules.
- [android.md](android.md) — the phone build, what it needs, and what it still lacks.
- [ci.md](ci.md) — what runs automatically, and how to run the same checks locally.
- [version-history.md](version-history.md) — dated record of behaviour changes and why they exist.
- [translation-guide.md](translation-guide.md) — the English-to-Chinese translation guide and
  terminology glossary for this repository.
- [functions/INDEX.md](functions/INDEX.md) — the function index: every declaration in `src/`,
  grouped by behaviour domain, with links to full per-domain documentation.

## Current status

The Rust core is `src/lib.rs` plus focused modules for the parts that stand on their own:
`options.rs`, `codec.rs`, `vmaf.rs`, `search.rs`, `review.rs` and `ffi.rs`. The application lives
under `gui/` and builds for Windows, macOS, Linux and Android from one source tree.

This tree documents version 0.1.0: three output codecs, quality measured with VMAF, flexible
preservation with review, and the single-version embedding ABI.

Per-file Dart declaration pages for `gui/lib/` are not maintained; [gui.md](gui.md) covers the
application's structure, and such pages can be added later without restructuring this tree.
