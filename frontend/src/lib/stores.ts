/**
 * Application-level Svelte stores for screen state and navigation.
 *
 * Screen routing is driven by `currentScreen`. Selecting a PR sets
 * `selectedPr` and transitions to the `'review'` screen. The review
 * screen reads `selectedPr` to determine which PR to display.
 *
 * Review-specific stores:
 * - `filterState` — active change-type / attention-tag filters for the diff viewer.
 * - `reviewedFiles` — set of file paths the user has marked as reviewed.
 * - `expandedFiles` — set of file paths whose code body is expanded (else collapsed).
 * - `focusedHunkId` — hunk id currently selected in the inspector (null = none).
 * - `diffViewMode` — whether the diff is shown inline (unified) or split (side-by-side).
 */

import { writable } from "svelte/store";
import type {
  FilterState,
  PullRequest,
  ReviewComment,
  CategoryOverride,
} from "./types.js";

/** The active top-level screen. */
export type Screen = "selection" | "review";

/** Which screen is currently shown. Defaults to the PR selection screen. */
export const currentScreen = writable<Screen>("selection");

/** The PR that was selected from the list. Null when on the selection screen. */
export const selectedPr = writable<PullRequest | null>(null);

/**
 * Active filter state for the diff viewer.
 *
 * `changeType: "all"` means no change-type filter is active.
 * Empty `attentionTags` means no tag filter is active.
 */
export const filterState = writable<FilterState>({
  changeType: "all",
  attentionTags: [],
});

/**
 * Set of file paths the user has explicitly marked as reviewed.
 *
 * Toggling a file's status pill in the review screen adds/removes its path.
 * The progress bar in the left rail reflects this set's size.
 */
export const reviewedFiles = writable<Set<string>>(new Set());

/**
 * Set of file paths whose FilePanel body (code) is expanded.
 *
 * Empty by default, so every file starts collapsed: the FileHeader and the
 * FileTagRow summary are always shown, but the code stays hidden until the
 * first click. Managed by `toggleExpanded` in ReviewScreen. Marking a file
 * reviewed removes it here (collapse); unmarking re-adds it (auto-expand).
 */
export const expandedFiles = writable<Set<string>>(new Set());

/**
 * The hunk id currently selected in the inspector panel.
 *
 * Set by clicking a hunk header in HunkSection. Setting a non-null value
 * opens the inspector; null closes it.
 */
export const focusedHunkId = writable<string | null>(null);

/**
 * Current diff view mode: `"inline"` (unified) or `"split"` (side-by-side).
 *
 * Driven by the SegToggle in MainToolbar. HunkSection reads this to choose
 * between InlineDiff and SplitDiff.
 */
export const diffViewMode = writable<"inline" | "split">("inline");

/**
 * Draft review comments for the currently open PR.
 *
 * Loaded on review screen mount via `list_comments`. Updated optimistically
 * as the user adds, edits, or deletes comments.
 */
export const draftComments = writable<ReviewComment[]>([]);

/**
 * Manual category overrides for the currently open PR, keyed by hunk id.
 *
 * Loaded on review screen mount via `get_category_overrides`. Updated
 * optimistically when the user applies an override from the context menu.
 */
export const categoryOverrides = writable<Map<string, CategoryOverride>>(
  new Map(),
);
