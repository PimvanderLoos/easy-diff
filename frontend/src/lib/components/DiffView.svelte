<script lang="ts">
  /**
   * Diff renderer dispatcher.
   *
   * Delegates to `InlineDiff` (unified view) or `SplitDiff` (side-by-side view)
   * based on the `view` prop. All other props are forwarded unchanged.
   *
   * @example
   * ```svelte
   * <DiffView
   *   view="split"
   *   lines={hunk.lines}
   *   filePath="src/Foo.java"
   *   hunkId="src/Foo.java:0"
   *   classification={cls}
   *   isDark={true}
   *   filter={$filterState}
   *   prId="42"
   *   comments={draftsForHunk}
   * />
   * ```
   */

  import InlineDiff from "./InlineDiff.svelte";
  import SplitDiff from "./SplitDiff.svelte";
  import type {
    DiffLineData,
    Classification,
    FilterState,
    ReviewComment,
  } from "../types.js";

  interface Props {
    /** Which renderer to use. */
    view: "inline" | "split";
    /** The diff lines to render. */
    lines: DiffLineData[];
    /** File path — used to select the syntax highlighter. */
    filePath: string;
    /** Stable hunk identifier, forwarded to the diff renderer. */
    hunkId: string;
    /** LLM classification for the enclosing hunk (provides fallback tags). */
    classification: Classification | null;
    /** Whether the dark theme is active. */
    isDark: boolean;
    /** Active filter — non-passing add/del lines are dimmed. */
    filter?: FilterState | null;
    /** Platform-specific PR identifier for comment storage. */
    prId?: string;
    /** Draft comments that overlap this hunk. */
    comments?: ReviewComment[];
    /** Emitted when a new comment is saved. */
    onCommentAdded?: (comment: ReviewComment) => void;
    /** Emitted when an existing comment is updated. */
    onCommentUpdated?: (comment: ReviewComment) => void;
    /** Emitted when a comment is deleted. */
    onCommentDeleted?: (id: number) => void;
    /** Emitted when the user overrides the change type. */
    onOverrideCategory?: (changeType: string) => void;
    /** Emitted when the user overrides attention tags. */
    onOverrideTags?: (tags: string[]) => void;
  }

  let {
    view,
    lines,
    filePath,
    hunkId,
    classification,
    isDark,
    filter = null,
    prId = "",
    comments = [],
    onCommentAdded,
    onCommentUpdated,
    onCommentDeleted,
    onOverrideCategory,
    onOverrideTags,
  }: Props = $props();
</script>

{#if view === "split"}
  <SplitDiff
    {lines}
    {filePath}
    {hunkId}
    {classification}
    {isDark}
    {filter}
    {prId}
    {comments}
    {onCommentAdded}
    {onCommentUpdated}
    {onCommentDeleted}
    {onOverrideCategory}
    {onOverrideTags}
  />
{:else}
  <InlineDiff
    {lines}
    {filePath}
    {hunkId}
    {classification}
    {isDark}
    {filter}
    {prId}
    {comments}
    {onCommentAdded}
    {onCommentUpdated}
    {onCommentDeleted}
    {onOverrideCategory}
    {onOverrideTags}
  />
{/if}
