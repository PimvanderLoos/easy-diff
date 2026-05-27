//! Change Type and Attention Tag category system: built-in definitions,
//! types, and the logic for matching files to categories.
//!
//! This module provides the two core enums used throughout the analysis pipeline:
//! - [`ChangeType`]: classifies what kind of change a file or PR represents.
//! - [`AttentionTag`]: flags areas that require special reviewer attention.
//!
//! Both enums serialize to/from kebab-case strings (e.g. `"bug-fix"`, `"breaking-change"`)
//! and implement [`std::fmt::Display`] returning the same representation, making them
//! safe to embed in LLM prompts and JSON schemas alike.
//!
//! # Example
//! ```rust
//! use easy_diff::categories::{ChangeType, AttentionTag};
//!
//! assert_eq!(format!("{}", ChangeType::BugFix), "bug-fix");
//! assert_eq!(ChangeType::all().len(), 10);
//! assert_eq!(AttentionTag::all().len(), 10);
//! ```

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Classification of the type of change a file (or PR) represents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ChangeType {
    /// A new user-visible feature.
    Feature,
    /// A bug fix.
    BugFix,
    /// Internal restructuring without behaviour change.
    Refactor,
    /// Addition or modification of tests.
    Test,
    /// Documentation-only changes.
    Docs,
    /// Code style / formatting changes.
    Style,
    /// Maintenance tasks (build scripts, CI, tooling).
    Chore,
    /// Changes that measurably improve performance.
    Performance,
    /// Security-hardening changes.
    Security,
    /// Dependency additions, removals, or upgrades.
    Dependency,
}

impl ChangeType {
    /// Returns a short description (< 15 words) suitable for embedding in LLM prompts.
    #[allow(dead_code)]
    pub fn description(&self) -> &'static str {
        match self {
            Self::Feature => "A new user-visible capability or behaviour.",
            Self::BugFix => "Corrects an unintended or erroneous behaviour.",
            Self::Refactor => "Internal restructuring without observable behaviour change.",
            Self::Test => "Adds or modifies automated tests.",
            Self::Docs => "Documentation-only changes, no code logic altered.",
            Self::Style => "Code style or formatting changes only.",
            Self::Chore => "Maintenance: build scripts, CI, tooling.",
            Self::Performance => "Measurably improves runtime or memory performance.",
            Self::Security => "Hardens the code against security threats.",
            Self::Dependency => "Adds, removes, or upgrades a dependency.",
        }
    }

    /// Returns all variants in declaration order. Useful for prompt construction
    /// and exhaustive filtering.
    #[allow(dead_code)]
    pub fn all() -> &'static [Self] {
        &[
            Self::Feature,
            Self::BugFix,
            Self::Refactor,
            Self::Test,
            Self::Docs,
            Self::Style,
            Self::Chore,
            Self::Performance,
            Self::Security,
            Self::Dependency,
        ]
    }
}

impl fmt::Display for ChangeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Delegate to serde's kebab-case representation so Display always matches
        // the serialised form.
        let s = serde_json::to_value(self)
            .ok()
            .and_then(|v| v.as_str().map(str::to_owned))
            .unwrap_or_else(|| format!("{self:?}").to_lowercase());
        f.write_str(&s)
    }
}

/// Tags that characterise changes for reviewer attention and filtering.
///
/// Every file and PR-level summary must have at least one tag. Tags range from
/// high-severity flags (`security`, `breaking-change`) to routine descriptors
/// (`straightforward`, `well-tested`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum AttentionTag {
    /// Change has security implications.
    Security,
    /// Change breaks backward compatibility.
    BreakingChange,
    /// Change is not yet covered by tests.
    NeedsTest,
    /// Change introduces notable complexity.
    Complexity,
    /// Change is unrelated to the PR's stated purpose.
    OffTopic,
    /// Minor style or naming issue — low priority.
    Nitpick,
    /// Change embeds a significant design decision worth discussing.
    DesignDecision,
    /// Change is straightforward and low-risk.
    Straightforward,
    /// Change is accompanied by good test coverage.
    WellTested,
    /// Change improves code clarity or maintainability.
    CleanUp,
}

impl AttentionTag {
    /// Returns a short description (< 15 words) suitable for embedding in LLM prompts.
    #[allow(dead_code)]
    pub fn description(&self) -> &'static str {
        match self {
            Self::Security => "Has security implications; requires careful review.",
            Self::BreakingChange => "Breaks backward compatibility for callers or consumers.",
            Self::NeedsTest => "The change is not yet covered by automated tests.",
            Self::Complexity => "Introduces notable algorithmic or structural complexity.",
            Self::OffTopic => "Unrelated to the PR's stated purpose.",
            Self::Nitpick => "Minor style or naming issue; low priority.",
            Self::DesignDecision => "Embeds a significant design choice worth discussing.",
            Self::Straightforward => "Simple, low-risk change that needs minimal scrutiny.",
            Self::WellTested => "Accompanied by thorough automated test coverage.",
            Self::CleanUp => "Improves code clarity, naming, or structure.",
        }
    }

    /// Returns all variants in declaration order. Useful for prompt construction
    /// and exhaustive filtering.
    #[allow(dead_code)]
    pub fn all() -> &'static [Self] {
        &[
            Self::Security,
            Self::BreakingChange,
            Self::NeedsTest,
            Self::Complexity,
            Self::OffTopic,
            Self::Nitpick,
            Self::DesignDecision,
            Self::Straightforward,
            Self::WellTested,
            Self::CleanUp,
        ]
    }
}

impl fmt::Display for AttentionTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = serde_json::to_value(self)
            .ok()
            .and_then(|v| v.as_str().map(str::to_owned))
            .unwrap_or_else(|| format!("{self:?}").to_lowercase());
        f.write_str(&s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn change_type_serializes_to_kebab_case() {
        // setup + execute
        let json = serde_json::to_string(&ChangeType::BugFix).expect("serialize");

        // verify
        assert_eq!(json, r#""bug-fix""#);
    }

    #[test]
    fn attention_tag_serializes_to_kebab_case() {
        // setup + execute
        let json = serde_json::to_string(&AttentionTag::BreakingChange).expect("serialize");

        // verify
        assert_eq!(json, r#""breaking-change""#);
    }

    #[test]
    fn change_type_display_matches_serde() {
        // setup + execute + verify
        assert_eq!(format!("{}", ChangeType::BugFix), "bug-fix");
        assert_eq!(format!("{}", ChangeType::Feature), "feature");
        assert_eq!(format!("{}", ChangeType::Performance), "performance");
        assert_eq!(format!("{}", ChangeType::Dependency), "dependency");
    }

    #[test]
    fn attention_tag_display_matches_serde() {
        // setup + execute + verify
        assert_eq!(
            format!("{}", AttentionTag::BreakingChange),
            "breaking-change"
        );
        assert_eq!(format!("{}", AttentionTag::Security), "security");
        assert_eq!(
            format!("{}", AttentionTag::DesignDecision),
            "design-decision"
        );
    }

    #[test]
    fn change_type_round_trips() {
        // setup + execute + verify
        for variant in ChangeType::all() {
            let json = serde_json::to_value(variant).expect("serialize");
            let decoded: ChangeType = serde_json::from_value(json).expect("deserialize");
            assert_eq!(&decoded, variant);
        }
    }

    #[test]
    fn attention_tag_round_trips() {
        // setup + execute + verify
        for variant in AttentionTag::all() {
            let json = serde_json::to_value(variant).expect("serialize");
            let decoded: AttentionTag = serde_json::from_value(json).expect("deserialize");
            assert_eq!(&decoded, variant);
        }
    }

    #[test]
    fn all_change_types_returns_all_variants() {
        // setup + execute
        let all = ChangeType::all();

        // verify — update this count when adding new variants
        assert_eq!(all.len(), 10);
    }

    #[test]
    fn all_attention_tags_returns_all_variants() {
        // setup + execute
        let all = AttentionTag::all();

        // verify — update this count when adding new variants
        assert_eq!(all.len(), 10);
    }
}
