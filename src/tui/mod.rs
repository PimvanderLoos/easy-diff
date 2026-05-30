//! Terminal UI: `dialoguer`-based PR selection and category filter prompts
//! (CLI MVP), with a future `ratatui` full-TUI path.
//!
//! All interactive functions require a real TTY. When stdin is not a TTY the
//! functions return an error so callers can fall back to non-interactive mode.
//!
//! # Example
//! ```no_run
//! use easy_diff::tui;
//! use easy_diff::platform::PullRequest;
//!
//! // let prs: Vec<PullRequest> = ...;
//! // let idx = tui::select_pr(&prs)?;
//! // let pr = &prs[idx];
//! ```

use anyhow::{bail, Result};
use dialoguer::{Confirm, MultiSelect, Select};

use crate::categories::{AttentionTag, ChangeType};
use crate::llm::schema::Pass1Output;
use crate::platform::PullRequest;

/// Presents an interactive list of open PRs and returns the index of the
/// selected one within `prs`.
///
/// Each row is formatted as `#{number} {title} ({author})`.
///
/// Returns an error when stdin is not a TTY (non-interactive mode) or if the
/// prompt is cancelled.
pub fn select_pr(prs: &[PullRequest]) -> Result<usize> {
    if prs.is_empty() {
        bail!("no open pull requests to select from");
    }

    let items: Vec<String> = prs.iter()
        .map(|pr| format!("#{} {} ({})", pr.number, pr.title, pr.author))
        .collect();

    let selection = Select::new()
        .with_prompt("Select a pull request")
        .items(&items)
        .default(0)
        .interact()
        .map_err(|e| anyhow::anyhow!("PR selection cancelled or failed: {e}"))?;

    Ok(selection)
}

/// Prints the Pass 1 analysis summary to stdout.
///
/// Output sections:
/// 1. A prose summary paragraph.
/// 2. Bullet list of detected change types.
/// 3. Bullet list of attention tags (omitted when empty).
/// 4. File clusters with their member files and rationale.
pub fn display_summary(result: &Pass1Output) {
    println!();
    println!("=== PR Summary ===");
    println!("{}", result.summary);

    println!();
    println!("Change types:");
    for ct in &result.change_types {
        println!("  • {ct}");
    }

    if !result.attention_tags.is_empty() {
        println!();
        println!("Attention tags:");
        for tag in &result.attention_tags {
            println!("  • {tag}");
        }
    }

    if !result.file_clusters.is_empty() {
        println!();
        println!("File clusters:");
        for cluster in &result.file_clusters {
            println!("  [{}]  {}", cluster.label, cluster.rationale);
            for file in &cluster.files {
                println!("    - {file}");
            }
        }
    }

    println!();
}

/// Presents a confirm prompt asking whether to show only unreviewed files.
///
/// Only shown when the caller has viewed-state data (i.e. `has_viewed_data` is `true`).
/// When `has_viewed_data` is `false`, returns `false` without prompting.
///
/// Returns an error when stdin is not a TTY or the prompt fails.
pub fn select_unreviewed_filter(has_viewed_data: bool) -> Result<bool> {
    if !has_viewed_data {
        return Ok(false);
    }

    let show_only_unreviewed = Confirm::new()
        .with_prompt("Show only files with changes since last review?")
        .default(false)
        .interact()
        .map_err(|e| anyhow::anyhow!("unreviewed filter prompt failed: {e}"))?;

    Ok(show_only_unreviewed)
}

/// Presents two `MultiSelect` prompts — one for [`ChangeType`] filters and one
/// for [`AttentionTag`] filters — with all items selected by default.
///
/// Returns `(selected_change_types, selected_attention_tags)`.
///
/// Returns an error when stdin is not a TTY or if a prompt is cancelled.
pub fn select_filters() -> Result<(Vec<ChangeType>, Vec<AttentionTag>)> {
    let change_types = ChangeType::all();
    let change_type_labels: Vec<String> = change_types.iter().map(|ct| ct.to_string()).collect();
    let ct_defaults = vec![true; change_types.len()];

    let ct_indices = MultiSelect::new()
        .with_prompt("Select Change Type filters (space to toggle, enter to confirm)")
        .items(&change_type_labels)
        .defaults(&ct_defaults)
        .interact()
        .map_err(|e| anyhow::anyhow!("change type filter selection cancelled or failed: {e}"))?;

    let selected_change_types: Vec<ChangeType> =
        ct_indices.into_iter().map(|i| change_types[i]).collect();

    let attention_tags = AttentionTag::all();
    let attention_tag_labels: Vec<String> =
        attention_tags.iter().map(|tag| tag.to_string()).collect();
    let tag_defaults = vec![true; attention_tags.len()];

    let tag_indices = MultiSelect::new()
        .with_prompt("Select Attention Tag filters (space to toggle, enter to confirm)")
        .items(&attention_tag_labels)
        .defaults(&tag_defaults)
        .interact()
        .map_err(|e| anyhow::anyhow!("attention tag filter selection cancelled or failed: {e}"))?;

    let selected_attention_tags: Vec<AttentionTag> =
        tag_indices.into_iter().map(|i| attention_tags[i]).collect();

    Ok((selected_change_types, selected_attention_tags))
}

/// Shows the estimated token count and asks for confirmation when the diff is
/// large (i.e. `estimated_tokens > threshold_lines`).
///
/// Returns `true` when the user confirms or when the PR is under the threshold
/// (no prompt shown). Returns `false` when the user declines.
///
/// Returns an error when stdin is not a TTY or the prompt fails.
pub fn confirm_large_pr(estimated_tokens: usize, threshold_lines: u32) -> Result<bool> {
    if estimated_tokens <= threshold_lines as usize {
        return Ok(true);
    }

    println!("\nThis PR has an estimated ~{estimated_tokens} tokens in its diff.");

    let confirmed = Confirm::new()
        .with_prompt("This is a large PR. Continue with analysis?")
        .default(true)
        .interact()
        .map_err(|e| anyhow::anyhow!("confirmation prompt failed: {e}"))?;

    Ok(confirmed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_change_types_in_defaults() {
        // setup
        let all = ChangeType::all();
        let defaults = vec![true; all.len()];

        // execute — simulate "all selected"
        let selected: Vec<ChangeType> = defaults
            .iter()
            .enumerate()
            .filter(|(_, &v)| v)
            .map(|(i, _)| all[i])
            .collect();

        // verify
        assert_eq!(
            selected.len(),
            all.len(),
            "all change types should be in defaults"
        );
        for variant in all {
            assert!(
                selected.contains(variant),
                "variant {variant} missing from default selection"
            );
        }
    }

    #[test]
    fn all_attention_tags_in_defaults() {
        // setup
        let all = AttentionTag::all();
        let defaults = vec![true; all.len()];

        // execute — simulate "all selected"
        let selected: Vec<AttentionTag> = defaults
            .iter()
            .enumerate()
            .filter(|(_, &v)| v)
            .map(|(i, _)| all[i])
            .collect();

        // verify
        assert_eq!(
            selected.len(),
            all.len(),
            "all attention tags should be in defaults"
        );
        for variant in all {
            assert!(
                selected.contains(variant),
                "variant {variant} missing from default selection"
            );
        }
    }

    #[test]
    fn confirm_large_pr_returns_true_when_under_threshold() {
        // setup
        let estimated_tokens = 500;
        let threshold = 1000_u32;

        // execute — below threshold: should return Ok(true) without prompting
        let result = confirm_large_pr(estimated_tokens, threshold);

        // verify
        assert!(result.is_ok());
        assert!(result.unwrap(), "should return true when under threshold");
    }

    #[test]
    fn display_summary_runs_without_panic() {
        // setup
        use crate::llm::schema::FileCluster;

        let result = Pass1Output {
            summary: "Test summary.".into(),
            change_types: vec![ChangeType::Feature, ChangeType::BugFix],
            attention_tags: vec![AttentionTag::Security],
            file_clusters: vec![FileCluster {
                label: "Core".into(),
                files: vec!["src/lib.rs".into()],
                rationale: "Core module changes.".into(),
            }],
        };

        // execute + verify — should not panic
        display_summary(&result);
    }
}
