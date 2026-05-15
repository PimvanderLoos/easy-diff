//! Unified diff parsing, per-hunk splitting, and filtered diff formatting
//! for terminal output.
//!
//! The primary entry point is [`parse_diff`], which converts a raw unified diff
//! string (as returned by `git diff` or a GitHub API) into a `Vec<DiffFile>`.
//!
//! # Example
//!
//! ```text
//! let raw = "diff --git a/src/lib.rs b/src/lib.rs\n--- a/src/lib.rs\n+++ b/src/lib.rs\n@@ -1,3 +1,4 @@\n fn foo() {}\n+fn bar() {}\n fn baz() {}\n fn qux() {}\n";
//! let files = parse_diff(raw);
//! assert_eq!(files.len(), 1);
//! ```

// Public types and functions are part of the diff module's API and will be
// consumed by future PRs (filtered output, TUI). Suppress dead_code until then.
#![allow(dead_code)]

use serde::{Deserialize, Serialize};

/// A parsed file entry within a unified diff.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffFile {
    /// The new (or current) path of the file.
    pub path: String,
    /// The original path, set only for renames.
    pub old_path: Option<String>,
    /// All hunks that belong to this file.
    pub hunks: Vec<DiffHunk>,
    /// True when the file was newly created (`--- /dev/null`).
    pub is_new: bool,
    /// True when the file was deleted (`+++ /dev/null`).
    pub is_deleted: bool,
}

/// A single contiguous hunk within a file diff.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffHunk {
    /// The raw `@@ … @@` header line (including any trailing context text).
    pub header: String,
    /// First line number in the old file covered by this hunk.
    pub old_start: u32,
    /// Number of lines from the old file covered by this hunk.
    pub old_count: u32,
    /// First line number in the new file covered by this hunk.
    pub new_start: u32,
    /// Number of lines from the new file covered by this hunk.
    pub new_count: u32,
    /// The classified lines of this hunk.
    pub lines: Vec<DiffLine>,
}

/// A single line within a hunk, classified by its diff marker.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DiffLine {
    /// A line present in both old and new (` ` prefix).
    Context(String),
    /// A line added in the new version (`+` prefix).
    Added(String),
    /// A line removed from the old version (`-` prefix).
    Removed(String),
}

/// Parses a full unified diff string into a structured list of [`DiffFile`] entries.
///
/// Lines that cannot be classified (e.g. `\ No newline at end of file`) are silently
/// skipped. Binary file markers are recognised: the file is included with an empty
/// `hunks` vec so callers know it exists.
pub fn parse_diff(diff: &str) -> Vec<DiffFile> {
    let mut files: Vec<DiffFile> = Vec::new();

    // Each file block starts with `diff --git a/<path> b/<path>`.
    // We split the input on those lines and process each block in turn.
    let mut current_block: Vec<&str> = Vec::new();

    for line in diff.lines() {
        if line.starts_with("diff --git ") && !current_block.is_empty() {
            if let Some(f) = parse_file_block(&current_block) {
                files.push(f);
            }
            current_block.clear();
        }
        current_block.push(line);
    }

    // Handle the last block.
    if !current_block.is_empty() {
        if let Some(f) = parse_file_block(&current_block) {
            files.push(f);
        }
    }

    files
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Parse a single file block (everything from one `diff --git` line up to the
/// next) into a [`DiffFile`], or `None` if the block header is malformed.
fn parse_file_block(lines: &[&str]) -> Option<DiffFile> {
    let header = lines.first()?;

    // Extract the `b/<path>` side from `diff --git a/<path> b/<path>`.
    // The path may contain spaces; take everything after the last ` b/`.
    let path = extract_diff_git_path(header)?;

    let mut file = DiffFile {
        path,
        old_path: None,
        hunks: Vec::new(),
        is_new: false,
        is_deleted: false,
    };

    let mut i = 1usize;
    let n = lines.len();

    // --- Extended header lines (between `diff --git` and `--- …`) ---
    while i < n {
        let line = lines[i];

        if line.starts_with("--- ") || line.starts_with("+++ ") || line.starts_with("@@ ") {
            // Reached the file-metadata or first hunk.
            break;
        }

        if let Some(from_path) = line.strip_prefix("rename from ") {
            file.old_path = Some(from_path.to_owned());
        } else if let Some(to_path) = line.strip_prefix("rename to ") {
            // `rename to` gives the new path — use it to override the path we
            // extracted from the `diff --git` header (they should be identical,
            // but being explicit avoids any edge-case mismatch).
            file.path = to_path.to_owned();
        } else if line.starts_with("Binary files ") && line.ends_with(" differ") {
            // Binary file: record it with no hunks and return early.
            return Some(file);
        }

        i += 1;
    }

    // --- `--- …` / `+++ …` metadata lines ---
    while i < n {
        let line = lines[i];
        if let Some(path_part) = line.strip_prefix("--- ") {
            if path_part == "/dev/null" {
                file.is_new = true;
            }
            i += 1;
        } else if let Some(path_part) = line.strip_prefix("+++ ") {
            if path_part == "/dev/null" {
                file.is_deleted = true;
            }
            i += 1;
        } else {
            break;
        }
    }

    // --- Hunks ---
    while i < n {
        let line = lines[i];

        if line.starts_with("@@ ") {
            let (hunk, consumed) = parse_hunk(&lines[i..]);
            if let Some(h) = hunk {
                file.hunks.push(h);
            }
            i += consumed;
        } else {
            i += 1;
        }
    }

    Some(file)
}

/// Extract the new-side path from a `diff --git a/<old> b/<new>` line.
///
/// Git always uses ` b/` as the separator between old and new paths; we find
/// the last occurrence so that paths containing ` b/` as a substring are still
/// handled correctly in the common case where both sides have the same length.
fn extract_diff_git_path(header: &str) -> Option<String> {
    // Format: `diff --git a/<old> b/<new>`
    // Strip the leading `diff --git ` prefix first.
    let rest = header.strip_prefix("diff --git ")?;

    // The line encodes `a/<old> b/<new>`. When paths don't contain spaces this
    // is unambiguous. When they do, git quotes them — but most diffs in the
    // wild don't. We find the ` b/` that splits the two sides by looking at
    // all candidate split points and choosing the one where old == new (the
    // most common case). Failing that we fall back to the last ` b/`.
    //
    // For our use-case (real PR diffs) the simple heuristic is sufficient.
    let b_marker = " b/";
    let mut best: Option<&str> = None;

    let bytes = rest.as_bytes();
    let marker = b_marker.as_bytes();
    let mut pos = 0usize;
    while pos + marker.len() <= bytes.len() {
        if bytes[pos..].starts_with(marker) {
            let a_side = &rest[2..pos]; // strip leading `a/`
            let b_side = &rest[pos + 3..]; // strip ` b/`
            if a_side == b_side {
                // Perfect match — this is the split point.
                return Some(b_side.to_owned());
            }
            best = Some(&rest[pos + 3..]);
        }
        pos += 1;
    }

    best.map(|s| s.to_owned())
}

/// Parse a hunk starting at `lines[0]` (which must be the `@@ … @@` line).
///
/// Returns `(Option<DiffHunk>, lines_consumed)`.
fn parse_hunk(lines: &[&str]) -> (Option<DiffHunk>, usize) {
    let header_line = lines[0];

    let Some((old_start, old_count, new_start, new_count)) = parse_hunk_header(header_line) else {
        return (None, 1);
    };

    let hunk_header = header_line.to_owned();
    let mut diff_lines: Vec<DiffLine> = Vec::new();
    let mut consumed = 1usize;

    for line in &lines[1..] {
        if line.starts_with("@@ ") || line.starts_with("diff --git ") {
            break;
        }

        if let Some(content) = line.strip_prefix('+') {
            diff_lines.push(DiffLine::Added(content.to_owned()));
        } else if let Some(content) = line.strip_prefix('-') {
            diff_lines.push(DiffLine::Removed(content.to_owned()));
        } else if let Some(content) = line.strip_prefix(' ') {
            diff_lines.push(DiffLine::Context(content.to_owned()));
        }
        // Lines starting with `\` (e.g. `\ No newline at end of file`) are skipped.

        consumed += 1;
    }

    let hunk = DiffHunk {
        header: hunk_header,
        old_start,
        old_count,
        new_start,
        new_count,
        lines: diff_lines,
    };

    (Some(hunk), consumed)
}

/// Parse a `@@ -old_start[,old_count] +new_start[,new_count] @@` header.
///
/// Returns `(old_start, old_count, new_start, new_count)` or `None` on parse
/// failure. When the count is omitted it defaults to 1.
fn parse_hunk_header(line: &str) -> Option<(u32, u32, u32, u32)> {
    // Example: `@@ -1,3 +1,4 @@ fn foo() {}`
    // We only care about the part between the first `@@` pair.
    let inner = line.strip_prefix("@@ ")?;
    let end = inner.find(" @@")?;
    let range_part = &inner[..end]; // e.g. `-1,3 +1,4`

    let mut parts = range_part.split_whitespace();
    let old_range = parts.next()?; // `-1,3`
    let new_range = parts.next()?; // `+1,4`

    let old_range = old_range.strip_prefix('-')?;
    let new_range = new_range.strip_prefix('+')?;

    let (old_start, old_count) = parse_range(old_range)?;
    let (new_start, new_count) = parse_range(new_range)?;

    Some((old_start, old_count, new_start, new_count))
}

/// Parse `start[,count]` returning `(start, count)`. Count defaults to 1.
fn parse_range(s: &str) -> Option<(u32, u32)> {
    if let Some((start_s, count_s)) = s.split_once(',') {
        let start = start_s.parse::<u32>().ok()?;
        let count = count_s.parse::<u32>().ok()?;
        Some((start, count))
    } else {
        let start = s.parse::<u32>().ok()?;
        Some((start, 1))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Fixtures
    // -----------------------------------------------------------------------

    const SINGLE_FILE_SINGLE_HUNK: &str = "\
diff --git a/src/lib.rs b/src/lib.rs
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -1,3 +1,4 @@
 fn foo() {}
+fn bar() {}
 fn baz() {}
 fn qux() {}
";

    const SINGLE_FILE_MULTIPLE_HUNKS: &str = "\
diff --git a/src/lib.rs b/src/lib.rs
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -1,3 +1,4 @@
 fn foo() {}
+fn bar() {}
 fn baz() {}
 fn qux() {}
@@ -10,3 +11,4 @@
 fn alpha() {}
+fn beta() {}
 fn gamma() {}
 fn delta() {}
";

    const MULTIPLE_FILES: &str = "\
diff --git a/src/a.rs b/src/a.rs
--- a/src/a.rs
+++ b/src/a.rs
@@ -1,2 +1,3 @@
 fn a() {}
+fn aa() {}
 fn aaa() {}
diff --git a/src/b.rs b/src/b.rs
--- a/src/b.rs
+++ b/src/b.rs
@@ -1,2 +1,3 @@
 fn b() {}
+fn bb() {}
 fn bbb() {}
diff --git a/src/c.rs b/src/c.rs
--- a/src/c.rs
+++ b/src/c.rs
@@ -1,2 +1,3 @@
 fn c() {}
+fn cc() {}
 fn ccc() {}
";

    const NEW_FILE: &str = "\
diff --git a/src/new.rs b/src/new.rs
new file mode 100644
--- /dev/null
+++ b/src/new.rs
@@ -0,0 +1,2 @@
+fn hello() {}
+fn world() {}
";

    const DELETED_FILE: &str = "\
diff --git a/src/old.rs b/src/old.rs
deleted file mode 100644
--- a/src/old.rs
+++ /dev/null
@@ -1,2 +0,0 @@
-fn goodbye() {}
-fn world() {}
";

    const RENAME: &str = "\
diff --git a/src/old_name.rs b/src/new_name.rs
similarity index 95%
rename from src/old_name.rs
rename to src/new_name.rs
--- a/src/old_name.rs
+++ b/src/new_name.rs
@@ -1,3 +1,3 @@
 fn foo() {}
-fn old() {}
+fn new_fn() {}
 fn bar() {}
";

    const BINARY: &str = "\
diff --git a/assets/logo.png b/assets/logo.png
index abc1234..def5678 100644
Binary files a/assets/logo.png and b/assets/logo.png differ
";

    // -----------------------------------------------------------------------
    // Tests
    // -----------------------------------------------------------------------

    #[test]
    fn parse_single_file_single_hunk() {
        // setup
        let input = SINGLE_FILE_SINGLE_HUNK;

        // execute
        let files = parse_diff(input);

        // verify
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "src/lib.rs");
        assert_eq!(files[0].hunks.len(), 1);
    }

    #[test]
    fn parse_single_file_multiple_hunks() {
        // setup
        let input = SINGLE_FILE_MULTIPLE_HUNKS;

        // execute
        let files = parse_diff(input);

        // verify
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].hunks.len(), 2);
    }

    #[test]
    fn parse_multiple_files() {
        // setup
        let input = MULTIPLE_FILES;

        // execute
        let files = parse_diff(input);

        // verify
        assert_eq!(files.len(), 3);
        assert_eq!(files[0].path, "src/a.rs");
        assert_eq!(files[1].path, "src/b.rs");
        assert_eq!(files[2].path, "src/c.rs");
    }

    #[test]
    fn parse_new_file() {
        // setup
        let input = NEW_FILE;

        // execute
        let files = parse_diff(input);

        // verify
        assert_eq!(files.len(), 1);
        assert!(files[0].is_new, "expected is_new = true");
        assert!(!files[0].is_deleted);
    }

    #[test]
    fn parse_deleted_file() {
        // setup
        let input = DELETED_FILE;

        // execute
        let files = parse_diff(input);

        // verify
        assert_eq!(files.len(), 1);
        assert!(files[0].is_deleted, "expected is_deleted = true");
        assert!(!files[0].is_new);
    }

    #[test]
    fn parse_rename() {
        // setup
        let input = RENAME;

        // execute
        let files = parse_diff(input);

        // verify
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "src/new_name.rs");
        assert_eq!(
            files[0].old_path.as_deref(),
            Some("src/old_name.rs"),
            "old_path should be set for renames"
        );
    }

    #[test]
    fn parse_binary_skipped() {
        // setup
        let input = BINARY;

        // execute
        let files = parse_diff(input);

        // verify — file present but no hunks
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "assets/logo.png");
        assert!(
            files[0].hunks.is_empty(),
            "binary files should have no hunks"
        );
    }

    #[test]
    fn parse_empty_diff() {
        // setup
        let input = "";

        // execute
        let files = parse_diff(input);

        // verify
        assert!(files.is_empty(), "empty input should produce empty vec");
    }

    #[test]
    fn line_classification() {
        // setup
        let input = SINGLE_FILE_SINGLE_HUNK;

        // execute
        let files = parse_diff(input);

        // verify
        let lines = &files[0].hunks[0].lines;
        assert_eq!(lines[0], DiffLine::Context("fn foo() {}".to_owned()));
        assert_eq!(lines[1], DiffLine::Added("fn bar() {}".to_owned()));
        assert_eq!(lines[2], DiffLine::Context("fn baz() {}".to_owned()));
        assert_eq!(lines[3], DiffLine::Context("fn qux() {}".to_owned()));
    }

    #[test]
    fn hunk_numbers_parsed() {
        // setup
        let input = SINGLE_FILE_MULTIPLE_HUNKS;

        // execute
        let files = parse_diff(input);

        // verify first hunk: `@@ -1,3 +1,4 @@`
        let h0 = &files[0].hunks[0];
        assert_eq!(h0.old_start, 1);
        assert_eq!(h0.old_count, 3);
        assert_eq!(h0.new_start, 1);
        assert_eq!(h0.new_count, 4);

        // verify second hunk: `@@ -10,3 +11,4 @@`
        let h1 = &files[0].hunks[1];
        assert_eq!(h1.old_start, 10);
        assert_eq!(h1.old_count, 3);
        assert_eq!(h1.new_start, 11);
        assert_eq!(h1.new_count, 4);
    }
}
