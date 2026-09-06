# Stream policy

Deciding which streams are mapped and which container can hold them. Described for readers in
[../output-format.md](../output-format.md).

| Declaration | Kind | Purpose |
|---|---|---|
| `is_chapter_carrier_candidate` | function | Checks whether a stream has all ffprobe-side traits required of a possible QuickTime chapter carrier, returning true only for data/bin_data FourCC text streams with a numeric track ID. |
| `chapters_are_placeholder_or_empty` | function | Classifies absent chapters or one unnamed full-duration chapter as an empty placeholder, returning true within practical muxer timestamp tolerance. |
| `parse_bmff_chapter_targets` | function | Reads ISO BMFF structure to collect track IDs explicitly targeted by tref/chap references.. |
| `matroska_incompatible_stream_reason` | function | Finds a mapped non-primary stream type not conservatively copy-compatible with Matroska, returning actual index/type/codec detail or none. |
| `mp4_incompatible_stream_reason` | function | Finds the first non-primary stream that cannot be copied into an MP4 container, returning a detail message or none. |
| `is_mp4_copy_compatible_stream` | function | Checks whether a non-primary stream can be copied into the MP4 target, returning true when the stream type/codec is supported. |
| `is_mp4_video_codec` | function | Checks whether a copied non-primary video codec is MP4-compatible, returning a boolean. |
| `is_mp4_audio_codec` | function | Checks is mp4 audio codec predicate, returning a boolean. |
| `is_mp4_subtitle_codec` | function | Checks whether a copied subtitle codec is MP4-compatible, returning a boolean. |
| `interlaced_source_reason` | function | Returns a skip reason when the source is explicitly interlaced, returning none for progressive or unknown field order. |
| `estimate_av1_crf` | function | Builds or derives estimate av1 crf data, returning the computed value. |
