<script lang="ts">
  /**
   * Diff renderer dispatcher.
   *
   * Delegates to `InlineDiff` (unified view) or `SplitDiff` (side-by-side view)
   * based on the `view` prop. All other props are forwarded unchanged.
   *
   * Translates `EDDiff` from `shared.jsx` to Svelte.
   *
   * @example
   * ```svelte
   * <DiffView
   *   view="split"
   *   lines={hunk.lines}
   *   filePath="src/Foo.java"
   *   classification={cls}
   *   isDark={true}
   *   filter={$filterState}
   * />
   * ```
   */

  import InlineDiff from "./InlineDiff.svelte";
  import SplitDiff from "./SplitDiff.svelte";
  import type { DiffLineData, Classification, FilterState } from "../types.js";

  interface Props {
    /** Which renderer to use. */
    view: "inline" | "split";
    /** The diff lines to render. */
    lines: DiffLineData[];
    /** File path — used to select the syntax highlighter. */
    filePath: string;
    /** LLM classification for the enclosing hunk (provides fallback tags). */
    classification: Classification | null;
    /** Whether the dark theme is active. */
    isDark: boolean;
    /** Active filter — non-passing add/del lines are dimmed. */
    filter?: FilterState | null;
  }

  let { view, lines, filePath, classification, isDark, filter = null }: Props =
    $props();
</script>

{#if view === "split"}
  <SplitDiff {lines} {filePath} {classification} {isDark} {filter} />
{:else}
  <InlineDiff {lines} {filePath} {classification} {isDark} {filter} />
{/if}
