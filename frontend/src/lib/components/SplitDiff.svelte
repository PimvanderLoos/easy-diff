<script lang="ts">
  /**
   * Side-by-side (split) diff renderer.
   *
   * Renders adjacent del/add lines paired in a two-column grid. Each column
   * has its own 4-column sub-grid: `3px | 36px | 14px | 1fr`
   *   stripe | line# | sign | syntax-highlighted code
   *
   * Line pairing logic:
   *   - del immediately followed by add → paired (left=del, right=add)
   *   - del only → left=del, right=empty
   *   - add only → left=empty, right=add
   *   - ctx → both sides show the same line
   *
   * Non-passing add/del lines are dimmed (opacity 0.32, saturate(0.65)).
   * Context lines always show at full opacity.
   *
   * Clicking a line number opens an inline `CommentInput` below that row.
   * Right-clicking a line opens a `ContextMenu` with comment/override options.
   * Saved draft comments are rendered inline via `DraftComment`.
   *
   * @example
   * ```svelte
   * <SplitDiff
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

  interface Row {
    left: DiffLineData | null;
    right: DiffLineData | null;
  }

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

  /** Pair adjacent del/add lines; ctx lines appear on both sides. */
  const rows = $derived.by<Row[]>(() => {
    const out: Row[] = [];
    let i = 0;
    while (i < lines.length) {
      const a = lines[i];
      if (
        a.type === "del" &&
        i + 1 < lines.length &&
        lines[i + 1].type === "add"
      ) {
        out.push({ left: a, right: lines[i + 1] });
        i += 2;
      } else if (a.type === "del") {
        out.push({ left: a, right: null });
        i++;
      } else if (a.type === "add") {
        out.push({ left: null, right: a });
        i++;
      } else {
        // ctx — same line on both sides
        out.push({ left: a, right: a });
        i++;
      }
    }
    return out;
  });

  /** Which row index has an open comment input. null = none. */
  let activeCommentRow = $state<number | null>(null);

  /** Context menu state. */
  let contextMenu = $state<{ x: number; y: number; rowIndex: number } | null>(
    null,
  );

  function bg(line: DiffLineData | null): string {
    if (!line) return "transparent";
    if (line.type === "add") return "var(--ed-added-bg)";
    if (line.type === "del") return "var(--ed-removed-bg)";
    return "transparent";
  }

  function sign(line: DiffLineData | null): string {
    if (!line) return "";
    if (line.type === "add") return "+";
    if (line.type === "del") return "−";
    return " ";
  }

  function signColor(line: DiffLineData | null): string {
    if (!line) return "transparent";
    if (line.type === "add") return "var(--ed-added)";
    if (line.type === "del") return "var(--ed-removed)";
    return "var(--ed-text-faint)";
  }

  function lineNum(line: DiffLineData | null, side: "left" | "right"): string {
    if (!line) return "";
    const n = side === "left" ? line.old : line.new;
    return n !== null ? String(n) : "";
  }

  function isDim(line: DiffLineData | null): boolean {
    if (!line) return false;
    if (line.type === "ctx") return false;
    return !linePasses(line, classification, filter);
  }

  /** Determine the best anchor line number for a row (prefer new, fall back to old). */
  function anchorLineForRow(row: Row): number {
    const line = row.right ?? row.left;
    if (!line) return 0;
    return line.new ?? line.old ?? 0;
  }

  function handleLineNumberClick(rowIndex: number) {
    activeCommentRow = activeCommentRow === rowIndex ? null : rowIndex;
    contextMenu = null;
  }

  function handleContextMenu(e: MouseEvent, rowIndex: number) {
    e.preventDefault();
    contextMenu = { x: e.clientX, y: e.clientY, rowIndex };
    activeCommentRow = null;
  }

  function handleContextAddComment() {
    if (contextMenu !== null) {
      activeCommentRow = contextMenu.rowIndex;
    }
  }

  function commentsForLine(lineNum: number): ReviewComment[] {
    if (lineNum <= 0) return [];
    return comments.filter(
      (c) =>
        c.start_line === lineNum ||
        (c.end_line !== null &&
          c.end_line >= lineNum &&
          c.start_line <= lineNum),
    );
  }
</script>

<!--
  Two-column outer grid. A 1px border-right separates old (left) from new (right).
  Each column is its own 4-column sub-grid for alignment.

  Comments and the CommentInput span full width below each row, so they live
  *outside* the two-column grid — they are placed in a wrapper div that contains
  both the row grid and any attached comments.
-->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  style="font-family: var(--font-mono); font-size: 12.5px;"
  oncontextmenu={(e) => e.preventDefault()}
>
  {#each rows as row, i (i)}
    {@const anchor = anchorLineForRow(row)}
    {@const rowComments = commentsForLine(anchor)}

    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div oncontextmenu={(e) => handleContextMenu(e, i)}>
      <!-- The two-column row -->
      <div style="display: grid; grid-template-columns: 1fr 1fr;">
        <!-- Left (old/del) column -->
        <div style="border-right: 1px solid var(--ed-border-subtle);">
          {#if row.left}
            {@const tags = lineTags(row.left, classification)}
            {@const dim = isDim(row.left)}
            <div
              style="
                display: grid;
                grid-template-columns: 3px 36px 14px 1fr;
                background: {bg(row.left)};
                align-items: stretch;
                opacity: {dim ? 0.32 : 1};
                filter: {dim ? 'saturate(0.65)' : 'none'};
                transition: opacity 0.15s, filter 0.15s;
              "
            >
              <LineStripe {tags} {isDark} />
              <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
              <span
                role="button"
                tabindex={-1}
                onclick={() => handleLineNumberClick(i)}
                onkeydown={(e) => e.key === "Enter" && handleLineNumberClick(i)}
                title="Click to add comment"
                style="
                  text-align: right;
                  padding-right: 6px;
                  color: var(--ed-text-faint);
                  font-size: 11px;
                  line-height: 1.7;
                  user-select: none;
                  cursor: pointer;
                "
              >
                {lineNum(row.left, "left")}
              </span>
              <span
                style="
                  text-align: center;
                  color: {signColor(row.left)};
                  line-height: 1.7;
                  user-select: none;
                "
              >
                {sign(row.left)}
              </span>
              <span style="color: var(--ed-text); line-height: 1.7;">
                {#each highlight(filePath, row.left.text) as tok (tok)}
                  {@const tokenColor =
                    tok.c === "ws" || tok.c === "ident" || tok.c === "punct"
                      ? "inherit"
                      : `var(--ed-sx-${tok.c}, inherit)`}
                  <span style="color: {tokenColor}; white-space: pre;"
                    >{tok.t}</span
                  >
                {/each}
              </span>
            </div>
          {:else}
            <!-- Empty left cell when the right side has an add-only line -->
            <div
              style="
                display: grid;
                grid-template-columns: 3px 36px 14px 1fr;
                line-height: 1.7;
              "
            >
              <span></span>
              <span></span>
              <span></span>
              <span></span>
            </div>
          {/if}
        </div>

        <!-- Right (new/add) column -->
        <div>
          {#if row.right}
            {@const tags = lineTags(row.right, classification)}
            {@const dim = isDim(row.right)}
            <div
              style="
                display: grid;
                grid-template-columns: 3px 36px 14px 1fr;
                background: {bg(row.right)};
                align-items: stretch;
                opacity: {dim ? 0.32 : 1};
                filter: {dim ? 'saturate(0.65)' : 'none'};
                transition: opacity 0.15s, filter 0.15s;
              "
            >
              <LineStripe {tags} {isDark} />
              <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
              <span
                role="button"
                tabindex={-1}
                onclick={() => handleLineNumberClick(i)}
                onkeydown={(e) => e.key === "Enter" && handleLineNumberClick(i)}
                title="Click to add comment"
                style="
                  text-align: right;
                  padding-right: 6px;
                  color: var(--ed-text-faint);
                  font-size: 11px;
                  line-height: 1.7;
                  user-select: none;
                  cursor: pointer;
                "
              >
                {lineNum(row.right, "right")}
              </span>
              <span
                style="
                  text-align: center;
                  color: {signColor(row.right)};
                  line-height: 1.7;
                  user-select: none;
                "
              >
                {sign(row.right)}
              </span>
              <span style="color: var(--ed-text); line-height: 1.7;">
                {#each highlight(filePath, row.right.text) as tok (tok)}
                  {@const tokenColor =
                    tok.c === "ws" || tok.c === "ident" || tok.c === "punct"
                      ? "inherit"
                      : `var(--ed-sx-${tok.c}, inherit)`}
                  <span style="color: {tokenColor}; white-space: pre;"
                    >{tok.t}</span
                  >
                {/each}
              </span>
            </div>
          {:else}
            <!-- Empty right cell when the left side has a del-only line -->
            <div
              style="
                display: grid;
                grid-template-columns: 3px 36px 14px 1fr;
                line-height: 1.7;
              "
            >
              <span></span>
              <span></span>
              <span></span>
              <span></span>
            </div>
          {/if}
        </div>
      </div>

      <!-- Draft comments spanning both columns -->
      {#each rowComments as comment (comment.id)}
        <DraftComment
          {comment}
          onUpdated={(c) => onCommentUpdated?.(c)}
          onDeleted={(id) => onCommentDeleted?.(id)}
        />
      {/each}

      <!-- Comment input spanning both columns -->
      {#if activeCommentRow === i && prId}
        <CommentInput
          {prId}
          {filePath}
          startLine={anchor}
          onSaved={(c) => {
            onCommentAdded?.(c);
            activeCommentRow = null;
          }}
          onCancel={() => (activeCommentRow = null)}
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
