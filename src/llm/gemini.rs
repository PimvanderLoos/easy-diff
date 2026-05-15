//! Gemini CLI backend for the `LlmProvider` trait.
//!
//! Full implementation (retry logic, `LlmProvider` impl) is added in PR-2.

/// Gemini CLI backend stub. Holds config needed for the PR-2 implementation.
pub struct GeminiProvider {
    /// Model override (e.g. `"gemini-2.0-flash"`). `None` uses the CLI default.
    #[allow(dead_code)]
    model: Option<String>,
    /// Maximum total attempts (including the first). Default: 3.
    #[allow(dead_code)]
    max_retries: u32,
}

impl GeminiProvider {
    /// Creates a new provider.
    ///
    /// `max_retries` must be at least 1. If 0 is passed it is silently clamped
    /// to 1 so the first attempt is always made.
    pub fn new(model: Option<String>, max_retries: u32) -> Self {
        Self {
            model,
            max_retries: max_retries.max(1),
        }
    }
}
