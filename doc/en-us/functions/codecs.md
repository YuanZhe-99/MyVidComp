# Codecs

What differs between the three output codecs: which encoders produce each one, how their quality
controls are shaped, and how colour metadata is carried. Described for readers in
[../codecs.md](../codecs.md).

| Declaration | Kind | Purpose |
|---|---|---|
| `encoder_candidates` | function | Lists the encoders that can produce one codec, best first, returning the ordered candidate table. |
| `canonical_encoder_name` | function | Expands a short encoder alias into the full name for one codec, returning the canonical encoder name. |
| `vvc_alias` | function | Expands a short encoder alias for the VVC codec, returning the canonical encoder name. |
| `quality_style` | function | Reports which quality knob an encoder family exposes, returning the style. |
| `quality_range` | function | Reports the searchable quality range for an encoder family, returning the range and step. |
| `convert_quality` | function | Converts a quality value chosen for AV1 into the equivalent for another codec, returning the converted value. |
| `codec_bitrate_multiplier` | function | Reports how much of the source bitrate a codec needs for comparable quality, returning a multiplier. |
| `required_pixel_format` | function | Reports the only pixel format an encoder accepts, when it accepts just one, returning the format or none. |
| `metadata_filter` | function | Reports the metadata bitstream filter for one codec, returning the filter description or none when the codec has no usable filter. |
| `chroma_location_value` | function | Maps an ffprobe chroma location onto the value one codec can record, returning the value or none when the codec cannot express it. |
| `color_range_value` | function | Maps a colour range onto the value one codec records, returning the value or none. |
| `codec_accepts_source` | function | Reports whether a source can be encoded to one codec without changing its pixel format, returning true when no conversion is needed. |
