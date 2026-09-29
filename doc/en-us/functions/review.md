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
| `pending_review_stems` | function | Lists, in one directory read, the source stems that already have a conversion waiting for a decision, returning the set of stems (empty when the directory cannot be read). |
| `write_sidecar` | function | Writes the sidecar describing one pending review, returning success or a disk error. |
| `sidecar_json` | function | Serializes one review item as the sidecar JSON document, returning the text. |
| `review_list_json` | function | Serializes a list of review items for the FFI, returning a JSON array document. |
| `list_reviews` | function | Finds every pending review under a folder tree, returning the items sorted by original path. |
| `collect_reviews` | function | Walks one directory level collecting review sidecars, returning nothing; recurses into real subdirectories only, never through a symbolic link. |
| `read_sidecar` | function | Reads one sidecar file into a review item, returning the item or none when the record is unreadable or its files are gone. |
| `resolve_review` | function | Applies a decision to one pending review, returning success or a user-facing error; keep-new refuses, before deleting anything, when a different file already holds the final name. |
| `final_path_is_taken` | function | Checks, before the original is deleted, whether keeping the new file would collide with some other file at its final name, returning true for a file other than the original (a differently-cased name for the original itself is not a collision). |
| `rename_into_place` | function | Renames a review file to its destination without overwriting anything, returning success or a user-facing error. |
| `kept_both_path` | function | Builds the name a converted file takes when the user keeps both copies, returning a path that does not collide with the original. |
| `review_summary_line` | function | Describes a review item for terminal output, returning one summary line. |
| `field` | function | Reads one string field out of a sidecar document, returning the unescaped value or none. |
| `number` | function | Reads one numeric field out of a sidecar document, returning the value or none for a missing or null field. |
| `deviations` | function | Reads the deviation list out of a sidecar document, returning every recorded change, including text that holds brackets or quotes. |
| `closing_bracket` | function | Finds the `]` that closes a JSON array, skipping any inside a quoted string, returning its offset or none when the array never closes. |
