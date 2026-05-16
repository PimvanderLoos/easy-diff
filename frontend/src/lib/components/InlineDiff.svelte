<script lang="ts">
  /**
   * Unified (inline) diff renderer.
   *
   * Renders each diff line in a 5-column CSS Grid:
   *   `3px | 44px | 44px | 18px | 1fr`
   *   stripe | old# | new# | sign | syntax-highlighted code
   *
   * Added lines get `--ed-added-bg`, removed lines get `--ed-removed-bg`.
   * Lines that do not pass the active filter are dimmed (opacity 0.32,
   * desaturated) to draw attention to what matters.
   *
   * Clicking a line number opens an inline `CommentInput` below that line.
   * Right-clicking a line opens a `ContextMenu` with comment/override options.
   * Saved draft comments are rendered inline via `DraftComment`.
   *
   * @example
   * ```svelte
   * <InlineDiff
   *   lines={hunk.lines}
   *   filePath="src/Foo.java"
   *   hunkId="src/Foo.java:0"
   *   classification={cls}
   *   isDark={true}
   *   prId="42"
   *   comments={draftsForHunk}
   *   onCommentAdded={(c) => store.add(c)}
   *   onCommentUpdated={(c) => store.update(c)}
   *   onCommentDeleted={(id) => store.remove(id)}
   *   onOverrideCategory={(ct) => store.setOverride(ct)}
   *   onOverrideTags={(tags) => store.setTagOverride(tags)}
   * />
   * ```
   */

  import LineStripe from "./LineStripe.svelte";
  import CommentInput from "./CommentInput.svelte";
  import DraftComment from "./DraftComment.svelte";
  import ContextMenu from "./ContextMenu.svelte";
  import { highlight } from "../syntax.js";
  import { lineTags, linePasses } from "../filters.js";
  import type {
    DiffLineData,
    Classification,
    FilterState,
    ReviewComment,
  } from "../types.js";

  interface Props {
    /** The diff lines to render. */
    lines: DiffLineData[];
    /** File path — used to select the syntax highlighter. */
    filePath: string;
    /** Stable hunk identifier, used as key for comment anchoring. */
    hunkId: string;
    /** LLM classification for the enclosing hunk (provides fallback tags). */
    classification: Classification | null;
    /** Whether the dark theme is active. */
    isDark: boolean;
    /** Active filter — lines that don't match are dimmed. */
    filter?: FilterState | null;
    /** Platform-specific PR identifier for comment storage. */
    prId?: string;
    /** Draft comments that overlap this hunk. */
    comments?: ReviewComment[];
    /** Emitted when a new comment is saved. */
    onCommentAdded?: (comment: ReviewComment) => void;
    /** Emitted when an existing comment is updated. */
    onCommentUpdated?: (comment: ReviewComment) => void;
    /** Emitted when a comment is deleted (provides the comment id). */
    onCommentDeleted?: (id: number) => void;
    /** Emitted when the user overrides the change type from the context menu. */
    onOverrideCategory?: (changeType: string) => void;
    /** Emitted when the user overrides attention tags from the context menu. */
    onOverrideTags?: (tags: string[]) => void;
  }

  let {
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

  /** Which line index (within this hunk) has an open comment input. null = none. */
  let activeCommentLine = $state<number | null>(null);

  /** Context menu state. */
  let contextMenu = $state<{
    x: number;
    y: number;
    lineIndex: number;
  } | null>(null);

  function bg(line: DiffLineData): string {
    if (line.type === "add") return "var(--ed-added-bg)";
    if (line.type === "del") return "var(--ed-removed-bg)";
    return "transparent";
  }

  function sign(line: DiffLineData): string {
    if (line.type === "add") return "+";
    if (line.type === "del") return "−";
    return " ";
  }

  function signColor(line: DiffLineData): string {
    if (line.type === "add") return "var(--ed-added)";
    if (line.type === "del") return "var(--ed-removed)";
    return "var(--ed-text-faint)";
  }

  /** Returns the 1-based line number to use as comment anchor for a given line. */
  function anchorLine(line: DiffLineData): number {
    return line.new ?? line.old ?? 0;
  }

  function handleLineNumberClick(lineIndex: number) {
    activeCommentLine = activeCommentLine === lineIndex ? null : lineIndex;
    contextMenu = null;
  }

  function handleContextMenu(e: MouseEvent, lineIndex: number) {
    e.preventDefault();
    contextMenu = { x: e.clientX, y: e.clientY, lineIndex };
    activeCommentLine = null;
  }

  function handleContextAddComment() {
    if (contextMenu !== null) {
      activeCommentLine = contextMenu.lineIndex;
    }
  }

  /** Compute which line numbers have draft comments attached. */
  function commentsForLine(lineNum: number): ReviewComment[] {
    return comments.filter(
      (c) => c.start_line === lineNum || (c.end_line !== null && c.end_line >= lineNum && c.start_line <= lineNum),
    );
  }
</script>

<!-- Suppress context menu on the whole renderer; individual lines handle right-click. -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  style="font-family: var(--font-mono); font-size: 12.5px; line-height: 1.7;"
  oncontextmenu={(e) => e.preventDefault()}
>
  {#each lines as line, i (i)}
    {@const tags = lineTags(line, classification)}
    {@const dim = !linePasses(line, classification, filter) && line.type !== "ctx"}
    {@const lineNum = anchorLine(line)}
    {@const lineComments = lineNum > 0 ? commentsForLine(lineNum) : []}

    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      oncontextmenu={(e) => handleContextMenu(e, i)}
    >
      <div
        style="
          display: grid;
          grid-template-columns: 3px 44px 44px 18px 1fr;
          background: {bg(line)};
          align-items: stretch;
          opacity: {dim ? 0.32 : 1};
          filter: {dim ? 'saturate(0.65)' : 'none'};
          transition: opacity 0.15s, filter 0.15s;
        "
      >
        <LineStripe {tags} {isDark} />

        <!-- old line number (clickable) -->
        <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
        <span
          role="button"
          tabindex={-1}
          onclick={() => handleLineNumberClick(i)}
          onkeydown={(e) => e.key === "Enter" && handleLineNumberClick(i)}
          title="Click to add comment"
          style="
            text-align: right;
            padding-right: 8px;
            color: var(--ed-text-faint);
            user-select: none;
            font-size: 11px;
            line-height: 1.7;
            cursor: pointer;
          "
        >
          {line.old ?? ""}
        </span>

        <!-- new line number (clickable) -->
        <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
        <span
          role="button"
          tabindex={-1}
          onclick={() => handleLineNumberClick(i)}
          onkeydown={(e) => e.key === "Enter" && handleLineNumberClick(i)}
          title="Click to add comment"
          style="
            text-align: right;
            padding-right: 8px;
            color: var(--ed-text-faint);
            user-select: none;
            font-size: 11px;
            line-height: 1.7;
            cursor: pointer;
          "
        >
          {line.new ?? ""}
        </span>

        <!-- +/−/space sign -->
        <span
          style="
            text-align: center;
            color: {signColor(line)};
            user-select: none;
            line-height: 1.7;
          "
        >
          {sign(line)}
        </span>

        <!-- syntax-highlighted code -->
        <span style="padding-right: 8px; color: var(--ed-text); line-height: 1.7;">
          {#each highlight(filePath, line.text) as tok (tok)}
            {@const tokenColor =
              tok.c === "ws" || tok.c === "ident" || tok.c === "punct"
                ? "inherit"
                : `var(--ed-sx-${tok.c}, inherit)`}
            <span style="color: {tokenColor}; white-space: pre;">{tok.t}</span>
          {/each}
        </span>
      </div>

      <!-- Draft comments anchored to this line -->
      {#each lineComments as comment (comment.id)}
        <DraftComment
          {comment}
          onUpdated={(c) => onCommentUpdated?.(c)}
          onDeleted={(id) => onCommentDeleted?.(id)}
        />
      {/each}

      <!-- Comment input below this line (when active) -->
      {#if activeCommentLine === i && prId}
        <CommentInput
          {prId}
          {filePath}
          startLine={lineNum}
          onSaved={(c) => {
            onCommentAdded?.(c);
            activeCommentLine = null;
          }}
          onCancel={() => (activeCommentLine = null)}
        />
      {/if}
    </div>
  {/each}
</div>

<!-- Context menu (portal-style fixed position) -->
{#if contextMenu !== null}
  <ContextMenu
    x={contextMenu.x}
    y={contextMenu.y}
    {hunkId}
    currentChangeType={classification?.changeType ?? "uncategorized"}
    currentTags={classification?.attentionTags ?? []}
    onAddComment={handleContextAddComment}
    onOverrideCategory={(ct) => {
      onOverrideCategory?.(ct);
      contextMenu = null;
    }}
    onOverrideTags={(tags) => {
      onOverrideTags?.(tags);
      contextMenu = null;
    }}
    onClose={() => (contextMenu = null)}
  />
{/if}
