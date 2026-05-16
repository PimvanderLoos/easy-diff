//! SQLite-backed caching layer for LLM analysis results, keyed by PR state,
//! file path, provider, and schema version.
#![allow(dead_code)]
//!
//! [`CacheStore`] stores and retrieves [`Pass1Output`] and [`Pass2Output`] values.
//! The cache is keyed by a [`CacheKey`] (PR id, base/head SHAs, and provider name).
//! Pass 2 results are additionally keyed by file path.
//!
//! Schema version is embedded in every row; bumping [`SCHEMA_VERSION`] automatically
//! causes all old entries to be treated as cache misses without requiring a migration.
//!
//! # Example
//! ```rust,no_run
//! use std::path::Path;
//! use easy_diff::cache::{CacheKey, CacheStore};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let store = CacheStore::open(Path::new(".easy-diff/cache/analysis.db"))?;
//! let key = CacheKey {
//!     pr_id: "42".into(),
//!     base_sha: "abc".into(),
//!     head_sha: "def".into(),
//!     provider: "claude".into(),
//! };
//! let cached = store.get_pass1(&key)?;
//! # Ok(())
//! # }
//! ```

use std::path::Path;

use rusqlite::{params, Connection};
use thiserror::Error;

use crate::llm::schema::{Pass1Output, Pass2Output};

/// Schema version embedded in every cached row. Bump to invalidate all existing entries.
const SCHEMA_VERSION: i64 = 1;

/// Identifies a unique analysis context (PR + provider combination).
#[derive(Debug, Clone)]
pub struct CacheKey {
    /// Platform-specific PR identifier (e.g. GitHub PR number as string).
    pub pr_id: String,
    /// Git SHA of the base (target) branch tip.
    pub base_sha: String,
    /// Git SHA of the head (source) branch tip.
    pub head_sha: String,
    /// LLM provider name used for analysis (e.g. `"claude"`, `"gemini"`).
    pub provider: String,
}

/// Errors returned by [`CacheStore`] operations.
#[derive(Debug, Error)]
pub enum CacheError {
    /// Underlying SQLite error.
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    /// JSON serialization/deserialization failure.
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    /// I/O error while creating directories or opening the database file.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// SQLite-backed store for Pass 1 and Pass 2 analysis results.
pub struct CacheStore {
    conn: Connection,
}

impl CacheStore {
    /// Opens (or creates) the SQLite database at `path`, creating parent directories as needed.
    ///
    /// Runs `CREATE TABLE IF NOT EXISTS` for both cache tables on every open so the
    /// schema is always present without requiring explicit migration steps.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, CacheError> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        let store = Self { conn };
        store.init_schema()?;
        Ok(store)
    }

    /// Opens an in-memory SQLite database. Intended for tests only.
    #[cfg(test)]
    pub fn open_in_memory() -> Result<Self, CacheError> {
        let conn = Connection::open_in_memory()?;
        let store = Self { conn };
        store.init_schema()?;
        Ok(store)
    }

    /// Creates the cache tables if they do not already exist.
    fn init_schema(&self) -> Result<(), CacheError> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS pass1_cache (
                pr_id          TEXT    NOT NULL,
                base_sha       TEXT    NOT NULL,
                head_sha       TEXT    NOT NULL,
                provider       TEXT    NOT NULL,
                schema_version INTEGER NOT NULL,
                result_json    TEXT    NOT NULL,
                created_at     TEXT    NOT NULL,
                PRIMARY KEY (pr_id, base_sha, head_sha, provider, schema_version)
            );
            CREATE TABLE IF NOT EXISTS pass2_cache (
                pr_id          TEXT    NOT NULL,
                base_sha       TEXT    NOT NULL,
                head_sha       TEXT    NOT NULL,
                file_path      TEXT    NOT NULL,
                provider       TEXT    NOT NULL,
                schema_version INTEGER NOT NULL,
                result_json    TEXT    NOT NULL,
                created_at     TEXT    NOT NULL,
                PRIMARY KEY (pr_id, base_sha, head_sha, file_path, provider, schema_version)
            );
            CREATE TABLE IF NOT EXISTS viewed_files (
                pr_id     TEXT NOT NULL,
                file_path TEXT NOT NULL,
                head_sha  TEXT NOT NULL,
                viewed_at TEXT NOT NULL,
                PRIMARY KEY (pr_id, file_path)
            );",
        )?;
        Ok(())
    }

    /// Records that `file_path` in PR `pr_id` was viewed at `head_sha`.
    ///
    /// Uses `INSERT OR REPLACE` so calling this twice for the same file
    /// updates the stored SHA to the most recent value.
    pub fn mark_viewed(
        &self,
        pr_id: &str,
        file_path: &str,
        head_sha: &str,
    ) -> Result<(), CacheError> {
        let now = chrono_now();
        self.conn.execute(
            "INSERT OR REPLACE INTO viewed_files (pr_id, file_path, head_sha, viewed_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![pr_id, file_path, head_sha, now],
        )?;
        Ok(())
    }

    /// Returns the head SHA at which `file_path` in PR `pr_id` was last viewed,
    /// or `None` if the file has never been marked as viewed.
    pub fn get_viewed_sha(
        &self,
        pr_id: &str,
        file_path: &str,
    ) -> Result<Option<String>, CacheError> {
        let mut stmt = self
            .conn
            .prepare("SELECT head_sha FROM viewed_files WHERE pr_id = ?1 AND file_path = ?2")?;
        let mut rows = stmt.query(params![pr_id, file_path])?;
        match rows.next()? {
            None => Ok(None),
            Some(row) => {
                let sha: String = row.get(0)?;
                Ok(Some(sha))
            }
        }
    }

    /// Returns all `(file_path, head_sha)` pairs for files that have been
    /// marked as viewed in PR `pr_id`.
    pub fn list_viewed(&self, pr_id: &str) -> Result<Vec<(String, String)>, CacheError> {
        let mut stmt = self.conn.prepare(
            "SELECT file_path, head_sha FROM viewed_files WHERE pr_id = ?1 ORDER BY file_path",
        )?;
        let rows = stmt.query_map(params![pr_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(CacheError::from)
    }

    /// Stores a Pass 1 result, replacing any existing entry with the same key.
    pub fn store_pass1(&self, key: &CacheKey, result: &Pass1Output) -> Result<(), CacheError> {
        let json = serde_json::to_string(result)?;
        let now = chrono_now();
        self.conn.execute(
            "INSERT OR REPLACE INTO pass1_cache
                (pr_id, base_sha, head_sha, provider, schema_version, result_json, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                key.pr_id,
                key.base_sha,
                key.head_sha,
                key.provider,
                SCHEMA_VERSION,
                json,
                now,
            ],
        )?;
        Ok(())
    }

    /// Retrieves a Pass 1 result for the given key, or `None` on a cache miss or version mismatch.
    pub fn get_pass1(&self, key: &CacheKey) -> Result<Option<Pass1Output>, CacheError> {
        let mut stmt = self.conn.prepare(
            "SELECT result_json FROM pass1_cache
             WHERE pr_id = ?1 AND base_sha = ?2 AND head_sha = ?3
               AND provider = ?4 AND schema_version = ?5",
        )?;
        let mut rows = stmt.query(params![
            key.pr_id,
            key.base_sha,
            key.head_sha,
            key.provider,
            SCHEMA_VERSION,
        ])?;
        match rows.next()? {
            None => Ok(None),
            Some(row) => {
                let json: String = row.get(0)?;
                let output = serde_json::from_str(&json)?;
                Ok(Some(output))
            }
        }
    }

    /// Stores a Pass 2 result for a specific file, replacing any existing entry.
    pub fn store_pass2(
        &self,
        key: &CacheKey,
        file_path: &str,
        result: &Pass2Output,
    ) -> Result<(), CacheError> {
        let json = serde_json::to_string(result)?;
        let now = chrono_now();
        self.conn.execute(
            "INSERT OR REPLACE INTO pass2_cache
                (pr_id, base_sha, head_sha, file_path, provider, schema_version, result_json, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                key.pr_id,
                key.base_sha,
                key.head_sha,
                file_path,
                key.provider,
                SCHEMA_VERSION,
                json,
                now,
            ],
        )?;
        Ok(())
    }

    /// Retrieves a Pass 2 result for the given key and file path, or `None` on miss/version mismatch.
    pub fn get_pass2(
        &self,
        key: &CacheKey,
        file_path: &str,
    ) -> Result<Option<Pass2Output>, CacheError> {
        let mut stmt = self.conn.prepare(
            "SELECT result_json FROM pass2_cache
             WHERE pr_id = ?1 AND base_sha = ?2 AND head_sha = ?3
               AND file_path = ?4 AND provider = ?5 AND schema_version = ?6",
        )?;
        let mut rows = stmt.query(params![
            key.pr_id,
            key.base_sha,
            key.head_sha,
            file_path,
            key.provider,
            SCHEMA_VERSION,
        ])?;
        match rows.next()? {
            None => Ok(None),
            Some(row) => {
                let json: String = row.get(0)?;
                let output = serde_json::from_str(&json)?;
                Ok(Some(output))
            }
        }
    }

    /// Returns the most recent head SHA for which a Pass 1 entry exists for
    /// the given `(pr_id, base_sha, provider)` combination at the current schema version.
    ///
    /// Returns `None` if no previous analysis exists. The "most recent" entry is
    /// determined by insertion order (rowid descending), which corresponds to the
    /// last analysis run for this PR.
    pub fn get_latest_head_sha(
        &self,
        pr_id: &str,
        base_sha: &str,
        provider: &str,
    ) -> Result<Option<String>, CacheError> {
        let mut stmt = self.conn.prepare(
            "SELECT head_sha FROM pass1_cache
             WHERE pr_id = ?1 AND base_sha = ?2 AND provider = ?3
               AND schema_version = ?4
             ORDER BY rowid DESC
             LIMIT 1",
        )?;
        let mut rows = stmt.query(params![pr_id, base_sha, provider, SCHEMA_VERSION])?;
        match rows.next()? {
            None => Ok(None),
            Some(row) => {
                let sha: String = row.get(0)?;
                Ok(Some(sha))
            }
        }
    }

    /// Returns all file paths that have a cached Pass 2 result for the given key
    /// at the current schema version.
    pub fn get_cached_files(&self, key: &CacheKey) -> Result<Vec<String>, CacheError> {
        let mut stmt = self.conn.prepare(
            "SELECT file_path FROM pass2_cache
             WHERE pr_id = ?1 AND base_sha = ?2 AND head_sha = ?3
               AND provider = ?4 AND schema_version = ?5
             ORDER BY file_path",
        )?;
        let rows = stmt.query_map(
            params![
                key.pr_id,
                key.base_sha,
                key.head_sha,
                key.provider,
                SCHEMA_VERSION,
            ],
            |row| row.get(0),
        )?;
        rows.collect::<Result<Vec<String>, _>>()
            .map_err(CacheError::from)
    }
}

/// Returns the current UTC time as an ISO 8601 string (seconds precision).
fn chrono_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Format as a basic ISO 8601 UTC timestamp.
    let (y, mo, d, h, mi, s) = epoch_to_datetime(secs);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z")
}

/// Converts a UNIX epoch seconds value to (year, month, day, hour, min, sec).
/// This is a minimal implementation that avoids pulling in the `chrono` crate.
fn epoch_to_datetime(epoch: u64) -> (u64, u64, u64, u64, u64, u64) {
    let s = epoch % 60;
    let epoch = epoch / 60;
    let mi = epoch % 60;
    let epoch = epoch / 60;
    let h = epoch % 24;
    let mut days = epoch / 24;

    // Days since 1970-01-01
    let mut year = 1970u64;
    loop {
        let dy = if is_leap(year) { 366 } else { 365 };
        if days < dy {
            break;
        }
        days -= dy;
        year += 1;
    }
    let months = if is_leap(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut month = 1u64;
    for dm in months {
        if days < dm {
            break;
        }
        days -= dm;
        month += 1;
    }
    (year, month, days + 1, h, mi, s)
}

fn is_leap(y: u64) -> bool {
    (y.is_multiple_of(4) && !y.is_multiple_of(100)) || y.is_multiple_of(400)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::categories::{AttentionTag, ChangeType};
    use crate::llm::schema::FileCluster;

    fn sample_key() -> CacheKey {
        CacheKey {
            pr_id: "42".into(),
            base_sha: "aaaaaa".into(),
            head_sha: "bbbbbb".into(),
            provider: "claude".into(),
        }
    }

    fn sample_pass1() -> Pass1Output {
        Pass1Output {
            summary: "Added auth middleware.".into(),
            change_types: vec![ChangeType::Feature],
            attention_tags: vec![AttentionTag::Security],
            file_clusters: vec![FileCluster {
                label: "Auth".into(),
                files: vec!["src/auth.rs".into()],
                rationale: "All auth changes.".into(),
            }],
        }
    }

    fn sample_pass2() -> Pass2Output {
        Pass2Output {
            summary: "Refactored error handling.".into(),
            change_types: vec![ChangeType::Refactor],
            attention_tags: vec![],
            details: vec!["Removed unwrap() calls.".into()],
        }
    }

    #[test]
    fn store_and_retrieve_pass1() {
        // setup
        let store = CacheStore::open_in_memory().unwrap();
        let key = sample_key();
        let result = sample_pass1();

        // execute
        store.store_pass1(&key, &result).unwrap();
        let cached = store.get_pass1(&key).unwrap();

        // verify
        let cached = cached.expect("expected a cache hit");
        assert_eq!(cached.summary, result.summary);
        assert_eq!(cached.change_types, result.change_types);
        assert_eq!(cached.attention_tags, result.attention_tags);
        assert_eq!(cached.file_clusters.len(), 1);
        assert_eq!(cached.file_clusters[0].label, "Auth");
    }

    #[test]
    fn store_and_retrieve_pass2() {
        // setup
        let store = CacheStore::open_in_memory().unwrap();
        let key = sample_key();
        let result = sample_pass2();
        let file = "src/auth.rs";

        // execute
        store.store_pass2(&key, file, &result).unwrap();
        let cached = store.get_pass2(&key, file).unwrap();

        // verify
        let cached = cached.expect("expected a cache hit");
        assert_eq!(cached.summary, result.summary);
        assert_eq!(cached.change_types, result.change_types);
        assert_eq!(cached.details, result.details);
    }

    #[test]
    fn get_pass1_returns_none_on_miss() {
        // setup
        let store = CacheStore::open_in_memory().unwrap();
        let key = sample_key();
        store.store_pass1(&key, &sample_pass1()).unwrap();

        let different_key = CacheKey {
            pr_id: "99".into(),
            ..key
        };

        // execute
        let result = store.get_pass1(&different_key).unwrap();

        // verify
        assert!(result.is_none());
    }

    #[test]
    fn get_pass2_returns_none_on_miss() {
        // setup
        let store = CacheStore::open_in_memory().unwrap();
        let key = sample_key();
        store
            .store_pass2(&key, "src/auth.rs", &sample_pass2())
            .unwrap();

        // execute — different file path
        let result = store.get_pass2(&key, "src/other.rs").unwrap();

        // verify
        assert!(result.is_none());
    }

    #[test]
    fn schema_version_mismatch_returns_none() {
        // setup — insert a row with schema_version = 0 directly via SQL
        let store = CacheStore::open_in_memory().unwrap();
        let key = sample_key();
        let json = serde_json::to_string(&sample_pass1()).unwrap();
        store
            .conn
            .execute(
                "INSERT INTO pass1_cache
                    (pr_id, base_sha, head_sha, provider, schema_version, result_json, created_at)
                 VALUES (?1, ?2, ?3, ?4, 0, ?5, '2026-01-01T00:00:00Z')",
                params![key.pr_id, key.base_sha, key.head_sha, key.provider, json],
            )
            .unwrap();

        // execute — get_pass1 queries for SCHEMA_VERSION = 1
        let result = store.get_pass1(&key).unwrap();

        // verify
        assert!(
            result.is_none(),
            "old schema version should be a cache miss"
        );
    }

    #[test]
    fn get_cached_files_returns_stored_paths() {
        // setup
        let store = CacheStore::open_in_memory().unwrap();
        let key = sample_key();
        let files = ["src/a.rs", "src/b.rs", "src/c.rs"];
        for file in &files {
            store.store_pass2(&key, file, &sample_pass2()).unwrap();
        }

        // execute
        let cached_files = store.get_cached_files(&key).unwrap();

        // verify
        assert_eq!(cached_files.len(), 3);
        for file in &files {
            assert!(cached_files.contains(&file.to_string()), "missing {file}");
        }
    }

    #[test]
    fn get_latest_head_sha_returns_none_when_empty() {
        // setup
        let store = CacheStore::open_in_memory().unwrap();

        // execute
        let result = store.get_latest_head_sha("42", "aaaaaa", "claude").unwrap();

        // verify
        assert!(result.is_none());
    }

    #[test]
    fn get_latest_head_sha_returns_stored_sha() {
        // setup
        let store = CacheStore::open_in_memory().unwrap();
        let key = sample_key();
        store.store_pass1(&key, &sample_pass1()).unwrap();

        // execute
        let result = store
            .get_latest_head_sha(&key.pr_id, &key.base_sha, &key.provider)
            .unwrap();

        // verify
        assert_eq!(result, Some("bbbbbb".to_owned()));
    }

    #[test]
    fn get_latest_head_sha_returns_most_recent_when_multiple() {
        // setup — store two entries with different head SHAs for the same PR+base+provider
        let store = CacheStore::open_in_memory().unwrap();
        let key1 = sample_key(); // head_sha = "bbbbbb"
        let key2 = CacheKey {
            head_sha: "cccccc".into(),
            ..sample_key()
        };
        store.store_pass1(&key1, &sample_pass1()).unwrap();
        store.store_pass1(&key2, &sample_pass1()).unwrap();

        // execute
        let result = store
            .get_latest_head_sha(&key1.pr_id, &key1.base_sha, &key1.provider)
            .unwrap();

        // verify — key2 was inserted last, so cccccc should be returned
        assert_eq!(result, Some("cccccc".to_owned()));
    }

    #[test]
    fn get_latest_head_sha_ignores_different_pr() {
        // setup — store for PR 42, query for PR 99
        let store = CacheStore::open_in_memory().unwrap();
        let key = sample_key();
        store.store_pass1(&key, &sample_pass1()).unwrap();

        // execute
        let result = store
            .get_latest_head_sha("99", &key.base_sha, &key.provider)
            .unwrap();

        // verify
        assert!(result.is_none());
    }

    #[test]
    fn mark_and_retrieve_viewed() {
        // setup
        let store = CacheStore::open_in_memory().unwrap();

        // execute
        store.mark_viewed("42", "src/auth.rs", "sha-abc").unwrap();
        let sha = store.get_viewed_sha("42", "src/auth.rs").unwrap();

        // verify
        assert_eq!(sha, Some("sha-abc".to_owned()));
    }

    #[test]
    fn mark_viewed_upserts() {
        // setup
        let store = CacheStore::open_in_memory().unwrap();
        store.mark_viewed("42", "src/auth.rs", "sha-old").unwrap();

        // execute — mark same file again with a new SHA
        store.mark_viewed("42", "src/auth.rs", "sha-new").unwrap();
        let sha = store.get_viewed_sha("42", "src/auth.rs").unwrap();

        // verify — second SHA wins
        assert_eq!(sha, Some("sha-new".to_owned()));
    }

    #[test]
    fn get_viewed_sha_returns_none_when_not_viewed() {
        // setup
        let store = CacheStore::open_in_memory().unwrap();

        // execute
        let sha = store.get_viewed_sha("42", "src/never_viewed.rs").unwrap();

        // verify
        assert!(sha.is_none());
    }

    #[test]
    fn list_viewed_returns_all_files_for_pr() {
        // setup
        let store = CacheStore::open_in_memory().unwrap();
        store.mark_viewed("42", "src/a.rs", "sha-a").unwrap();
        store.mark_viewed("42", "src/b.rs", "sha-b").unwrap();
        store.mark_viewed("99", "src/c.rs", "sha-c").unwrap(); // different PR

        // execute
        let viewed = store.list_viewed("42").unwrap();

        // verify — only PR 42's files, sorted
        assert_eq!(viewed.len(), 2);
        assert_eq!(viewed[0], ("src/a.rs".to_owned(), "sha-a".to_owned()));
        assert_eq!(viewed[1], ("src/b.rs".to_owned(), "sha-b".to_owned()));
    }

    #[test]
    fn overwrite_updates_existing() {
        // setup
        let store = CacheStore::open_in_memory().unwrap();
        let key = sample_key();
        let first = sample_pass1();
        let second = Pass1Output {
            summary: "Updated summary.".into(),
            ..sample_pass1()
        };

        // execute
        store.store_pass1(&key, &first).unwrap();
        store.store_pass1(&key, &second).unwrap();
        let cached = store.get_pass1(&key).unwrap();

        // verify
        let cached = cached.expect("expected a cache hit");
        assert_eq!(cached.summary, "Updated summary.");
    }
}
