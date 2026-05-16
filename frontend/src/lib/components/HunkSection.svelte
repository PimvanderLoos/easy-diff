<script lang="ts">
  /**
   * A single hunk within a FilePanel.
   *
   * Renders a header strip showing the line range (`@@ L18-L35`), attention-tag
   * pills, and confidence percentage. Below the header: the InlineDiff (or a
   * stub when the diff is hidden). When `focused` is true, the header gets an
   * accent left-border and soft background — used by the inspector panel.
   *
   * @example
   * ```svelte
   * <HunkSection
   *   hunk={h}
   *   classification={cls}
   *   filePath="src/Foo.java"
   *   isFirst={i === 0}
   *   isDark={true}
   *   prId="42"
   *   comments={draftsForHunk}
   * />
   * ```
   */

  import TagPill from "./TagPill.svelte";
  import DiffView from "./DiffView.svelte";
  import type {
    HunkData,
    Classification,
    FilterState,
    ReviewComment,
  } from "../types.js";

  interface Props {
    /** The hunk to render. */
    hunk: HunkData;
    /** LLM classification for this hunk. */
    classification: Classification | null;
    /** File path — forwarded to DiffView for syntax highlighting. */
    filePath: string;
    /** Whether this is the first hunk in a file (suppresses the top border). */
    isFirst?: boolean;
    /** Whether this hunk is currently selected in the inspector. */
    focused?: boolean;
    /** Whether the hunk body (diff lines) is hidden (e.g. file is collapsed). */
    bodyHidden?: boolean;
    /** Whether the dark theme is active. */
    isDark: boolean;
    /** Active diff filter. */
    filter?: FilterState | null;
    /** Diff view mode: inline (unified) or split (side-by-side). */
    diffView?: "inline" | "split";
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
    /** Emitted when the user overrides the change type from the context menu. */
    onOverrideCategory?: (changeType: string) => void;
    /** Emitted when the user overrides attention tags from the context menu. */
    onOverrideTags?: (tags: string[]) => void;
    /** Called when the user clicks the hunk header to focus it in the inspector. */
    onFocus?: () => void;
  }

  let {
    hunk,
    classification,
    filePath,
    isFirst = false,
    focused = false,
    bodyHidden = false,
    isDark,
    filter = null,
    diffView = "inline",
    prId = "",
    comments = [],
    onCommentAdded,
    onCommentUpdated,
    onCommentDeleted,
    onOverrideCategory,
    onOverrideTags,
    onFocus,
  }: Props = $props();

  /** Build "@@ L18-L35" label from the hunk's new-side line range. */
  const label = $derived(() => {
    const nonDel = hunk.lines.filter((l) => l.type !== "del").length;
    const end = hunk.newStart + nonDel - 1;
    return `@@ L${hunk.newStart}-L${end}`;
  });

  const confidencePct = $derived(
    classification ? Math.round(classification.confidence * 100) : null,
  );

  const attentionTags = $derived(classification?.attentionTags ?? []);
</script>

<div style="border-top: {isFirst ? 'none' : '1px solid var(--ed-border-subtle)'};">
  <!-- Hunk header strip — click to focus in inspector -->
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    role="button"
    tabindex="0"
    onclick={onFocus}
    onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onFocus?.(); } }}
    style="
      padding: 8px 16px;
      background: {focused ? 'var(--ed-accent-soft)' : 'var(--ed-panel)'};
      border-bottom: {bodyHidden ? 'none' : '1px solid var(--ed-border-subtle)'};
      display: flex;
      align-items: center;
      gap: 12px;
      border-left: 2px solid {focused ? 'var(--ed-accent)' : 'transparent'};
      cursor: pointer;
      user-select: none;
    "
  >
    <span
      style="
        font-family: var(--font-mono);
        font-size: 12px;
        color: {focused ? 'var(--ed-accent)' : 'var(--ed-text-muted)'};
        white-space: nowrap;
      "
    >
      {label()}
    </span>

    <!-- Attention tag pills -->
    <div
      style="display: flex; align-items: center; gap: 6px; flex: 1; flex-wrap: wrap;"
    >
      {#each attentionTags as tagId (tagId)}
        <TagPill id={tagId} />
      {/each}
    </div>

    <!-- Confidence percentage -->
    {#if confidencePct !== null}
      <span
        style="
          font-family: var(--font-mono);
          font-size: 11.5px;
          color: var(--ed-text-faint);
          flex-shrink: 0;
        "
      >
        {confidencePct}%
      </span>
    {/if}
  </div>

  <!-- Diff body -->
  {#if !bodyHidden}
    <div style="padding: 4px 0; background: var(--ed-bg);">
      <DiffView
        view={diffView}
        lines={hunk.lines}
        {filePath}
        hunkId={hunk.id}
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
    </div>
  {/if}
</div>
