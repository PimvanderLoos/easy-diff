//! Debug file dumping for failed LLM responses.
//!
//! When an LLM response fails to parse or validate, the raw output is written
//! to `.easy-diff/debug/` in the repository root for post-mortem analysis.

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// Writes a failed LLM response to a debug file.
///
/// Creates `.easy-diff/debug/<label>_<epoch_seconds>.txt` containing `content`.
/// Returns the path written to, or `None` if writing failed (logged as a warning).
pub fn dump_failed_response(repo_root: &Path, label: &str, content: &str) -> Option<String> {
    let epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let dir = repo_root.join(".easy-diff").join("debug");
    if let Err(e) = std::fs::create_dir_all(&dir) {
        tracing::warn!(error = %e, "failed to create debug directory");
        return None;
    }

    let filename = format!("{label}_{epoch}.txt");
    let path = dir.join(&filename);

    match std::fs::write(&path, content) {
        Ok(()) => {
            let path_str = path.display().to_string();
            tracing::info!(path = %path_str, "dumped failed LLM response for debugging");
            Some(path_str)
        }
        Err(e) => {
            tracing::warn!(error = %e, "failed to write debug dump");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dump_creates_file_in_debug_dir() {
        // setup
        let dir = tempfile::tempdir().unwrap();
        let content = "raw LLM output that failed to parse";

        // execute
        let result = dump_failed_response(dir.path(), "pass1", content);

        // verify
        assert!(result.is_some());
        let path_str = result.unwrap();
        assert!(path_str.contains("pass1_"));
        assert!(path_str.contains(".txt"));
        let written = std::fs::read_to_string(&path_str).unwrap();
        assert_eq!(written, content);
    }

    #[test]
    fn dump_label_appears_in_filename() {
        // setup
        let dir = tempfile::tempdir().unwrap();

        // execute
        let result = dump_failed_response(dir.path(), "pass2_src_main", "{}");

        // verify
        assert!(result.unwrap().contains("pass2_src_main_"));
    }
}
