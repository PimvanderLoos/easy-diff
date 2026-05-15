//! Pass 1: global PR summary and file-cluster identification.
//!
//! This module builds the LLM prompt for Pass 1, which analyses the entire pull
//! request and produces a structured global summary. The prompt embeds all
//! [`ChangeType`] and [`AttentionTag`] category definitions so the LLM can use
//! the correct vocabulary in its response.
//!
//! # Example
//! ```rust
//! use easy_diff::analysis::pass1_summary::build_pass1_prompt;
//!
//! let prompt = build_pass1_prompt("--- a/foo.rs\n+++ b/foo.rs", &["foo.rs"], false);
//! assert!(prompt.contains("foo.rs"));
//! ```

use crate::categories::{AttentionTag, ChangeType};

/// Builds the Pass 1 prompt for global PR analysis.
///
/// In normal mode, `large_pr = false`, the full unified `diff` is appended so
/// the LLM can inspect every changed line. In large-PR mode, `large_pr = true`,
/// only `file_names` are included to stay within context limits; the LLM is
/// asked to classify based on file-path patterns alone.
#[allow(dead_code)]
pub fn build_pass1_prompt(diff: &str, file_names: &[&str], large_pr: bool) -> String {
    let mut prompt = String::new();

    // 1. System instruction
    prompt.push_str(
        "You are a code review assistant analyzing a pull request.\n\
         Your task is to produce a structured global summary of all changes.\n\n",
    );

    // 2. Category definitions
    prompt.push_str(&build_category_definitions());

    // 3. Output format instruction
    prompt.push_str(
        "## Output Format\n\n\
         Respond with a single JSON object (no markdown fences) matching this schema:\n\n\
         {\n\
           \"summary\": \"<one-paragraph overview of what this PR changes and why>\",\n\
           \"change_types\": [\"<change-type>\", ...],\n\
           \"attention_tags\": [\"<attention-tag>\", ...],\n\
           \"file_clusters\": [\n\
             {\n\
               \"label\": \"<short cluster name, e.g. 'Auth middleware'>\",\n\
               \"files\": [\"<file path>\", ...],\n\
               \"rationale\": \"<one sentence explaining why these files are grouped>\"\n\
             }\n\
           ]\n\
         }\n\n\
         Rules:\n\
         - Use only the change type and attention tag identifiers defined above.\n\
         - `change_types` and `attention_tags` may be empty arrays if none apply.\n\
         - Group files into clusters by logical concern, not directory structure.\n\
         - Every changed file must appear in exactly one cluster.\n\n",
    );

    // 4. Diff content or file list
    if large_pr {
        prompt.push_str("## Changed Files\n\n");
        prompt.push_str(
            "This is a large PR; only file names are provided. \
             Classify based on file-path patterns and names.\n\n",
        );
        for name in file_names {
            prompt.push_str(name);
            prompt.push('\n');
        }
    } else {
        prompt.push_str("## Diff\n\n");
        prompt.push_str(diff);
        prompt.push('\n');
    }

    prompt
}

/// Builds the shared category-definitions block used in both Pass 1 and Pass 2 prompts.
#[allow(dead_code)]
pub(crate) fn build_category_definitions() -> String {
    let mut out = String::new();

    out.push_str("## Change Types\n\n");
    out.push_str(
        "Classify each file (and the PR as a whole) using one or more of the \
         following change type identifiers:\n\n",
    );
    for ct in ChangeType::all() {
        out.push_str(&format!("- `{ct}`: {}\n", ct.description()));
    }
    out.push('\n');

    out.push_str("## Attention Tags\n\n");
    out.push_str(
        "Flag areas that require special reviewer attention using one or more of \
         the following tag identifiers:\n\n",
    );
    for tag in AttentionTag::all() {
        out.push_str(&format!("- `{tag}`: {}\n", tag.description()));
    }
    out.push('\n');

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pass1_prompt_contains_diff() {
        // setup
        let diff = "--- a/src/main.rs\n+++ b/src/main.rs\n@@ -1 +1 @@\n-fn old() {}\n+fn new() {}";

        // execute
        let prompt = build_pass1_prompt(diff, &[], false);

        // verify
        assert!(
            prompt.contains(diff),
            "normal mode should embed the full diff"
        );
    }

    #[test]
    fn pass1_prompt_large_pr_uses_file_names() {
        // setup
        let diff = "--- a/src/main.rs\n+++ b/src/main.rs";
        let files = &["src/main.rs", "src/lib.rs"];

        // execute
        let prompt = build_pass1_prompt(diff, files, true);

        // verify — file names present, diff body absent
        assert!(
            prompt.contains("src/main.rs"),
            "large-PR mode should include file names"
        );
        assert!(
            prompt.contains("src/lib.rs"),
            "large-PR mode should include all file names"
        );
        assert!(
            !prompt.contains("@@ -1"),
            "large-PR mode must not include the diff body"
        );
    }

    #[test]
    fn pass1_prompt_contains_all_change_types() {
        // setup + execute
        let prompt = build_pass1_prompt("", &[], false);

        // verify — every ChangeType serialized name must appear
        for ct in ChangeType::all() {
            assert!(
                prompt.contains(&format!("`{ct}`")),
                "expected change type `{ct}` in prompt"
            );
        }
    }

    #[test]
    fn pass1_prompt_contains_all_attention_tags() {
        // setup + execute
        let prompt = build_pass1_prompt("", &[], false);

        // verify — every AttentionTag serialized name must appear
        for tag in AttentionTag::all() {
            assert!(
                prompt.contains(&format!("`{tag}`")),
                "expected attention tag `{tag}` in prompt"
            );
        }
    }
}
