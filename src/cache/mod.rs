//! SQLite-backed caching layer for LLM analysis results, keyed by PR state,
//! file path, provider, and schema version. Also stores review comments and
//! manual category overrides locally until submission.
#![allow(dead_code)]
//!
//! [`CacheStore`] stores and retrieves [`Pass1Output`] and [`Pass2Output`] values.
//! The cache is keyed by a [`CacheKey`] (PR id, base/head SHAs, and provider name).
//! Pass 2 results are additionally keyed by file path.
//!
//! Schema version is embedded in every row; bumping [`SCHEMA_VERSION`] automatically
//! causes all old entries to be treated as cache misses without requiring a migration.
//!
//! Review comments ([`ReviewComment`]) are persisted in `review_comments` and
//! category overrides ([`CategoryOverride`]) in `category_overrides`. Both tables
//! are not versioned — they survive schema bumps.
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
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::llm::schema::{Pass1Output, Pass2Output};

/// Status of a [`ReviewComment`]: either a local draft or already submitted to the platform.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CommentStatus {
    /// Comment has been written locally but not yet submitted.
    Draft,
    /// Comment has been submitted to GitHub/BitBucket.
    Submitted,
}

/// An inline review comment on a specific line range within a file in a PR.
///
/// Comments are persisted in SQLite until submission. The `id` is assigned by the
/// database on insert; callers constructing a new comment before saving should use
/// a sentinel value (e.g. `0`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewComment {
    /// Database-assigned primary key. `0` before the first save.
    pub id: i64,
    /// Platform-specific PR identifier (e.g. GitHub PR number as string).
    pub pr_id: String,
    /// Relative path of the file being commented on.
    pub file_path: String,
    /// First line of the commented range (1-based).
    pub start_line: u32,
    /// Last line of the commented range, or `None` for a single-line comment.
    pub end_line: Option<u32>,
    /// Markdown body of the comment.
    pub body: String,
    /// ISO 8601 UTC timestamp when the comment was created locally.
    pub created_at: String,
    /// Whether the comment has been submitted to the platform.
    pub status: CommentStatus,
}

/// A manual override of the LLM classification for a specific diff hunk.
///
/// Overrides are keyed by `(pr_id, file_path, hunk_id)` and replace the LLM's
/// `change_type` and/or `attention_tags` with user-supplied values. Both fields
/// are optional — setting only one leaves the other at its LLM-inferred value.
///
/// The `id` is assigned by the database on insert.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryOverride {
    /// Database-assigned primary key.
    pub id: i64,
    /// Platform-specific PR identifier.
    pub pr_id: String,
    /// Relative path of the file containing the hunk.
    pub file_path: String,
    /// Identifier for the hunk being overridden (e.g. the `@@` header string).
    pub hunk_id: String,
    /// User-supplied change type, overriding the LLM value. `None` = no override.
    pub change_type: Option<String>,
    /// User-supplied attention tags, overriding the LLM value. `None` = no override.
    pub attention_tags: Option<Vec<String>>,
}

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
            );
            CREATE TABLE IF NOT EXISTS review_comments (
                id         INTEGER PRIMARY KEY AUTOINCREMENT,
                pr_id      TEXT    NOT NULL,
                file_path  TEXT    NOT NULL,
                start_line INTEGER NOT NULL,
                end_line   INTEGER,
                body       TEXT    NOT NULL,
                created_at TEXT    NOT NULL,
                status     TEXT    NOT NULL DEFAULT 'draft'
            );
            CREATE TABLE IF NOT EXISTS category_overrides (
                id             INTEGER PRIMARY KEY AUTOINCREMENT,
                pr_id          TEXT NOT NULL,
                file_path      TEXT NOT NULL,
                hunk_id        TEXT NOT NULL,
                change_type    TEXT,
                attention_tags TEXT,
                UNIQUE(pr_id, file_path, hunk_id)
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

    /// Removes the viewed record for `(pr_id, file_path)`, if one exists.
    ///
    /// Used when a file is unmarked as reviewed. Succeeds as a no-op when no
    /// record exists.
    pub fn unmark_viewed(&self, pr_id: &str, file_path: &str) -> Result<(), CacheError> {
        self.conn.execute(
            "DELETE FROM viewed_files WHERE pr_id = ?1 AND file_path = ?2",
            params![pr_id, file_path],
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

    // ── Review comment CRUD ──────────────────────────────────────────────────

    /// Inserts a new [`ReviewComment`] and returns it with its database-assigned `id`.
    ///
    /// The `id` field on the input value is ignored; the returned struct reflects
    /// the actual row id assigned by SQLite.
    pub fn add_comment(&self, comment: &ReviewComment) -> Result<ReviewComment, CacheError> {
        let status = match comment.status {
            CommentStatus::Draft => "draft",
            CommentStatus::Submitted => "submitted",
        };
        self.conn.execute(
            "INSERT INTO review_comments
                (pr_id, file_path, start_line, end_line, body, created_at, status)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                comment.pr_id,
                comment.file_path,
                comment.start_line,
                comment.end_line,
                comment.body,
                comment.created_at,
                status,
            ],
        )?;
        let id = self.conn.last_insert_rowid();
        Ok(ReviewComment {
            id,
            ..comment.clone()
        })
    }

    /// Returns all [`ReviewComment`]s for the given PR, ordered by `id` ascending.
    pub fn list_comments(&self, pr_id: &str) -> Result<Vec<ReviewComment>, CacheError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, pr_id, file_path, start_line, end_line, body, created_at, status
             FROM review_comments
             WHERE pr_id = ?1
             ORDER BY id ASC",
        )?;
        let rows = stmt.query_map(params![pr_id], |row| {
            let status_str: String = row.get(7)?;
            let status = if status_str == "submitted" {
                CommentStatus::Submitted
            } else {
                CommentStatus::Draft
            };
            Ok(ReviewComment {
                id: row.get(0)?,
                pr_id: row.get(1)?,
                file_path: row.get(2)?,
                start_line: row.get(3)?,
                end_line: row.get(4)?,
                body: row.get(5)?,
                created_at: row.get(6)?,
                status,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(CacheError::from)
    }

    /// Replaces the `body` and `status` of an existing comment identified by `id`.
    ///
    /// Returns `Ok(true)` when a row was updated, `Ok(false)` when no row matched.
    pub fn update_comment(
        &self,
        id: i64,
        body: &str,
        status: &CommentStatus,
    ) -> Result<bool, CacheError> {
        let status_str = match status {
            CommentStatus::Draft => "draft",
            CommentStatus::Submitted => "submitted",
        };
        let affected = self.conn.execute(
            "UPDATE review_comments SET body = ?1, status = ?2 WHERE id = ?3",
            params![body, status_str, id],
        )?;
        Ok(affected > 0)
    }

    /// Deletes the comment with the given `id`.
    ///
    /// Returns `Ok(true)` when a row was deleted, `Ok(false)` when no row matched.
    pub fn delete_comment(&self, id: i64) -> Result<bool, CacheError> {
        let affected = self
            .conn
            .execute("DELETE FROM review_comments WHERE id = ?1", params![id])?;
        Ok(affected > 0)
    }

    /// Marks all draft comments for `pr_id` as [`CommentStatus::Submitted`].
    ///
    /// Called after a successful review submission so local state stays in sync
    /// with what was sent to the platform.
    ///
    /// Returns the number of rows updated.
    pub fn mark_comments_submitted(&self, pr_id: &str) -> Result<usize, CacheError> {
        let affected = self.conn.execute(
            "UPDATE review_comments SET status = 'submitted' WHERE pr_id = ?1 AND status = 'draft'",
            params![pr_id],
        )?;
        Ok(affected)
    }

    // ── Category override CRUD ───────────────────────────────────────────────

    /// Inserts or replaces a [`CategoryOverride`] for the given `(pr_id, file_path, hunk_id)`.
    ///
    /// If an override already exists for that hunk it is replaced (upsert semantics).
    /// Returns the stored override with its database-assigned `id`.
    pub fn set_override(
        &self,
        pr_id: &str,
        file_path: &str,
        hunk_id: &str,
        change_type: Option<&str>,
        attention_tags: Option<&[String]>,
    ) -> Result<CategoryOverride, CacheError> {
        let tags_json = attention_tags.map(serde_json::to_string).transpose()?;
        self.conn.execute(
            "INSERT INTO category_overrides
                (pr_id, file_path, hunk_id, change_type, attention_tags)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(pr_id, file_path, hunk_id)
             DO UPDATE SET change_type = excluded.change_type,
                           attention_tags = excluded.attention_tags",
            params![pr_id, file_path, hunk_id, change_type, tags_json],
        )?;
        // Fetch the just-upserted row to return the canonical id.
        let mut stmt = self.conn.prepare(
            "SELECT id, change_type, attention_tags
             FROM category_overrides
             WHERE pr_id = ?1 AND file_path = ?2 AND hunk_id = ?3",
        )?;
        let mut rows = stmt.query(params![pr_id, file_path, hunk_id])?;
        let row = rows
            .next()?
            .ok_or_else(|| rusqlite::Error::QueryReturnedNoRows)?;
        let id: i64 = row.get(0)?;
        let ct: Option<String> = row.get(1)?;
        let tags_raw: Option<String> = row.get(2)?;
        let tags: Option<Vec<String>> = tags_raw
            .map(|s| serde_json::from_str(&s))
            .transpose()
            .map_err(CacheError::Serialization)?;
        Ok(CategoryOverride {
            id,
            pr_id: pr_id.to_owned(),
            file_path: file_path.to_owned(),
            hunk_id: hunk_id.to_owned(),
            change_type: ct,
            attention_tags: tags,
        })
    }

    /// Returns all [`CategoryOverride`]s for the given PR, ordered by `id` ascending.
    pub fn get_overrides(&self, pr_id: &str) -> Result<Vec<CategoryOverride>, CacheError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, pr_id, file_path, hunk_id, change_type, attention_tags
             FROM category_overrides
             WHERE pr_id = ?1
             ORDER BY id ASC",
        )?;
        let rows = stmt.query_map(params![pr_id], |row| {
            let tags_raw: Option<String> = row.get(5)?;
            // Deserialise attention_tags JSON inside the closure; map the error
            // to a rusqlite::Error so query_map stays happy.
            let attention_tags: Option<Vec<String>> = match tags_raw {
                None => None,
                Some(s) => Some(serde_json::from_str(&s).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        5,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?),
            };
            Ok(CategoryOverride {
                id: row.get(0)?,
                pr_id: row.get(1)?,
                file_path: row.get(2)?,
                hunk_id: row.get(3)?,
                change_type: row.get(4)?,
                attention_tags,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(CacheError::from)
    }

    /// Deletes the override for `(pr_id, file_path, hunk_id)`.
    ///
    /// Returns `Ok(true)` when a row was deleted, `Ok(false)` when no row matched.
    pub fn delete_override(
        &self,
        pr_id: &str,
        file_path: &str,
        hunk_id: &str,
    ) -> Result<bool, CacheError> {
        let affected = self.conn.execute(
            "DELETE FROM category_overrides
             WHERE pr_id = ?1 AND file_path = ?2 AND hunk_id = ?3",
            params![pr_id, file_path, hunk_id],
        )?;
        Ok(affected > 0)
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
    fn unmark_viewed_removes_record() {
        // setup
        let store = CacheStore::open_in_memory().unwrap();
        store.mark_viewed("42", "src/auth.rs", "sha-abc").unwrap();

        // execute
        store.unmark_viewed("42", "src/auth.rs").unwrap();
        let sha = store.get_viewed_sha("42", "src/auth.rs").unwrap();

        // verify — record is gone
        assert!(sha.is_none());
    }

    #[test]
    fn unmark_viewed_is_noop_when_absent() {
        // setup
        let store = CacheStore::open_in_memory().unwrap();

        // execute — unmarking a never-viewed file must not error
        let result = store.unmark_viewed("42", "src/never_viewed.rs");

        // verify
        assert!(result.is_ok());
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

    // ── ReviewComment CRUD ────────────────────────────────────────────────────

    fn sample_comment(pr_id: &str, file_path: &str) -> ReviewComment {
        ReviewComment {
            id: 0,
            pr_id: pr_id.to_owned(),
            file_path: file_path.to_owned(),
            start_line: 10,
            end_line: Some(15),
            body: "Consider extracting this into a helper.".to_owned(),
            created_at: "2026-01-01T00:00:00Z".to_owned(),
            status: CommentStatus::Draft,
        }
    }

    #[test]
    fn add_and_list_comments() {
        // setup
        let store = CacheStore::open_in_memory().unwrap();
        let c1 = sample_comment("42", "src/a.rs");
        let c2 = ReviewComment {
            start_line: 20,
            body: "Second comment.".to_owned(),
            ..sample_comment("42", "src/b.rs")
        };
        let c3 = ReviewComment {
            start_line: 30,
            body: "Third comment.".to_owned(),
            ..sample_comment("42", "src/c.rs")
        };

        // execute
        store.add_comment(&c1).unwrap();
        store.add_comment(&c2).unwrap();
        store.add_comment(&c3).unwrap();
        let list = store.list_comments("42").unwrap();

        // verify
        assert_eq!(list.len(), 3);
        assert_eq!(list[0].file_path, "src/a.rs");
        assert_eq!(list[1].file_path, "src/b.rs");
        assert_eq!(list[2].file_path, "src/c.rs");
        assert_eq!(list[0].status, CommentStatus::Draft);
    }

    #[test]
    fn update_comment_body() {
        // setup
        let store = CacheStore::open_in_memory().unwrap();
        let saved = store
            .add_comment(&sample_comment("42", "src/a.rs"))
            .unwrap();

        // execute
        let updated = store.update_comment(saved.id, "New body.", &CommentStatus::Submitted);

        // verify
        assert!(updated.unwrap());
        let list = store.list_comments("42").unwrap();
        assert_eq!(list[0].body, "New body.");
        assert_eq!(list[0].status, CommentStatus::Submitted);
    }

    #[test]
    fn delete_comment() {
        // setup
        let store = CacheStore::open_in_memory().unwrap();
        let saved = store
            .add_comment(&sample_comment("42", "src/a.rs"))
            .unwrap();

        // execute
        let deleted = store.delete_comment(saved.id).unwrap();
        let list = store.list_comments("42").unwrap();

        // verify
        assert!(deleted);
        assert!(list.is_empty());
    }

    #[test]
    fn comments_filtered_by_pr() {
        // setup
        let store = CacheStore::open_in_memory().unwrap();
        store
            .add_comment(&sample_comment("42", "src/a.rs"))
            .unwrap();
        store
            .add_comment(&sample_comment("99", "src/b.rs"))
            .unwrap();

        // execute
        let pr42 = store.list_comments("42").unwrap();
        let pr99 = store.list_comments("99").unwrap();

        // verify
        assert_eq!(pr42.len(), 1);
        assert_eq!(pr42[0].pr_id, "42");
        assert_eq!(pr99.len(), 1);
        assert_eq!(pr99[0].pr_id, "99");
    }

    #[test]
    fn mark_comments_submitted_updates_all_drafts() {
        // setup
        let store = CacheStore::open_in_memory().unwrap();
        store
            .add_comment(&sample_comment("42", "src/a.rs"))
            .unwrap();
        store
            .add_comment(&sample_comment("42", "src/b.rs"))
            .unwrap();
        // a comment for a different PR — must NOT be touched
        store
            .add_comment(&sample_comment("99", "src/c.rs"))
            .unwrap();

        // execute
        let count = store.mark_comments_submitted("42").unwrap();

        // verify — two PR-42 comments become Submitted; PR-99 stays Draft
        assert_eq!(count, 2);
        let pr42 = store.list_comments("42").unwrap();
        assert!(pr42.iter().all(|c| c.status == CommentStatus::Submitted));
        let pr99 = store.list_comments("99").unwrap();
        assert!(pr99.iter().all(|c| c.status == CommentStatus::Draft));
    }

    #[test]
    fn mark_comments_submitted_is_idempotent() {
        // setup
        let store = CacheStore::open_in_memory().unwrap();
        store
            .add_comment(&sample_comment("42", "src/a.rs"))
            .unwrap();

        // execute — call twice
        let first = store.mark_comments_submitted("42").unwrap();
        let second = store.mark_comments_submitted("42").unwrap();

        // verify — second call updates 0 rows (already submitted)
        assert_eq!(first, 1);
        assert_eq!(second, 0);
    }

    // ── CategoryOverride CRUD ─────────────────────────────────────────────────

    #[test]
    fn set_and_get_override() {
        // setup
        let store = CacheStore::open_in_memory().unwrap();
        let tags = vec!["security".to_owned(), "breaking-change".to_owned()];

        // execute
        store
            .set_override(
                "42",
                "src/auth.rs",
                "@@ -1,5 +1,10 @@",
                Some("feature"),
                Some(&tags),
            )
            .unwrap();
        let overrides = store.get_overrides("42").unwrap();

        // verify
        assert_eq!(overrides.len(), 1);
        let ov = &overrides[0];
        assert_eq!(ov.pr_id, "42");
        assert_eq!(ov.file_path, "src/auth.rs");
        assert_eq!(ov.hunk_id, "@@ -1,5 +1,10 @@");
        assert_eq!(ov.change_type.as_deref(), Some("feature"));
        assert_eq!(ov.attention_tags.as_deref(), Some(tags.as_slice()));
    }

    #[test]
    fn override_upserts() {
        // setup
        let store = CacheStore::open_in_memory().unwrap();
        let hunk = "@@ -1,5 +1,10 @@";
        store
            .set_override("42", "src/auth.rs", hunk, Some("refactor"), None)
            .unwrap();

        // execute — second call for same hunk with different values
        store
            .set_override(
                "42",
                "src/auth.rs",
                hunk,
                Some("feature"),
                Some(&["security".to_owned()]),
            )
            .unwrap();
        let overrides = store.get_overrides("42").unwrap();

        // verify — second set wins; still only one row
        assert_eq!(overrides.len(), 1);
        assert_eq!(overrides[0].change_type.as_deref(), Some("feature"));
        assert_eq!(
            overrides[0].attention_tags.as_deref(),
            Some(["security".to_owned()].as_slice())
        );
    }

    #[test]
    fn delete_override() {
        // setup
        let store = CacheStore::open_in_memory().unwrap();
        let hunk = "@@ -1,5 +1,10 @@";
        store
            .set_override("42", "src/auth.rs", hunk, Some("feature"), None)
            .unwrap();

        // execute
        let deleted = store.delete_override("42", "src/auth.rs", hunk).unwrap();
        let overrides = store.get_overrides("42").unwrap();

        // verify
        assert!(deleted);
        assert!(overrides.is_empty());
    }
}
