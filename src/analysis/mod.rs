//! Two-pass analysis orchestration: Pass 1 (global PR summary) followed by
//! parallel Pass 2 (per-file categorisation and annotation).
//!
//! [`AnalysisEngine`] coordinates both passes. Pass 1 is run first to produce
//! a [`Pass1Output`], which is then fed into each per-file Pass 2 task that
//! runs in parallel behind a semaphore.
//!
//! When a single file's diff exceeds `max_file_context` lines, [`split_into_chunks`]
//! splits it at hunk boundaries and each chunk is analysed separately. The results
//! are merged with [`merge_pass2_outputs`] before being stored.
//!
//! # Example
//! ```no_run
//! use std::sync::Arc;
//! use easy_diff::analysis::AnalysisEngine;
//! use easy_diff::llm::LlmDispatcher;
//!
//! // let dispatcher: Arc<LlmDispatcher> = ...;
//! // let engine = AnalysisEngine::new(dispatcher, 5000, 5, 1000);
//! // let result = engine.run(diff).await?;
//! ```

pub mod pass1_summary;
pub mod pass2_files;

use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;

use anyhow::Context as _;
use serde::{Deserialize, Serialize};
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

use crate::llm::schema::{schema_for_pass1, schema_for_pass2, Pass1Output, Pass2Output};
use crate::llm::LlmDispatcher;

use pass1_summary::build_pass1_prompt;
use pass2_files::build_pass2_prompt;

/// Combined result of both analysis passes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    /// Global PR summary produced by Pass 1.
    pub pass1: Pass1Output,
    /// Per-file analysis produced by Pass 2, keyed by file path.
    pub files: HashMap<String, Pass2Output>,
}

/// Orchestrates the two-pass analysis pipeline.
///
/// Pass 1 analyses the entire diff at once; Pass 2 analyses each file in
/// parallel, bounded by `max_concurrency`. Files whose diff exceeds
/// `max_file_context` lines are split into hunk-aligned chunks and each
/// chunk is analysed separately before results are merged.
pub struct AnalysisEngine {
    dispatcher: Arc<LlmDispatcher>,
    /// Lines-of-diff threshold above which large-PR mode is activated.
    large_pr_threshold: u32,
    /// Maximum number of concurrent Pass 2 LLM calls.
    max_concurrency: usize,
    /// Maximum lines per file diff sent to Pass 2 in a single LLM call.
    max_file_context: u32,
}

impl AnalysisEngine {
    /// Creates a new engine wrapping `dispatcher`.
    ///
    /// - `large_pr_threshold`: when the total diff line count exceeds this,
    ///   Pass 1 switches to file-names-only mode.
    /// - `max_concurrency`: caps the number of concurrent Pass 2 LLM calls.
    /// - `max_file_context`: maximum lines per file diff for a single Pass 2
    ///   call; oversized diffs are split at hunk boundaries before analysis.
    pub fn new(
        dispatcher: Arc<LlmDispatcher>,
        large_pr_threshold: u32,
        max_concurrency: usize,
        max_file_context: u32,
    ) -> Self {
        Self {
            dispatcher,
            large_pr_threshold,
            max_concurrency,
            max_file_context,
        }
    }

    /// Runs the full two-pass analysis and returns the combined result.
    ///
    /// Files that fail Pass 2 are omitted from `result.files`; a warning is
    /// logged for each failure.
    pub async fn run(&self, diff: &str) -> anyhow::Result<AnalysisResult> {
        // ── Pass 1 ─────────────────────────────────────────────────────────
        let file_pairs = split_diff_by_file(diff);
        let file_names: Vec<&str> = file_pairs.iter().map(|(name, _)| name.as_str()).collect();

        let line_count = diff.lines().count() as u32;
        let large_pr = line_count > self.large_pr_threshold;
        if large_pr {
            tracing::info!(
                lines = line_count,
                threshold = self.large_pr_threshold,
                "large PR detected — Pass 1 uses file-names-only mode"
            );
        }

        let pass1_prompt = build_pass1_prompt(diff, &file_names, large_pr);
        let pass1_schema = schema_for_pass1();

        let pass1_value = self
            .dispatcher
            .analyze(&pass1_prompt, &pass1_schema)
            .await
            .context("Pass 1 LLM call failed")?;

        let pass1: Pass1Output =
            serde_json::from_value(pass1_value).context("failed to deserialize Pass 1 output")?;

        tracing::info!(
            files = file_pairs.len(),
            "Pass 1 complete — starting parallel Pass 2"
        );

        // ── Pass 2 ─────────────────────────────────────────────────────────
        let semaphore = Arc::new(Semaphore::new(self.max_concurrency));
        let pass2_schema = schema_for_pass2();
        let pass1_arc = Arc::new(pass1.clone());

        let mut join_set: JoinSet<(String, anyhow::Result<Pass2Output>)> = JoinSet::new();

        for (file_path, file_diff) in file_pairs {
            let dispatcher = Arc::clone(&self.dispatcher);
            let sem = Arc::clone(&semaphore);
            let schema = pass2_schema.clone();
            let p1 = Arc::clone(&pass1_arc);
            let max_file_context = self.max_file_context;

            join_set.spawn(async move {
                let _permit = sem.acquire().await.expect("semaphore never closed");

                let chunks = if file_diff.lines().count() as u32 > max_file_context {
                    tracing::debug!(
                        file = %file_path,
                        lines = file_diff.lines().count(),
                        max = max_file_context,
                        "file diff exceeds threshold — splitting into chunks"
                    );
                    split_into_chunks(&file_diff, max_file_context)
                } else {
                    vec![file_diff]
                };

                let result: anyhow::Result<Pass2Output> = async {
                    let mut outputs: Vec<Pass2Output> = Vec::with_capacity(chunks.len());
                    for chunk in &chunks {
                        let prompt = build_pass2_prompt(&file_path, chunk, &p1);
                        let value = dispatcher
                            .analyze(&prompt, &schema)
                            .await
                            .context("Pass 2 LLM call failed")?;
                        let output = serde_json::from_value::<Pass2Output>(value)
                            .context("failed to deserialize Pass 2 output")?;
                        outputs.push(output);
                    }
                    Ok(merge_pass2_outputs(outputs))
                }
                .await;

                (file_path, result)
            });
        }

        let mut files: HashMap<String, Pass2Output> = HashMap::new();

        while let Some(join_result) = join_set.join_next().await {
            match join_result {
                Ok((path, Ok(output))) => {
                    files.insert(path, output);
                }
                Ok((path, Err(e))) => {
                    tracing::warn!(file = %path, error = %e, "Pass 2 failed for file — skipping");
                }
                Err(e) => {
                    tracing::warn!(error = %e, "Pass 2 task panicked — skipping");
                }
            }
        }

        tracing::info!(successful = files.len(), "Pass 2 complete");

        Ok(AnalysisResult { pass1, files })
    }
}

/// Splits a file diff into chunks of at most `max_lines` lines each,
/// breaking at hunk boundaries (`@@ ... @@`).
///
/// Each chunk retains the file header (`--- a/...` / `+++ b/...`) so the
/// LLM has file context. If the diff has fewer than or equal to `max_lines`
/// lines, returns a single-element vec with the original diff.
///
/// A single hunk that exceeds `max_lines` on its own is returned as-is —
/// it cannot be split further.
pub fn split_into_chunks(file_diff: &str, max_lines: u32) -> Vec<String> {
    let line_count = file_diff.lines().count() as u32;
    if line_count <= max_lines {
        return vec![file_diff.to_owned()];
    }

    // Collect header lines (everything before the first `@@`).
    let all_lines: Vec<&str> = file_diff.lines().collect();
    let first_hunk_idx = all_lines.iter().position(|l| l.starts_with("@@"));

    let Some(first_hunk_idx) = first_hunk_idx else {
        // No hunks at all — return as-is.
        return vec![file_diff.to_owned()];
    };

    let header_lines = &all_lines[..first_hunk_idx];
    let header = header_lines.join("\n");
    let header_len = header_lines.len() as u32;

    // Split remaining lines into individual hunks.
    // Each hunk starts at a line beginning with `@@`.
    let mut hunks: Vec<Vec<&str>> = Vec::new();
    let mut current_hunk: Vec<&str> = Vec::new();

    for line in &all_lines[first_hunk_idx..] {
        if line.starts_with("@@") && !current_hunk.is_empty() {
            hunks.push(current_hunk);
            current_hunk = Vec::new();
        }
        current_hunk.push(line);
    }
    if !current_hunk.is_empty() {
        hunks.push(current_hunk);
    }

    // Greedily group hunks until adding the next would exceed max_lines.
    let mut chunks: Vec<String> = Vec::new();
    let mut group: Vec<&[&str]> = Vec::new();
    let mut group_lines: u32 = header_len;

    for hunk in &hunks {
        let hunk_len = hunk.len() as u32;
        // Always include at least one hunk per chunk, even if it exceeds max_lines.
        if !group.is_empty() && group_lines + hunk_len > max_lines {
            // Flush current group.
            let mut chunk = header.clone();
            for h in &group {
                chunk.push('\n');
                chunk.push_str(&h.join("\n"));
            }
            chunks.push(chunk);
            group = Vec::new();
            group_lines = header_len;
        }
        group.push(hunk.as_slice());
        group_lines += hunk_len;
    }

    // Flush the last group.
    if !group.is_empty() {
        let mut chunk = header.clone();
        for h in &group {
            chunk.push('\n');
            chunk.push_str(&h.join("\n"));
        }
        chunks.push(chunk);
    }

    chunks
}

/// Merges multiple [`Pass2Output`] results from chunked analysis into one.
///
/// Change types and attention tags are de-duplicated (union). Summaries are
/// joined with a space separator. Details from all chunks are concatenated.
pub fn merge_pass2_outputs(outputs: Vec<Pass2Output>) -> Pass2Output {
    if outputs.is_empty() {
        return Pass2Output {
            summary: String::new(),
            change_types: Vec::new(),
            attention_tags: Vec::new(),
            details: Vec::new(),
        };
    }

    if outputs.len() == 1 {
        return outputs.into_iter().next().expect("checked non-empty");
    }

    let mut seen_change_types: HashSet<crate::categories::ChangeType> = HashSet::new();
    let mut seen_attention_tags: HashSet<crate::categories::AttentionTag> = HashSet::new();
    let mut summaries: Vec<String> = Vec::new();
    let mut details: Vec<String> = Vec::new();

    for output in outputs {
        summaries.push(output.summary);
        for ct in output.change_types {
            seen_change_types.insert(ct);
        }
        for tag in output.attention_tags {
            seen_attention_tags.insert(tag);
        }
        details.extend(output.details);
    }

    Pass2Output {
        summary: summaries.join(" "),
        change_types: seen_change_types.into_iter().collect(),
        attention_tags: seen_attention_tags.into_iter().collect(),
        details,
    }
}

/// Splits a unified diff into `(file_path, file_diff)` pairs.
///
/// Splits on `diff --git` header lines. The file path is taken from the `b/`
/// side of the header (i.e. `diff --git a/... b/<path>`). Each returned diff
/// string includes the `diff --git` header line and all subsequent lines up to
/// the next `diff --git` header.
pub fn split_diff_by_file(diff: &str) -> Vec<(String, String)> {
    if diff.is_empty() {
        return Vec::new();
    }

    let mut result: Vec<(String, String)> = Vec::new();
    let mut current_path: Option<String> = None;
    let mut current_lines: Vec<&str> = Vec::new();

    for line in diff.lines() {
        if let Some(rest) = line.strip_prefix("diff --git ") {
            // Flush the previous file.
            if let Some(path) = current_path.take() {
                result.push((path, current_lines.join("\n")));
                current_lines.clear();
            }
            // Extract path from `a/<path> b/<path>` — take the `b/` side.
            current_path = parse_b_path(rest);
            current_lines.push(line);
        } else {
            current_lines.push(line);
        }
    }

    // Flush the last file.
    if let Some(path) = current_path {
        result.push((path, current_lines.join("\n")));
    }

    result
}

/// Extracts the `b/<path>` side from a `diff --git` header suffix.
///
/// The suffix is the part after `diff --git `, e.g. `a/foo.rs b/foo.rs`.
fn parse_b_path(header_suffix: &str) -> Option<String> {
    // The header format is `a/<path> b/<path>`.  Because file paths can
    // contain spaces, we split at the last ` b/` occurrence.
    let b_marker = " b/";
    let pos = header_suffix.rfind(b_marker)?;
    Some(header_suffix[pos + b_marker.len()..].to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Realistic diff fixture helpers.

    fn single_file_diff() -> &'static str {
        "diff --git a/src/main.rs b/src/main.rs\n\
         index 1234567..abcdef0 100644\n\
         --- a/src/main.rs\n\
         +++ b/src/main.rs\n\
         @@ -1,3 +1,3 @@\n\
         -fn old() {}\n\
         +fn new() {}\n\
          fn unchanged() {}"
    }

    fn three_file_diff() -> &'static str {
        "diff --git a/src/a.rs b/src/a.rs\n\
         index 0000000..1111111 100644\n\
         --- a/src/a.rs\n\
         +++ b/src/a.rs\n\
         @@ -1 +1 @@\n\
         -old a\n\
         +new a\n\
         diff --git a/src/b.rs b/src/b.rs\n\
         index 2222222..3333333 100644\n\
         --- a/src/b.rs\n\
         +++ b/src/b.rs\n\
         @@ -1 +1 @@\n\
         -old b\n\
         +new b\n\
         diff --git a/src/c.rs b/src/c.rs\n\
         index 4444444..5555555 100644\n\
         --- a/src/c.rs\n\
         +++ b/src/c.rs\n\
         @@ -1 +1 @@\n\
         -old c\n\
         +new c"
    }

    #[test]
    fn split_diff_by_file_empty() {
        // setup
        let diff = "";

        // execute
        let pairs = split_diff_by_file(diff);

        // verify
        assert!(pairs.is_empty(), "empty diff should produce no pairs");
    }

    #[test]
    fn split_diff_by_file_single_file() {
        // setup
        let diff = single_file_diff();

        // execute
        let pairs = split_diff_by_file(diff);

        // verify
        assert_eq!(pairs.len(), 1, "expected one pair for a single-file diff");
        assert_eq!(pairs[0].0, "src/main.rs");
    }

    #[test]
    fn split_diff_by_file_multiple_files() {
        // setup
        let diff = three_file_diff();

        // execute
        let pairs = split_diff_by_file(diff);

        // verify
        assert_eq!(pairs.len(), 3, "expected three pairs");
        assert_eq!(pairs[0].0, "src/a.rs");
        assert_eq!(pairs[1].0, "src/b.rs");
        assert_eq!(pairs[2].0, "src/c.rs");
    }

    #[test]
    fn split_diff_by_file_preserves_content() {
        // setup
        let diff = single_file_diff();

        // execute
        let pairs = split_diff_by_file(diff);

        // verify — the returned diff chunk must contain the key diff lines
        let file_diff = &pairs[0].1;
        assert!(
            file_diff.contains("-fn old() {}"),
            "file diff should contain the removed line"
        );
        assert!(
            file_diff.contains("+fn new() {}"),
            "file diff should contain the added line"
        );
        assert!(
            file_diff.contains("diff --git"),
            "file diff should include the header line"
        );
    }

    // ── split_into_chunks tests ───────────────────────────────────────────────

    /// Builds a minimal file diff with a header and two hunks of 3 lines each.
    fn two_hunk_file_diff() -> String {
        // header: 2 lines
        // hunk 1: @@ line + 2 content lines = 3 lines
        // hunk 2: @@ line + 2 content lines = 3 lines
        // total: 8 lines
        "--- a/src/lib.rs\n\
         +++ b/src/lib.rs\n\
         @@ -1,2 +1,2 @@\n\
         -old_a\n\
         +new_a\n\
         @@ -10,2 +10,2 @@\n\
         -old_b\n\
         +new_b"
            .to_owned()
    }

    #[test]
    fn split_into_chunks_short_diff_returns_one() {
        // setup
        let diff = two_hunk_file_diff(); // 8 lines

        // execute
        let chunks = split_into_chunks(&diff, 100);

        // verify
        assert_eq!(chunks.len(), 1, "short diff should produce one chunk");
        assert_eq!(chunks[0], diff, "chunk should equal the original diff");
    }

    #[test]
    fn split_into_chunks_splits_at_hunk_boundary() {
        // setup — 8 lines total; header=2, hunk1=3, hunk2=3
        // max_lines=5 forces hunk1 and hunk2 into separate chunks
        let diff = two_hunk_file_diff();

        // execute
        let chunks = split_into_chunks(&diff, 5);

        // verify
        assert_eq!(chunks.len(), 2, "should split into two chunks");
        assert!(
            chunks[0].contains("@@ -1,2 +1,2 @@"),
            "first chunk should contain the first hunk"
        );
        assert!(
            chunks[1].contains("@@ -10,2 +10,2 @@"),
            "second chunk should contain the second hunk"
        );
    }

    #[test]
    fn split_into_chunks_preserves_file_header() {
        // setup
        let diff = two_hunk_file_diff();

        // execute
        let chunks = split_into_chunks(&diff, 5);

        // verify — every chunk must start with the file header
        for chunk in &chunks {
            assert!(
                chunk.starts_with("--- a/src/lib.rs"),
                "each chunk should start with the file header, got: {chunk}"
            );
            assert!(
                chunk.contains("+++ b/src/lib.rs"),
                "each chunk should contain the +++ header line"
            );
        }
    }

    #[test]
    fn split_into_chunks_respects_max_lines() {
        // setup — diff with 3 hunks of 4 lines each; header = 2 lines
        // total = 2 + 3*4 = 14 lines; max_lines = 7
        // header(2) + hunk1(4) = 6 <= 7 → group
        // header(2) + hunk1(4) + hunk2(4) = 10 > 7 → flush after hunk1
        // header(2) + hunk2(4) = 6 <= 7 → group
        // header(2) + hunk2(4) + hunk3(4) = 10 > 7 → flush after hunk2
        // header(2) + hunk3(4) = 6 — final group
        let diff = "--- a/big.rs\n\
                    +++ b/big.rs\n\
                    @@ -1,3 +1,3 @@\n\
                     ctx\n\
                    -old1\n\
                    +new1\n\
                    @@ -10,3 +10,3 @@\n\
                     ctx\n\
                    -old2\n\
                    +new2\n\
                    @@ -20,3 +20,3 @@\n\
                     ctx\n\
                    -old3\n\
                    +new3";
        let max_lines = 7u32;

        // execute
        let chunks = split_into_chunks(diff, max_lines);

        // verify — no chunk (ignoring header) should push a normal hunk over limit
        // Each chunk is at most header(2) + one hunk(4) = 6 lines here.
        for chunk in &chunks {
            let line_count = chunk.lines().count() as u32;
            assert!(
                line_count <= max_lines,
                "chunk has {line_count} lines, exceeds max {max_lines}"
            );
        }
        assert_eq!(
            chunks.len(),
            3,
            "three hunks at threshold should yield three chunks"
        );
    }

    // ── merge_pass2_outputs tests ─────────────────────────────────────────────

    use crate::categories::{AttentionTag, ChangeType};

    fn make_pass2(
        summary: &str,
        change_types: Vec<ChangeType>,
        attention_tags: Vec<AttentionTag>,
        details: Vec<&str>,
    ) -> Pass2Output {
        Pass2Output {
            summary: summary.to_owned(),
            change_types,
            attention_tags,
            details: details.into_iter().map(str::to_owned).collect(),
        }
    }

    #[test]
    fn merge_pass2_deduplicates_categories() {
        // setup
        let a = make_pass2(
            "a",
            vec![ChangeType::Feature, ChangeType::BugFix],
            vec![],
            vec![],
        );
        let b = make_pass2(
            "b",
            vec![ChangeType::Feature, ChangeType::Refactor],
            vec![],
            vec![],
        );

        // execute
        let merged = merge_pass2_outputs(vec![a, b]);

        // verify — Feature appears in both; should appear once
        let mut ct = merged.change_types.clone();
        ct.sort_by_key(|v| format!("{v}"));
        assert_eq!(ct.len(), 3, "expected 3 unique change types");
        assert!(merged.change_types.contains(&ChangeType::Feature));
        assert!(merged.change_types.contains(&ChangeType::BugFix));
        assert!(merged.change_types.contains(&ChangeType::Refactor));
    }

    #[test]
    fn merge_pass2_concatenates_summaries() {
        // setup
        let a = make_pass2("First summary.", vec![], vec![], vec![]);
        let b = make_pass2("Second summary.", vec![], vec![], vec![]);

        // execute
        let merged = merge_pass2_outputs(vec![a, b]);

        // verify
        assert_eq!(merged.summary, "First summary. Second summary.");
    }

    #[test]
    fn merge_pass2_concatenates_details() {
        // setup
        let a = make_pass2("a", vec![], vec![], vec!["detail 1", "detail 2"]);
        let b = make_pass2("b", vec![], vec![], vec!["detail 3"]);

        // execute
        let merged = merge_pass2_outputs(vec![a, b]);

        // verify
        assert_eq!(merged.details.len(), 3);
        assert!(merged.details.contains(&"detail 1".to_owned()));
        assert!(merged.details.contains(&"detail 2".to_owned()));
        assert!(merged.details.contains(&"detail 3".to_owned()));
    }
}
