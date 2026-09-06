# Review

Conversions kept beside their originals for the user to decide about. Described for readers in
[../review.md](../review.md).

| Declaration | Kind | Purpose |
|---|---|---|
| `new` | function | Builds a deviation record, returning the value. |
| `size_percent` | function | Reports the size change as a percentage of the original, returning the percentage or none when the original is empty. |
| `parse_review_decision` | function | Parses a review decision wire value, returning the decision or a user-facing error. |
| `review_output_path` | function | Builds the pending-review output path for a converted file, returning the path carrying the review suffix. |
| `review_sidecar_path` | function | Builds the sidecar path for a pending review file, returning the sidecar path. |
| `is_review_artifact` | function | Checks whether a file name marks a pending review output or its sidecar, returning true for either. |
| `has_pending_review` | function | Checks whether a source file already has a conversion waiting for a decision, returning true when a review file sits beside it. |
| `write_sidecar` | function | Writes the sidecar describing one pending review, returning success or a disk error. |
| `sidecar_json` | function | Serializes one review item as the sidecar JSON document, returning the text. |
| `review_list_json` | function | Serializes a list of review items for the FFI, returning a JSON array document. |
| `list_reviews` | function | Finds every pending review under a folder tree, returning the items sorted by original path. |
| `collect_reviews` | function | Walks one directory level collecting review sidecars, returning nothing. |
| `read_sidecar` | function | Reads one sidecar file into a review item, returning the item or none when the record is unreadable or its files are gone. |
| `resolve_review` | function | Applies a decision to one pending review, returning success or a user-facing error. |
| `rename_into_place` | function | Renames a review file to its destination without overwriting anything, returning success or a user-facing error. |
| `kept_both_path` | function | Builds the name a converted file takes when the user keeps both copies, returning a path that does not collide with the original. |
| `review_summary_line` | function | Describes a review item for terminal output, returning one summary line. |
| `field` | function | Reads one string field out of a sidecar document, returning the unescaped value or none. |
| `number` | function | Reads one numeric field out of a sidecar document, returning the value or none for a missing or null field. |
| `deviations` | function | Reads the deviation list out of a sidecar document, returning every recorded change. |
