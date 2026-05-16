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
 * - `collapsedFiles` — set of file paths whose panels are collapsed.
 * - `focusedHunkId` — hunk id currently selected in the inspector (null = none).
 * - `diffViewMode` — whether the diff is shown inline (unified) or split (side-by-side).
 */

import { writable } from "svelte/store";
import type { FilterState, PullRequest } from "./types.js";

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
 * Set of file paths whose FilePanel is collapsed.
 *
 * Managed by `toggleCollapsed` in ReviewScreen. Marking a file reviewed
 * automatically adds it here; unmarking removes it (auto-expand).
 */
export const collapsedFiles = writable<Set<string>>(new Set());

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
