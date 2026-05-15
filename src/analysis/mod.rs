//! Two-pass analysis orchestration: Pass 1 (global PR summary) followed by
//! parallel Pass 2 (per-file categorisation and annotation).
//!
//! [`AnalysisEngine`] coordinates both passes. Pass 1 is run first to produce
//! a [`Pass1Output`], which is then fed into each per-file Pass 2 task that
//! runs in parallel behind a semaphore.
//!
//! # Example
//! ```no_run
//! use std::sync::Arc;
//! use easy_diff::analysis::AnalysisEngine;
//! use easy_diff::llm::LlmDispatcher;
//!
//! // let dispatcher: Arc<LlmDispatcher> = ...;
//! // let engine = AnalysisEngine::new(dispatcher, 5000, 5);
//! // let result = engine.run(diff).await?;
//! ```

pub mod pass1_summary;
pub mod pass2_files;

use std::collections::HashMap;
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
/// parallel, bounded by `max_concurrency`.
pub struct AnalysisEngine {
    dispatcher: Arc<LlmDispatcher>,
    /// Lines-of-diff threshold above which large-PR mode is activated.
    large_pr_threshold: u32,
    /// Maximum number of concurrent Pass 2 LLM calls.
    max_concurrency: usize,
}

impl AnalysisEngine {
    /// Creates a new engine wrapping `dispatcher`.
    ///
    /// `large_pr_threshold` controls when Pass 1 switches to file-names-only
    /// mode. `max_concurrency` caps the number of concurrent Pass 2 calls.
    pub fn new(
        dispatcher: Arc<LlmDispatcher>,
        large_pr_threshold: u32,
        max_concurrency: usize,
    ) -> Self {
        Self {
            dispatcher,
            large_pr_threshold,
            max_concurrency,
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

            join_set.spawn(async move {
                let _permit = sem.acquire().await.expect("semaphore never closed");
                let prompt = build_pass2_prompt(&file_path, &file_diff, &p1);
                let result = async {
                    let value = dispatcher
                        .analyze(&prompt, &schema)
                        .await
                        .context("Pass 2 LLM call failed")?;
                    serde_json::from_value::<Pass2Output>(value)
                        .context("failed to deserialize Pass 2 output")
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
}
