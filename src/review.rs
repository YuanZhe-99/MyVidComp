//! Conversions that are waiting for the user to choose a file.
//!
//! A flexible conversion that changed something recoverable, or that scored
//! below the quality target, is never committed over the original. Instead the
//! converted video is written beside it as `<name>.myvidcomp-review.<ext>` with
//! a small sidecar file recording what changed and how it scored. Nothing is
//! deleted until the user decides.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use super::{REVIEW_SUFFIX, format_quality, is_same_existing_file, json_string, with_added_suffix};

/// Extension of the sidecar written next to every pending review file.
const SIDECAR_EXTENSION: &str = "json";

/// One recoverable difference between the source and the converted file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Deviation {
    /// Stable identifier, for example `pixel_format` or `dropped_stream`.
    pub kind: String,
    /// One sentence a non-technical reader can act on.
    pub detail: String,
}

impl Deviation {
    // AI-FUNC-SUMMARY: Builds a deviation record; returns the value; side effects: none.
    pub fn new(kind: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            detail: detail.into(),
        }
    }
}

/// A converted file waiting for a keep-or-discard decision.
#[derive(Debug, Clone, PartialEq)]
pub struct ReviewItem {
    /// The untouched source file.
    pub original_path: PathBuf,
    /// The converted file, still carrying the review suffix.
    pub review_path: PathBuf,
    /// The sidecar describing this pair.
    pub sidecar_path: PathBuf,
    /// The name the converted file takes if the user keeps it.
    pub final_path: PathBuf,
    pub original_bytes: u64,
    pub review_bytes: u64,
    pub encoder: String,
    pub codec: String,
    pub quality: String,
    /// Measured quality in hundredths of a VMAF point, when it could be measured.
    pub score: Option<u32>,
    /// The score the run was aiming for, in hundredths.
    pub target: u32,
    pub deviations: Vec<Deviation>,
}

impl ReviewItem {
    // AI-FUNC-SUMMARY: Reports the size change as a percentage of the original; returns the percentage or none when the original is empty; side effects: none.
    pub fn size_percent(&self) -> Option<f64> {
        if self.original_bytes == 0 {
            None
        } else {
            Some(self.review_bytes as f64 / self.original_bytes as f64 * 100.0)
        }
    }
}

/// What the user decided about one pending review.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewDecision {
    /// Replace the original with the converted file.
    KeepNew,
    /// Discard the converted file and leave the original alone.
    KeepOriginal,
    /// Keep both, renaming the converted file so it no longer looks pending.
    KeepBoth,
}

// AI-FUNC-SUMMARY: Parses a review decision wire value; returns the decision or a user-facing error; side effects: none.
pub fn parse_review_decision(value: &str) -> Result<ReviewDecision, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "keep-new" | "keep_new" | "new" => Ok(ReviewDecision::KeepNew),
        "keep-original" | "keep_original" | "original" => Ok(ReviewDecision::KeepOriginal),
        "keep-both" | "keep_both" | "both" => Ok(ReviewDecision::KeepBoth),
        other => Err(format!(
            "invalid review decision: {other}. Supported values: keep-new, keep-original, keep-both"
        )),
    }
}

// AI-FUNC-SUMMARY: Builds the pending-review output path for a converted file; returns the path carrying the review suffix; side effects: none.
pub fn review_output_path(output_path: &Path) -> PathBuf {
    let stem = output_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("video");
    let extension = output_path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("mp4");
    let file_name = format!("{stem}{REVIEW_SUFFIX}.{extension}");
    output_path
        .parent()
        .map(|dir| dir.join(&file_name))
        .unwrap_or_else(|| PathBuf::from(file_name))
}

// AI-FUNC-SUMMARY: Builds the sidecar path for a pending review file; returns the sidecar path; side effects: none.
pub fn review_sidecar_path(review_path: &Path) -> PathBuf {
    with_added_suffix(review_path, &format!(".{SIDECAR_EXTENSION}"))
}

// AI-FUNC-SUMMARY: Checks whether a file name marks a pending review output or its sidecar; returns true for either; side effects: none.
pub(crate) fn is_review_artifact(path: &Path) -> bool {
    path.file_name()
        .and_then(|value| value.to_str())
        .is_some_and(|name| name.contains(REVIEW_SUFFIX))
}

// AI-FUNC-SUMMARY:
// Purpose: Lists, in one directory read, the source stems that already have a conversion waiting for a decision.
// Inputs: The directory to read.
// Returns: Every `<stem>` for which a `<stem>.myvidcomp-review.<ext>` file (not its sidecar) sits in the directory; empty when the directory cannot be read.
// Side effects: Reads the directory once.
// Notes: A source file has a pending review exactly when its file stem is in this set. Callers cache the set per directory so a scan reads each directory once instead of once per file.
pub(crate) fn pending_review_stems(dir: &Path) -> HashSet<String> {
    let marker = format!("{REVIEW_SUFFIX}.");
    let mut stems = HashSet::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return stems;
    };
    for entry in entries.filter_map(Result::ok) {
        let file_name = entry.file_name();
        let Some(name) = file_name.to_str() else {
            continue;
        };
        if name.ends_with(".json") {
            continue;
        }
        for (index, _) in name.match_indices(&marker) {
            stems.insert(name[..index].to_string());
        }
    }
    stems
}
// AI-FUNC-SUMMARY: Writes the sidecar describing one pending review; returns success or a disk error; side effects: creates or replaces the sidecar file.
pub fn write_sidecar(item: &ReviewItem) -> Result<(), String> {
    fs::write(&item.sidecar_path, sidecar_json(item)).map_err(|err| {
        format!(
            "failed to write review record {}: {err}",
            item.sidecar_path.to_string_lossy()
        )
    })
}

// AI-FUNC-SUMMARY: Serializes one review item as the sidecar JSON document; returns the text; side effects: none.
fn sidecar_json(item: &ReviewItem) -> String {
    let deviations = item
        .deviations
        .iter()
        .map(|deviation| {
            format!(
                "{{\"kind\":{},\"detail\":{}}}",
                json_string(&deviation.kind),
                json_string(&deviation.detail)
            )
        })
        .collect::<Vec<_>>()
        .join(",");

    format!(
        "{{\"version\":1,\"original_path\":{},\"review_path\":{},\"final_path\":{},\"original_bytes\":{},\"review_bytes\":{},\"encoder\":{},\"codec\":{},\"quality\":{},\"score\":{},\"target\":{},\"deviations\":[{}]}}",
        json_string(&item.original_path.to_string_lossy()),
        json_string(&item.review_path.to_string_lossy()),
        json_string(&item.final_path.to_string_lossy()),
        item.original_bytes,
        item.review_bytes,
        json_string(&item.encoder),
        json_string(&item.codec),
        json_string(&item.quality),
        item.score
            .map(|score| score.to_string())
            .unwrap_or_else(|| "null".to_string()),
        item.target,
        deviations
    )
}

// AI-FUNC-SUMMARY: Serializes a list of review items for the FFI; returns a JSON array document; side effects: none.
pub fn review_list_json(items: &[ReviewItem]) -> String {
    let entries = items
        .iter()
        .map(|item| {
            format!(
                "{{\"sidecar_path\":{},\"item\":{}}}",
                json_string(&item.sidecar_path.to_string_lossy()),
                sidecar_json(item)
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("{{\"reviews\":[{entries}]}}")
}

// AI-FUNC-SUMMARY: Finds every pending review under a folder tree; returns the items sorted by original path; side effects: reads directory entries and sidecar files.
pub fn list_reviews(folder: &Path) -> Vec<ReviewItem> {
    let mut items = Vec::new();
    collect_reviews(folder, &mut items);
    items.sort_by(|left, right| left.original_path.cmp(&right.original_path));
    items
}

// AI-FUNC-SUMMARY: Walks one directory level collecting review sidecars; returns nothing; side effects: reads directory entries and sidecar files and recurses into real subdirectories, never through a symbolic link, so a link loop cannot recurse forever.
fn collect_reviews(folder: &Path, items: &mut Vec<ReviewItem>) {
    let Ok(entries) = fs::read_dir(folder) else {
        return;
    };

    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
            collect_reviews(&path, items);
            continue;
        }

        let is_sidecar = path
            .file_name()
            .and_then(|value| value.to_str())
            .is_some_and(|name| {
                name.contains(REVIEW_SUFFIX) && name.ends_with(&format!(".{SIDECAR_EXTENSION}"))
            });
        if !is_sidecar {
            continue;
        }

        if let Some(item) = read_sidecar(&path) {
            items.push(item);
        }
    }
}

// AI-FUNC-SUMMARY: Reads one sidecar file into a review item; returns the item or none when the record is unreadable or its files are gone; side effects: reads the sidecar and checks that both files still exist.
pub fn read_sidecar(sidecar_path: &Path) -> Option<ReviewItem> {
    let text = fs::read_to_string(sidecar_path).ok()?;

    let item = ReviewItem {
        original_path: PathBuf::from(field(&text, "original_path")?),
        review_path: PathBuf::from(field(&text, "review_path")?),
        sidecar_path: sidecar_path.to_path_buf(),
        final_path: PathBuf::from(field(&text, "final_path")?),
        original_bytes: number(&text, "original_bytes").unwrap_or(0),
        review_bytes: number(&text, "review_bytes").unwrap_or(0),
        encoder: field(&text, "encoder").unwrap_or_default(),
        codec: field(&text, "codec").unwrap_or_default(),
        quality: field(&text, "quality").unwrap_or_default(),
        score: number(&text, "score").map(|value| value as u32),
        target: number(&text, "target").unwrap_or(0) as u32,
        deviations: deviations(&text),
    };

    (item.original_path.is_file() && item.review_path.is_file()).then_some(item)
}

// AI-FUNC-SUMMARY: Applies a decision to one pending review; returns success or a user-facing error; side effects: renames or deletes the reviewed files and removes the sidecar; keep-new refuses, before deleting anything, when a different file already holds the final name.
pub fn resolve_review(sidecar_path: &Path, decision: &str) -> Result<(), String> {
    let decision = parse_review_decision(decision)?;
    let item = read_sidecar(sidecar_path).ok_or_else(|| {
        format!(
            "review record is missing or its files have moved: {}",
            sidecar_path.to_string_lossy()
        )
    })?;

    match decision {
        ReviewDecision::KeepNew => {
            if final_path_is_taken(&item) {
                return Err(format!(
                    "cannot keep the converted file: {} already exists and is not the original, so nothing was changed",
                    item.final_path.to_string_lossy()
                ));
            }
            fs::remove_file(&item.original_path).map_err(|err| {
                format!(
                    "failed to remove the original {}: {err}",
                    item.original_path.to_string_lossy()
                )
            })?;
            rename_into_place(&item.review_path, &item.final_path)?;
        }
        ReviewDecision::KeepOriginal => {
            fs::remove_file(&item.review_path).map_err(|err| {
                format!(
                    "failed to remove the converted file {}: {err}",
                    item.review_path.to_string_lossy()
                )
            })?;
        }
        ReviewDecision::KeepBoth => {
            let kept = kept_both_path(&item);
            rename_into_place(&item.review_path, &kept)?;
        }
    }

    let _ = fs::remove_file(sidecar_path);
    Ok(())
}

// AI-FUNC-SUMMARY:
// Purpose: Checks, before the original is deleted, whether keeping the new file would collide with some other file at its final name.
// Inputs: The pending review item.
// Returns: True when a file other than the original (or a symbolic link) already sits at the final name.
// Side effects: Reads file metadata.
// Notes: A final name that is the original itself, including a differently-cased name for the same file on a case-insensitive disk, is not a collision.
fn final_path_is_taken(item: &ReviewItem) -> bool {
    item.final_path != item.original_path
        && fs::symlink_metadata(&item.final_path).is_ok()
        && !is_same_existing_file(&item.final_path, &item.original_path)
}

// AI-FUNC-SUMMARY: Renames a review file to its destination without overwriting anything; returns success or a user-facing error; side effects: renames a file on disk.
fn rename_into_place(from: &Path, to: &Path) -> Result<(), String> {
    if to.exists() && to != from {
        return Err(format!(
            "cannot rename to {}: a file with that name already exists",
            to.to_string_lossy()
        ));
    }
    if to == from {
        return Ok(());
    }
    fs::rename(from, to).map_err(|err| {
        format!(
            "failed to rename {} to {}: {err}",
            from.to_string_lossy(),
            to.to_string_lossy()
        )
    })
}

// AI-FUNC-SUMMARY: Builds the name a converted file takes when the user keeps both copies; returns a path that does not collide with the original; side effects: none.
fn kept_both_path(item: &ReviewItem) -> PathBuf {
    let stem = item
        .final_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("video");
    let extension = item
        .final_path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("mp4");
    let file_name = format!("{stem}.myvidcomp.{extension}");
    item.final_path
        .parent()
        .map(|dir| dir.join(&file_name))
        .unwrap_or_else(|| PathBuf::from(file_name))
}

// AI-FUNC-SUMMARY: Describes a review item for terminal output; returns one summary line; side effects: none.
pub fn review_summary_line(item: &ReviewItem) -> String {
    let score = item
        .score
        .map(format_quality)
        .unwrap_or_else(|| "not measured".to_string());
    let size = item
        .size_percent()
        .map(|percent| format!("{percent:.0}% of the original"))
        .unwrap_or_else(|| "unknown size".to_string());
    let changes = if item.deviations.is_empty() {
        "no changes".to_string()
    } else {
        format!("{} change(s)", item.deviations.len())
    };
    format!(
        "{}: quality {score}, {size}, {changes}",
        item.original_path.to_string_lossy()
    )
}

// AI-FUNC-SUMMARY: Reads one string field out of a sidecar document; returns the unescaped value or none; side effects: none.
fn field(text: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\":\"");
    let start = text.find(&needle)? + needle.len();
    let rest = &text[start..];

    let mut value = String::new();
    let mut chars = rest.chars();
    while let Some(character) = chars.next() {
        match character {
            '"' => return Some(value),
            '\\' => match chars.next()? {
                'n' => value.push('\n'),
                'r' => value.push('\r'),
                't' => value.push('\t'),
                other => value.push(other),
            },
            other => value.push(other),
        }
    }
    None
}

// AI-FUNC-SUMMARY: Reads one numeric field out of a sidecar document; returns the value or none for a missing or null field; side effects: none.
fn number(text: &str, key: &str) -> Option<u64> {
    let needle = format!("\"{key}\":");
    let start = text.find(&needle)? + needle.len();
    let digits: String = text[start..]
        .chars()
        .take_while(|character| character.is_ascii_digit())
        .collect();
    digits.parse().ok()
}

// AI-FUNC-SUMMARY: Reads the deviation list out of a sidecar document; returns every recorded change; side effects: none.
fn deviations(text: &str) -> Vec<Deviation> {
    let Some(start) = text.find("\"deviations\":[") else {
        return Vec::new();
    };
    let rest = &text[start..];
    let Some(end) = closing_bracket(rest) else {
        return Vec::new();
    };

    rest[..end]
        .split("{\"kind\":")
        .skip(1)
        .filter_map(|chunk| {
            let chunk = format!("{{\"kind\":{chunk}");
            Some(Deviation {
                kind: field(&chunk, "kind")?,
                detail: field(&chunk, "detail").unwrap_or_default(),
            })
        })
        .collect()
}

// AI-FUNC-SUMMARY: Finds the `]` that closes a JSON array, skipping any `]` inside a quoted string (escapes included); returns its byte offset or none when the array never closes; side effects: none.
fn closing_bracket(text: &str) -> Option<usize> {
    let mut in_string = false;
    let mut escaped = false;
    for (index, character) in text.char_indices() {
        if in_string {
            match character {
                _ if escaped => escaped = false,
                '\\' => escaped = true,
                '"' => in_string = false,
                _ => {}
            }
        } else {
            match character {
                '"' => in_string = true,
                ']' => return Some(index),
                _ => {}
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::time::{SystemTime, UNIX_EPOCH};

    // AI-FUNC-SUMMARY: Creates an empty temporary directory for one test; returns its path; side effects: creates a directory.
    fn temp_dir(name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let path = env::temp_dir().join(format!("myvidcomp-review-{name}-{stamp}"));
        fs::create_dir_all(&path).unwrap();
        path
    }

    // AI-FUNC-SUMMARY: Writes a review pair and its sidecar into a directory; returns the item; side effects: creates three files.
    fn sample_pair(dir: &Path) -> ReviewItem {
        let original = dir.join("clip.mkv");
        let final_path = dir.join("clip.mp4");
        let review = review_output_path(&final_path);
        fs::write(&original, b"original-bytes").unwrap();
        fs::write(&review, b"new").unwrap();

        let item = ReviewItem {
            original_path: original,
            review_path: review.clone(),
            sidecar_path: review_sidecar_path(&review),
            final_path,
            original_bytes: 14,
            review_bytes: 3,
            encoder: "libsvtav1".to_string(),
            codec: "av1".to_string(),
            quality: "crf=28".to_string(),
            score: Some(9412),
            target: 9500,
            deviations: vec![Deviation::new(
                "chroma_location",
                "Chroma position could not be recorded in this codec.",
            )],
        };
        write_sidecar(&item).unwrap();
        item
    }

    #[test]
    // AI-FUNC-SUMMARY: Verifies that a sidecar survives a write and read cycle unchanged; returns nothing; side effects: creates and removes temporary files.
    fn sidecar_round_trips() {
        let dir = temp_dir("round-trip");
        let written = sample_pair(&dir);

        let read = read_sidecar(&written.sidecar_path).unwrap();

        assert_eq!(read.original_path, written.original_path);
        assert_eq!(read.review_path, written.review_path);
        assert_eq!(read.final_path, written.final_path);
        assert_eq!(read.original_bytes, 14);
        assert_eq!(read.review_bytes, 3);
        assert_eq!(read.encoder, "libsvtav1");
        assert_eq!(read.score, Some(9412));
        assert_eq!(read.target, 9500);
        assert_eq!(read.deviations.len(), 1);
        assert_eq!(read.deviations[0].kind, "chroma_location");
        assert!(read.deviations[0].detail.contains("Chroma position"));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    // AI-FUNC-SUMMARY: Verifies that listing finds a pending review and skips unrelated files; returns nothing; side effects: creates and removes temporary files.
    fn lists_pending_reviews() {
        let dir = temp_dir("list");
        fs::write(dir.join("unrelated.mp4"), b"x").unwrap();
        sample_pair(&dir);

        let items = list_reviews(&dir);

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].encoder, "libsvtav1");
        assert!(items[0].size_percent().unwrap() < 50.0);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    // AI-FUNC-SUMMARY: Verifies that keeping the new file replaces the original and clears the sidecar; returns nothing; side effects: creates and removes temporary files.
    fn keep_new_replaces_the_original() {
        let dir = temp_dir("keep-new");
        let item = sample_pair(&dir);

        resolve_review(&item.sidecar_path, "keep-new").unwrap();

        assert!(!item.original_path.exists());
        assert!(!item.review_path.exists());
        assert!(!item.sidecar_path.exists());
        assert!(item.final_path.is_file());

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    // AI-FUNC-SUMMARY: Verifies that keep-new leaves every file alone when a different file already holds the final name; returns nothing; side effects: creates and removes temporary files.
    fn keep_new_refuses_when_another_file_holds_the_final_name() {
        let dir = temp_dir("keep-new-taken");
        let item = sample_pair(&dir);
        fs::write(&item.final_path, b"someone-else").unwrap();

        let err = resolve_review(&item.sidecar_path, "keep-new").unwrap_err();

        assert!(err.contains("already exists"), "{err}");
        assert_eq!(fs::read(&item.original_path).unwrap(), b"original-bytes");
        assert_eq!(fs::read(&item.review_path).unwrap(), b"new");
        assert_eq!(fs::read(&item.final_path).unwrap(), b"someone-else");
        assert!(item.sidecar_path.is_file());

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    // AI-FUNC-SUMMARY: Verifies that one directory read reports exactly the stems with a pending review file, ignoring sidecars; returns nothing; side effects: creates and removes temporary files.
    fn pending_review_stems_reads_the_directory_once() {
        let dir = temp_dir("pending-stems");
        fs::write(dir.join("clip.mkv"), b"x").unwrap();
        fs::write(dir.join("clip.myvidcomp-review.mp4"), b"x").unwrap();
        fs::write(dir.join("other.myvidcomp-review.mp4.json"), b"{}").unwrap();
        fs::write(dir.join("a-b.myvidcomp-review.mkv"), b"x").unwrap();

        let stems = pending_review_stems(&dir);

        let mut sorted = stems.into_iter().collect::<Vec<_>>();
        sorted.sort();
        assert_eq!(sorted, vec!["a-b".to_string(), "clip".to_string()]);
        assert!(pending_review_stems(&dir.join("missing")).is_empty());

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    // AI-FUNC-SUMMARY: Verifies that deviation text holding brackets, quotes and backslashes survives a sidecar round trip; returns nothing; side effects: creates and removes temporary files.
    fn deviation_text_with_brackets_round_trips() {
        let dir = temp_dir("brackets");
        let mut item = sample_pair(&dir);
        item.deviations = vec![
            Deviation::new("pixel_format", "Converted [yuv444p] to \"yuv420p\" \\ ok]"),
            Deviation::new("dropped_stream", "Stream [2] was dropped."),
        ];
        write_sidecar(&item).unwrap();

        let read = read_sidecar(&item.sidecar_path).unwrap();

        assert_eq!(read.deviations, item.deviations);
        assert_eq!(read.target, 9500);

        fs::remove_dir_all(&dir).ok();
    }

    #[cfg(unix)]
    #[test]
    // AI-FUNC-SUMMARY: Verifies that listing does not follow a symbolic link back into its own folder; returns nothing; side effects: creates and removes temporary files and a link.
    fn listing_does_not_follow_a_directory_link_loop() {
        let dir = temp_dir("link-loop");
        sample_pair(&dir);
        std::os::unix::fs::symlink(&dir, dir.join("loop")).unwrap();

        assert_eq!(list_reviews(&dir).len(), 1);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    // AI-FUNC-SUMMARY: Verifies that keeping the original discards the converted file; returns nothing; side effects: creates and removes temporary files.
    fn keep_original_discards_the_conversion() {
        let dir = temp_dir("keep-original");
        let item = sample_pair(&dir);

        resolve_review(&item.sidecar_path, "keep-original").unwrap();

        assert!(item.original_path.is_file());
        assert!(!item.review_path.exists());
        assert!(!item.sidecar_path.exists());

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    // AI-FUNC-SUMMARY: Verifies that keeping both renames the conversion out of the pending state; returns nothing; side effects: creates and removes temporary files.
    fn keep_both_renames_the_conversion() {
        let dir = temp_dir("keep-both");
        let item = sample_pair(&dir);

        resolve_review(&item.sidecar_path, "keep-both").unwrap();

        assert!(item.original_path.is_file());
        assert!(!item.review_path.exists());
        assert!(dir.join("clip.myvidcomp.mp4").is_file());

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    // AI-FUNC-SUMMARY: Verifies review path naming and artifact detection; returns nothing; side effects: none.
    fn builds_review_names() {
        let review = review_output_path(Path::new("/videos/clip.mp4"));
        assert_eq!(review, PathBuf::from("/videos/clip.myvidcomp-review.mp4"));
        assert_eq!(
            review_sidecar_path(&review),
            PathBuf::from("/videos/clip.myvidcomp-review.mp4.json")
        );
        assert!(is_review_artifact(&review));
        assert!(!is_review_artifact(Path::new("/videos/clip.mp4")));
    }

    #[test]
    // AI-FUNC-SUMMARY: Verifies that every decision wire value parses and that others are rejected; returns nothing; side effects: none.
    fn parses_decisions() {
        assert_eq!(
            parse_review_decision("keep-new"),
            Ok(ReviewDecision::KeepNew)
        );
        assert_eq!(
            parse_review_decision("keep_original"),
            Ok(ReviewDecision::KeepOriginal)
        );
        assert_eq!(parse_review_decision("both"), Ok(ReviewDecision::KeepBoth));
        assert!(parse_review_decision("delete").is_err());
    }
}
