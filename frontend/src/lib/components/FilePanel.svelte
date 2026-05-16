<script lang="ts">
  /**
   * Collapsible card for a single file in the review screen.
   *
   * When expanded: shows all hunks via HunkSection.
   * When collapsed: shows only the FileHeader + FileTagRow summary.
   *
   * Aggregates all attention tags across the file's hunks so the collapsed
   * row can display them without expanding.
   *
   * Translates `BFilePanel` from `concept-b.jsx` to Svelte.
   *
   * @example
   * ```svelte
   * <FilePanel
   *   file={f}
   *   hunks={hunks[f.path] ?? []}
   *   classifications={classifications}
   *   reviewed={reviewedSet.has(f.path)}
   *   collapsed={collapsedSet.has(f.path)}
   *   onToggleReviewed={() => toggleReviewed(f.path)}
   *   onToggleCollapsed={() => toggleCollapsed(f.path)}
   *   isDark={true}
   * />
   * ```
   */

  import FileHeader from "./FileHeader.svelte";
  import FileTagRow from "./FileTagRow.svelte";
  import HunkSection from "./HunkSection.svelte";
  import type {
    FileEntry,
    HunkData,
    Classification,
    FilterState,
  } from "../types.js";

  interface Props {
    /** File metadata. */
    file: FileEntry;
    /** Hunks for this file. */
    hunks: HunkData[];
    /** Classification map keyed by hunk id. */
    classifications: Record<string, Classification>;
    /** Whether the file is marked as reviewed. */
    reviewed: boolean;
    /** Whether the panel is collapsed. */
    collapsed: boolean;
    /** Called to toggle reviewed state. */
    onToggleReviewed?: () => void;
    /** Called to toggle collapsed state. */
    onToggleCollapsed?: () => void;
    /** Hunk id that is currently focused in the inspector (null = none). */
    focusedHunkId?: string | null;
    /** Whether the dark theme is active. */
    isDark: boolean;
    /** Active filter for dimming non-matching lines. */
    filter?: FilterState | null;
    /** Diff view mode: inline (unified) or split (side-by-side). */
    diffView?: "inline" | "split";
  }

  let {
    file,
    hunks,
    classifications,
    reviewed,
    collapsed,
    onToggleReviewed,
    onToggleCollapsed,
    focusedHunkId = null,
    isDark,
    filter = null,
    diffView = "inline",
  }: Props = $props();

  /**
   * Aggregate every unique attention tag across all hunks in the file.
   * Used by FileTagRow to show a quick summary in collapsed state.
   */
  const fileTags = $derived(() => {
    const seen = new Set<string>();
    for (const h of hunks) {
      const cls = classifications[h.id];
      if (cls) {
        for (const tag of cls.attentionTags) seen.add(tag);
      }
    }
    return [...seen];
  });
</script>

<div
  style="
    background: var(--ed-bg);
    border: 1px solid var(--ed-border-subtle);
    border-radius: 8px;
    overflow: hidden;
    box-shadow: 0 1px 2px rgba(15,23,42,0.04);
    flex-shrink: 0;
  "
>
  <FileHeader
    {file}
    {reviewed}
    {collapsed}
    onToggleCollapsed={onToggleCollapsed}
    onToggleReviewed={onToggleReviewed}
  />

  {#if collapsed}
    <FileTagRow tags={fileTags()} hunkCount={hunks.length} />
  {:else}
    {#each hunks as hunk, i (hunk.id)}
      <HunkSection
        {hunk}
        classification={classifications[hunk.id] ?? null}
        filePath={file.path}
        isFirst={i === 0}
        focused={focusedHunkId === hunk.id}
        {isDark}
        {filter}
        {diffView}
      />
    {/each}
  {/if}
</div>
