//! JSON schema definitions for LLM structured output (Pass 1 and Pass 2 responses).
//!
//! [`Pass1Output`] and [`Pass2Output`] are the types the LLM must return during the
//! two-pass analysis pipeline. Use [`schema_for_pass1`] and [`schema_for_pass2`] to
//! obtain the corresponding JSON Schema values to pass to [`crate::llm::LlmProvider::analyze`].
//!
//! # Example
//! ```rust
//! use easy_diff::llm::schema::{schema_for_pass1, schema_for_pass2};
//!
//! let pass1_schema = schema_for_pass1();
//! let pass2_schema = schema_for_pass2();
//! assert!(pass1_schema.is_object());
//! assert!(pass2_schema.is_object());
//! ```

use crate::categories::{AttentionTag, ChangeType};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Global (Pass 1) analysis output for an entire pull request.
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Pass1Output {
    /// One-paragraph summary of the overall change.
    pub summary: String,
    /// Change types present across the whole PR.
    pub change_types: Vec<ChangeType>,
    /// Attention tags raised at the PR level.
    pub attention_tags: Vec<AttentionTag>,
    /// Logical groupings of related files.
    pub file_clusters: Vec<FileCluster>,
}

/// A named cluster of related files identified during Pass 1.
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FileCluster {
    /// Short label for the cluster (e.g. "Auth middleware").
    pub label: String,
    /// File paths belonging to this cluster.
    pub files: Vec<String>,
    /// One sentence explaining why these files are related.
    pub rationale: String,
}

/// Per-file (Pass 2) analysis output.
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Pass2Output {
    /// One-paragraph summary of changes in this file.
    pub summary: String,
    /// Change types for this file specifically.
    pub change_types: Vec<ChangeType>,
    /// Attention tags raised for this file.
    pub attention_tags: Vec<AttentionTag>,
    /// Notable observations (e.g. edge cases, risks, suggestions).
    pub details: Vec<String>,
}

/// Returns the JSON Schema for [`Pass1Output`] as a `serde_json::Value`,
/// suitable for passing directly to [`crate::llm::LlmProvider::analyze`].
#[allow(dead_code)]
pub fn schema_for_pass1() -> serde_json::Value {
    let schema = schemars::schema_for!(Pass1Output);
    serde_json::to_value(schema).expect("Pass1Output schema is always serializable")
}

/// Returns the JSON Schema for [`Pass2Output`] as a `serde_json::Value`,
/// suitable for passing directly to [`crate::llm::LlmProvider::analyze`].
#[allow(dead_code)]
pub fn schema_for_pass2() -> serde_json::Value {
    let schema = schemars::schema_for!(Pass2Output);
    serde_json::to_value(schema).expect("Pass2Output schema is always serializable")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::categories::{AttentionTag, ChangeType};
    use serde_json::json;

    #[test]
    fn schema_for_pass1_is_valid_json() {
        // setup + execute
        let schema = schema_for_pass1();

        // verify — schemars always emits a "$schema" key and a "type" key
        assert!(schema.is_object(), "expected JSON object");
        assert!(
            schema.get("type").is_some() || schema.get("$schema").is_some(),
            "expected 'type' or '$schema' key, got: {schema}"
        );
    }

    #[test]
    fn schema_for_pass2_is_valid_json() {
        // setup + execute
        let schema = schema_for_pass2();

        // verify
        assert!(schema.is_object(), "expected JSON object");
        assert!(
            schema.get("type").is_some() || schema.get("$schema").is_some(),
            "expected 'type' or '$schema' key, got: {schema}"
        );
    }

    #[test]
    fn pass1_output_round_trips() {
        // setup
        let original = Pass1Output {
            summary: "Added auth middleware.".into(),
            change_types: vec![ChangeType::Feature],
            attention_tags: vec![AttentionTag::Security],
            file_clusters: vec![FileCluster {
                label: "Auth".into(),
                files: vec!["src/auth.rs".into()],
                rationale: "All authentication-related changes.".into(),
            }],
        };

        // execute
        let json = serde_json::to_value(&original).expect("serialize");
        let decoded: Pass1Output = serde_json::from_value(json).expect("deserialize");

        // verify
        assert_eq!(decoded.summary, original.summary);
        assert_eq!(decoded.change_types, original.change_types);
        assert_eq!(decoded.attention_tags, original.attention_tags);
        assert_eq!(decoded.file_clusters.len(), 1);
        assert_eq!(decoded.file_clusters[0].label, "Auth");
        assert_eq!(decoded.file_clusters[0].files, vec!["src/auth.rs"]);
    }

    #[test]
    fn pass2_output_round_trips() {
        // setup
        let original = Pass2Output {
            summary: "Refactored error handling.".into(),
            change_types: vec![ChangeType::Refactor],
            attention_tags: vec![],
            details: vec!["Removed unwrap() calls.".into()],
        };

        // execute
        let json = serde_json::to_value(&original).expect("serialize");
        let decoded: Pass2Output = serde_json::from_value(json).expect("deserialize");

        // verify
        assert_eq!(decoded.summary, original.summary);
        assert_eq!(decoded.change_types, original.change_types);
        assert!(decoded.attention_tags.is_empty());
        assert_eq!(decoded.details, vec!["Removed unwrap() calls."]);
    }

    #[test]
    fn pass1_output_rejects_missing_required_field() {
        // setup — missing "summary"
        let json = json!({
            "change_types": [],
            "attention_tags": [],
            "file_clusters": []
        });

        // execute
        let result = serde_json::from_value::<Pass1Output>(json);

        // verify
        assert!(result.is_err(), "expected error for missing required field");
    }
}
