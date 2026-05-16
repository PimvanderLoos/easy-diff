/**
 * Filter logic for the diff viewer.
 *
 * These functions determine which diff lines pass the active filter and what
 * attention tags are effective for a given line. They are shared between
 * `InlineDiff` and `SplitDiff` so dimming behavior is consistent.
 *
 * Ported from `edLinePasses` and `edLineTags` in `shared.jsx`.
 *
 * @example
 * ```ts
 * const passes = linePasses(line, classification, filter);
 * const tags   = lineTags(line, classification);
 * ```
 */

import type { DiffLineData, Classification, FilterState } from "./types.js";

/**
 * Returns the effective attention tags for a diff line.
 *
 * If the line has explicit per-line tag overrides, those are returned.
 * Otherwise falls back to the hunk-level classification tags.
 */
export function lineTags(
  line: DiffLineData,
  classification: Classification | null,
): string[] {
  if (line.tags && line.tags.length > 0) return line.tags;
  return classification?.attentionTags ?? [];
}

/**
 * Returns whether a diff line passes the active filter.
 *
 * Context lines always pass (they are never dimmed). For add/del lines:
 * - If a change-type filter is active (not "all"), the hunk's change type
 *   must match.
 * - If attention-tag filters are active, the line must carry at least one
 *   of the selected tags (checked via `lineTags`).
 *
 * A null or undefined `filter` is treated as "no filter" (all lines pass).
 */
export function linePasses(
  line: DiffLineData,
  classification: Classification | null,
  filter: FilterState | null | undefined,
): boolean {
  if (!filter) return true;
  if (line.type === "ctx") return true;

  // Change-type filter (single-select).
  if (filter.changeType && filter.changeType !== "all") {
    if (
      (classification?.changeType ?? "uncategorized") !== filter.changeType
    ) {
      return false;
    }
  }

  // Attention-tag filter (multi-select). Empty set means no constraint.
  if (filter.attentionTags.length > 0) {
    const tags = lineTags(line, classification);
    if (!tags.some((t) => filter.attentionTags.includes(t))) return false;
  }

  return true;
}
