/**
 * TypeScript interfaces matching the Rust structs exposed via Tauri commands.
 *
 * Field names use snake_case to match serde's default serialisation from Rust.
 * Keep in sync with:
 *   - `src/platform/mod.rs` — `PullRequest`
 *   - `src/git/mod.rs` — `RepoInfo`
 *   - `src/gui/mod.rs` — `GuiConfig`
 */

/** A pull request from any supported platform. */
export interface PullRequest {
  /** PR number, unique within a repository. */
  number: number;
  /** PR title. */
  title: string;
  /** Author username. */
  author: string;
  /** Source (head) branch name. */
  source_branch: string;
  /** Target (base) branch name. */
  target_branch: string;
  /** Git SHA of the base branch tip at PR creation time. */
  base_sha: string;
  /** Git SHA of the head branch tip. */
  head_sha: string;
  /** ISO 8601 creation timestamp. */
  created_at: string;
  /** ISO 8601 last-updated timestamp. */
  updated_at: string;
}

/** Hosting platform detected from a git remote URL. */
export type Platform = "GitHub" | "BitBucket";

/** Information about the current git repository. */
export interface RepoInfo {
  /** Absolute path to the repository root. */
  root: string;
  /** Hosting platform. */
  platform: Platform;
  /** Hostname of the remote (e.g. "github.com"). */
  host: string;
  /** Repository owner (user or organisation). */
  owner: string;
  /** Repository name (without `.git` suffix). */
  repo: string;
  /** Remote name used for detection (always "origin" for now). */
  remote_name: string;
  /** Raw remote URL string as configured in git. */
  remote_url: string;
}

/** Active LLM provider name. */
export type Provider = "claude" | "codex" | "gemini";

/** Resolved configuration returned by the `get_config` Tauri command. */
export interface GuiConfig {
  /** Primary LLM provider. */
  default_provider: Provider;
  /** Fallback LLM provider, if configured. */
  fallback_provider: Provider | null;
  /** GitHub backend in use ("auto", "gh", or "api"). */
  github_backend: string;
}
