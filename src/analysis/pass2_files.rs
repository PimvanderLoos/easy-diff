//! Pass 2: per-file Change Type and Attention Tag classification, run in parallel.
//!
//! This module builds the LLM prompt for Pass 2, which analyses a single file's
//! diff in the context of the global summary produced by Pass 1. The prompt
//! embeds the full category definitions alongside the Pass 1 results so that
//! per-file classifications are coherent with the global view.
//!
//! # Example
//! ```rust,no_run
//! use easy_diff::analysis::pass2_files::build_pass2_prompt;
//! use easy_diff::llm::schema::{Pass1Output, FileCluster};
//!
//! let pass1 = Pass1Output {
//!     summary: "Added auth layer.".into(),
//!     change_types: vec![],
//!     attention_tags: vec![],
//!     file_clusters: vec![],
//! };
//! let prompt = build_pass2_prompt("src/auth.rs", "--- a/src/auth.rs\n...", &pass1);
//! assert!(prompt.contains("src/auth.rs"));
//! ```

use crate::analysis::pass1_summary::build_category_definitions;
use crate::llm::schema::Pass1Output;

/// Builds the Pass 2 prompt for a single file.
///
/// Includes the Pass 1 summary as context so the per-file analysis is coherent
/// with the global view. If `file_path` belongs to a cluster identified in
/// `pass1_summary`, that cluster is also included.
#[allow(dead_code)]
pub fn build_pass2_prompt(file_path: &str, file_diff: &str, pass1_summary: &Pass1Output) -> String {
    let mut prompt = String::new();

    // 1. System instruction
    prompt.push_str(
        "You are a code review assistant analyzing a single file from a pull request.\n\
         Use the global PR context below to ensure your per-file classification is \
         consistent with the overall analysis.\n\n",
    );

    // 2. Pass 1 context
    prompt.push_str("## Global PR Context (Pass 1 Summary)\n\n");
    prompt.push_str(&format!("**Summary:** {}\n\n", pass1_summary.summary));

    if !pass1_summary.change_types.is_empty() {
        let types: Vec<String> = pass1_summary
            .change_types
            .iter()
            .map(|ct| format!("`{ct}`"))
            .collect();
        prompt.push_str(&format!(
            "**PR-level change types:** {}\n\n",
            types.join(", ")
        ));
    }

    if !pass1_summary.attention_tags.is_empty() {
        let tags: Vec<String> = pass1_summary
            .attention_tags
            .iter()
            .map(|t| format!("`{t}`"))
            .collect();
        prompt.push_str(&format!(
            "**PR-level attention tags:** {}\n\n",
            tags.join(", ")
        ));
    }

    // Include the cluster this file belongs to, if any
    let cluster = pass1_summary
        .file_clusters
        .iter()
        .find(|c| c.files.iter().any(|f| f == file_path));

    if let Some(cluster) = cluster {
        prompt.push_str(&format!(
            "**File cluster:** {} — {}\n\
             **Other files in cluster:** {}\n\n",
            cluster.label,
            cluster.rationale,
            cluster
                .files
                .iter()
                .filter(|f| f.as_str() != file_path)
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    // 3. Category definitions
    prompt.push_str(&build_category_definitions());

    // 4. Output format instruction
    prompt.push_str(
        "## Output Format\n\n\
         Respond with a single JSON object (no markdown fences) matching this schema:\n\n\
         {\n\
           \"summary\": \"<one-paragraph overview of what changed in this file and why>\",\n\
           \"change_types\": [\"<change-type>\", ...],\n\
           \"attention_tags\": [\"<attention-tag>\", ...],\n\
           \"details\": [\"<notable observation, edge case, risk, or suggestion>\", ...]\n\
         }\n\n\
         Rules:\n\
         - Use only the change type and attention tag identifiers defined above.\n\
         - `change_types` must contain at least one entry.\n\
         - `attention_tags` must contain at least one entry. Use descriptive tags like `straightforward`, `well-tested`, or `clean-up` when no warning-level tags apply.\n\
         - `details` should list concrete, actionable observations about this file.\n\n",
    );

    // 5. File path and diff
    prompt.push_str(&format!("## File: {file_path}\n\n"));
    prompt.push_str(file_diff);
    prompt.push('\n');

    prompt
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::categories::{AttentionTag, ChangeType};
    use crate::llm::schema::{FileCluster, Pass1Output};

    fn sample_pass1() -> Pass1Output {
        Pass1Output {
            summary: "Introduced authentication middleware across the request pipeline.".into(),
            change_types: vec![ChangeType::Feature, ChangeType::Security],
            attention_tags: vec![AttentionTag::Security],
            file_clusters: vec![FileCluster {
                label: "Auth layer".into(),
                files: vec!["src/auth.rs".into(), "src/middleware.rs".into()],
                rationale: "Both files implement the new authentication flow.".into(),
            }],
        }
    }

    #[test]
    fn pass2_prompt_contains_file_path() {
        // setup
        let pass1 = sample_pass1();

        // execute
        let prompt = build_pass2_prompt(
            "src/auth.rs",
            "--- a/src/auth.rs\n+++ b/src/auth.rs",
            &pass1,
        );

        // verify
        assert!(
            prompt.contains("src/auth.rs"),
            "prompt must contain the file path"
        );
    }

    #[test]
    fn pass2_prompt_contains_file_diff() {
        // setup
        let pass1 = sample_pass1();
        let diff = "--- a/src/auth.rs\n+++ b/src/auth.rs\n@@ -1 +1 @@\n-old\n+new";

        // execute
        let prompt = build_pass2_prompt("src/auth.rs", diff, &pass1);

        // verify
        assert!(prompt.contains(diff), "prompt must embed the file diff");
    }

    #[test]
    fn pass2_prompt_contains_pass1_summary() {
        // setup
        let pass1 = sample_pass1();

        // execute
        let prompt = build_pass2_prompt("src/auth.rs", "", &pass1);

        // verify
        assert!(
            prompt.contains(&pass1.summary),
            "prompt must include the Pass 1 summary text"
        );
    }

    #[test]
    fn pass2_prompt_contains_category_definitions() {
        // setup
        let pass1 = sample_pass1();

        // execute
        let prompt = build_pass2_prompt("src/auth.rs", "", &pass1);

        // verify — spot-check a few change types and tags
        for ct in ChangeType::all() {
            assert!(
                prompt.contains(&format!("`{ct}`")),
                "expected change type `{ct}` in prompt"
            );
        }
        for tag in AttentionTag::all() {
            assert!(
                prompt.contains(&format!("`{tag}`")),
                "expected attention tag `{tag}` in prompt"
            );
        }
    }

    #[test]
    fn pass2_prompt_includes_cluster_context() {
        // setup
        let pass1 = sample_pass1();

        // execute
        let prompt = build_pass2_prompt("src/auth.rs", "", &pass1);

        // verify — cluster label and sibling file present
        assert!(
            prompt.contains("Auth layer"),
            "prompt should include the cluster label"
        );
        assert!(
            prompt.contains("src/middleware.rs"),
            "prompt should name sibling files in the cluster"
        );
    }

    #[test]
    fn pass2_prompt_no_cluster_when_file_not_in_any() {
        // setup
        let pass1 = sample_pass1();

        // execute — use a file not in any cluster
        let prompt = build_pass2_prompt("src/unrelated.rs", "", &pass1);

        // verify — cluster section absent
        assert!(
            !prompt.contains("Auth layer"),
            "cluster block should not appear when file is not in a cluster"
        );
    }
}
