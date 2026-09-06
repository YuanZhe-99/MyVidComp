# Chapter-carrier parsing

Reading bounded ISO BMFF metadata to tell a chapter carrier from an ordinary text track. Described
for readers in [../chapter-carrier.md](../chapter-carrier.md).

| Declaration | Kind | Purpose |
|---|---|---|
| `bmff_children` | function | Enumerates direct child boxes inside a bounded parent range, returning validated headers or a structural/read error. |
| `read_tkhd_track_id` | function | Reads a version 0 or 1 tkhd track ID within its validated box bounds, returning a positive track ID or a structural/read error. |
| `read_be_u32` | function | Reads one big-endian u32 from a bounded parser position, returning the value or a concise read error. |
| `read_be_u64` | function | Reads one big-endian u64 from a bounded parser position, returning the value or a concise read error. |
| `select_output_container` | function | Selects MP4 or conservative Matroska fallback after carrier filtering, returning a container or detailed unsupported-stream reason. |
