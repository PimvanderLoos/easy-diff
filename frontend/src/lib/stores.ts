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

/** Light or dark UI theme. */
export type Theme = "light" | "dark";

/** Storage key for the persisted theme choice. */
const THEME_STORAGE_KEY = "ed-theme";

/**
 * Resolve the initial theme: a previously persisted choice, falling back to the
 * OS `prefers-color-scheme` preference, then light.
 */
function initialTheme(): Theme {
  if (typeof window === "undefined") return "light";
  const stored = window.localStorage.getItem(THEME_STORAGE_KEY);
  if (stored === "light" || stored === "dark") return stored;
  return window.matchMedia("(prefers-color-scheme: dark)").matches
    ? "dark"
    : "light";
}

/**
 * Active UI theme. Initialised from localStorage (falling back to the OS
 * preference) and persisted on every change. `App.svelte` applies it by
 * toggling the `dark` class on `<html>`.
 */
export const theme = writable<Theme>(initialTheme());

if (typeof window !== "undefined") {
  theme.subscribe((value) => {
    window.localStorage.setItem(THEME_STORAGE_KEY, value);
  });
}

/** Which screen is currently shown. Defaults to the PR selection screen. */
export const currentScreen = writable<Screen>("selection");

/** Whether the keyboard-shortcuts help dialog is open. */
export const helpOpen = writable<boolean>(false);

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

/**
 * Resets all review-scoped stores to their initial values.
 *
 * Called when leaving a PR's review screen (e.g. the "back to PR list" button)
 * so that file-path-keyed state (reviewed/expanded sets), filters, inspector
 * focus, and draft comments/overrides do not leak into the next PR opened.
 *
 * Deliberately leaves `diffViewMode` (and the theme) untouched — those are user
 * preferences worth preserving across PRs.
 */
export function resetReviewSession(): void {
  reviewedFiles.set(new Set());
  expandedFiles.set(new Set());
  filterState.set({ changeType: "all", attentionTags: [] });
  focusedHunkId.set(null);
  draftComments.set([]);
  categoryOverrides.set(new Map());
}
