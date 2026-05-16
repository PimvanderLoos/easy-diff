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
