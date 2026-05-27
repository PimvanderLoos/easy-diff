/**
 * TypeScript interfaces matching the Rust structs exposed via Tauri commands.
 *
 * Field names use snake_case to match serde's default serialisation from Rust.
 * Keep in sync with:
 *   - `src/platform/mod.rs` — `PullRequest`
 *   - `src/git/mod.rs` — `RepoInfo`
 *   - `src/gui/mod.rs` — `GuiConfig`
 *   - `src/diff/mod.rs` — `DiffFile`, `DiffHunk`, `DiffLine`
 *   - `src/llm/schema/mod.rs` — `Pass1Output`, `Pass2Output`
 *   - `src/analysis/mod.rs` — `AnalysisResult`
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

// ── Analysis / diff types ──────────────────────────────────────────────────

/**
 * A single line within a diff hunk, with its type and optional per-line
 * attention tags. Mirrors the shape from `diff-data.js`.
 */
export interface DiffLineData {
  /** Line classification: added, removed, or unchanged context. */
  type: "add" | "del" | "ctx";
  /** Old file line number (null for added lines). */
  old: number | null;
  /** New file line number (null for deleted lines). */
  new: number | null;
  /** Raw text of the line (no leading +/-/ marker). */
  text: string;
  /** Per-line attention tag overrides (falls back to hunk-level tags). */
  tags?: string[];
}

/**
 * A contiguous block of diff lines within a file.
 *
 * `id` is a unique identifier used as the key in `CLASSIFICATIONS`.
 */
export interface HunkData {
  /** Stable identifier for this hunk, used as key in classification map. */
  id: string;
  /** Raw `@@ -old +new @@` header. */
  header: string;
  /** First line number in the old file. */
  oldStart: number;
  /** First line number in the new file. */
  newStart: number;
  /** Whether this hunk introduces an entirely new file. */
  isNewFile?: boolean;
  /** Classified diff lines within this hunk. */
  lines: DiffLineData[];
}

/**
 * LLM classification for a single hunk.
 *
 * Mirrors `CLASSIFICATIONS` entries from `diff-data.js`.
 */
export interface Classification {
  /** Primary change type for this hunk. */
  changeType: string;
  /** Attention tags flagged by the LLM. */
  attentionTags: string[];
  /** Confidence score in [0, 1]. */
  confidence: number;
  /** One-sentence summary of the change. */
  summary: string;
  /** Rationale explaining why this hunk warrants attention. */
  rationale: string;
}

/** A file entry within the parsed diff, with per-file metadata. */
export interface FileEntry {
  /** Full file path within the repository. */
  path: string;
  /** Short filename (basename). */
  short: string;
  /** Directory portion of the path. */
  dir: string;
  /** Number of added lines. */
  adds: number;
  /** Number of deleted lines. */
  dels: number;
  /** Whether the file is risky according to the analysis. */
  risky: boolean;
  /** Whether the file has already been reviewed. */
  reviewed: boolean;
  /** Whether this is a newly created file. */
  isNew?: boolean;
}

/**
 * Pass 1 (global summary) output from the LLM analysis.
 *
 * Mirrors `src/llm/schema/mod.rs` → `Pass1Output` (snake_case).
 */
export interface Pass1Output {
  /** One-paragraph summary of the overall PR change. */
  summary: string;
  /** Change types present across the whole PR. */
  change_types: string[];
  /** Attention tags raised at the PR level. */
  attention_tags: string[];
  /** Logical groupings of related files. */
  file_clusters: Array<{
    label: string;
    files: string[];
    rationale: string;
  }>;
}

/**
 * Per-file (Pass 2) output from the LLM analysis.
 *
 * Mirrors `src/llm/schema/mod.rs` → `Pass2Output` (snake_case).
 */
export interface Pass2Output {
  /** One-paragraph summary of changes in this file. */
  summary: string;
  /** Change types for this file. */
  change_types: string[];
  /** Attention tags raised for this file. */
  attention_tags: string[];
  /** Notable observations (risks, suggestions, edge cases). */
  details: string[];
}

/**
 * Combined result of both analysis passes.
 *
 * Mirrors `src/analysis/mod.rs` → `AnalysisResult`.
 */
export interface AnalysisResult {
  /** Global PR summary from Pass 1. */
  pass1: Pass1Output;
  /** Per-file analysis from Pass 2, keyed by file path. */
  files: Record<string, Pass2Output>;
  /** Whether each file has unseen changes since last viewed. */
  has_changes_since_viewed: Record<string, boolean>;
}

/**
 * Active filter state for the diff viewer.
 *
 * `changeType` of `"all"` means no type filter is active.
 * Empty `attentionTags` array means no tag filter is active.
 */
export interface FilterState {
  /** Selected change type, or `"all"` to show all. */
  changeType: string;
  /** Active attention tag filters (multi-select). */
  attentionTags: string[];
}

// ── Change type / attention tag constants ──────────────────────────────────

/**
 * Built-in change type descriptors, matching `shared.jsx` → `CHANGE_TYPES`.
 * The `"all"` entry is the "no filter" sentinel.
 */
export const CHANGE_TYPES: Array<{ id: string; label: string }> = [
  { id: "all", label: "All" },
  { id: "logic", label: "Logic" },
  { id: "refactor", label: "Refactor" },
  { id: "structural", label: "Structural" },
  { id: "chore", label: "Chore" },
  { id: "documentation", label: "Documentation" },
  { id: "uncategorized", label: "Uncategorized" },
];

/**
 * Built-in attention tag descriptors, matching `shared.jsx` → `ATTENTION_TAGS`.
 * Hue values drive the oklch() colour formula used by `edTagColor`.
 */
export const ATTENTION_TAGS: Array<{
  id: string;
  label: string;
  short: string;
  hue: number;
}> = [
  { id: "security", label: "Security", short: "sec", hue: 12 },
  { id: "breaking-change", label: "Breaking Change", short: "brk", hue: 0 },
  { id: "needs-test", label: "Needs Test", short: "test", hue: 28 },
  { id: "complexity", label: "Complexity", short: "cplx", hue: 285 },
  { id: "off-topic", label: "Off-Topic", short: "off", hue: 340 },
  { id: "nitpick", label: "Nitpick", short: "nit", hue: 235 },
  { id: "design-decision", label: "Design Decision", short: "dsgn", hue: 70 },
  { id: "straightforward", label: "Straightforward", short: "ok", hue: 145 },
  { id: "well-tested", label: "Well Tested", short: "test+", hue: 160 },
  { id: "clean-up", label: "Clean Up", short: "cln", hue: 190 },
];

/** Lookup map from attention tag id to its descriptor. */
export const ATTENTION_TAG_BY_ID = Object.fromEntries(
  ATTENTION_TAGS.map((t) => [t.id, t]),
) as Record<string, (typeof ATTENTION_TAGS)[number]>;

/**
 * Returns the foreground colour for an attention tag (oklch formula).
 *
 * Dark/light theming is handled via the CSS `prefers-color-scheme` class on
 * `<html>` — but we need the raw colour for inline styles (e.g. tag pills).
 * Pass `isDark` from the app's theme state.
 */
export function edTagColor(tagId: string, isDark: boolean): string | null {
  const tag = ATTENTION_TAG_BY_ID[tagId];
  if (!tag) return null;
  return isDark
    ? `oklch(0.74 0.17 ${tag.hue})`
    : `oklch(0.50 0.18 ${tag.hue})`;
}

/**
 * Returns the background colour for an attention tag (oklch formula).
 *
 * `strong` produces a more saturated background (used in pill chips).
 */
export function edTagBg(
  tagId: string,
  isDark: boolean,
  strong = false,
): string | null {
  const tag = ATTENTION_TAG_BY_ID[tagId];
  if (!tag) return null;
  if (isDark) {
    return `oklch(0.74 0.17 ${tag.hue} / ${strong ? 0.2 : 0.12})`;
  }
  return strong
    ? `oklch(0.93 0.08 ${tag.hue})`
    : `oklch(0.96 0.05 ${tag.hue})`;
}

// ── Review comment types ───────────────────────────────────────────────────

/**
 * Status of a review comment: draft (local only) or submitted to the platform.
 *
 * Mirrors `src/cache/mod.rs` → `CommentStatus`.
 */
export type CommentStatus = "Draft" | "Submitted";

/**
 * An inline review comment on a specific line (or line range) within a file.
 *
 * Mirrors `src/cache/mod.rs` → `ReviewComment`.
 */
export interface ReviewComment {
  /** Database-assigned primary key. 0 before the first save. */
  id: number;
  /** Platform-specific PR identifier (e.g. GitHub PR number as string). */
  pr_id: string;
  /** Relative path of the file being commented on. */
  file_path: string;
  /** First line of the commented range (1-based). */
  start_line: number;
  /** Last line of the commented range, or null for a single-line comment. */
  end_line: number | null;
  /** Markdown body of the comment. */
  body: string;
  /** ISO 8601 UTC timestamp when the comment was created locally. */
  created_at: string;
  /** Whether the comment has been submitted to the platform. */
  status: CommentStatus;
}

/**
 * A manual override of the LLM classification for a specific diff hunk.
 *
 * Mirrors `src/cache/mod.rs` → `CategoryOverride`.
 */
export interface CategoryOverride {
  /** Database-assigned primary key. */
  id: number;
  /** Platform-specific PR identifier. */
  pr_id: string;
  /** Relative path of the file containing the hunk. */
  file_path: string;
  /** Identifier for the hunk being overridden (e.g. the `@@` header string). */
  hunk_id: string;
  /** User-supplied change type, overriding the LLM value. null = no override. */
  change_type: string | null;
  /** User-supplied attention tags, overriding the LLM value. null = no override. */
  attention_tags: string[] | null;
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
