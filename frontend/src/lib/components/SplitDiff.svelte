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
   * Translates `EDSplitDiff` from `shared.jsx` to Svelte.
   *
   * @example
   * ```svelte
   * <SplitDiff
   *   lines={hunk.lines}
   *   filePath="src/Foo.java"
   *   classification={cls}
   *   isDark={true}
   *   filter={$filterState}
   * />
   * ```
   */

  import LineStripe from "./LineStripe.svelte";
  import { highlight } from "../syntax.js";
  import { lineTags, linePasses } from "../filters.js";
  import type { DiffLineData, Classification, FilterState } from "../types.js";

  interface Row {
    left: DiffLineData | null;
    right: DiffLineData | null;
  }

  interface Props {
    /** The diff lines to render. */
    lines: DiffLineData[];
    /** File path — used to select the syntax highlighter. */
    filePath: string;
    /** LLM classification for the enclosing hunk (provides fallback tags). */
    classification: Classification | null;
    /** Whether the dark theme is active. */
    isDark: boolean;
    /** Active filter — lines that don't match are dimmed. */
    filter?: FilterState | null;
  }

  let { lines, filePath, classification, isDark, filter = null }: Props =
    $props();

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
</script>

<!--
  Two-column outer grid. A 1px border-right separates old (left) from new (right).
  Each column is its own 4-column sub-grid for alignment.
-->
<div
  style="
    font-family: var(--font-mono);
    font-size: 12.5px;
    display: grid;
    grid-template-columns: 1fr 1fr;
  "
>
  <!-- Left (old/del) column -->
  <div style="border-right: 1px solid var(--ed-border-subtle);">
    {#each rows as row, i (i)}
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
          <span
            style="
              text-align: right;
              padding-right: 6px;
              color: var(--ed-text-faint);
              font-size: 11px;
              line-height: 1.7;
              user-select: none;
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
              <span style="color: {tokenColor}; white-space: pre;">{tok.t}</span>
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
    {/each}
  </div>

  <!-- Right (new/add) column -->
  <div>
    {#each rows as row, i (i)}
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
          <span
            style="
              text-align: right;
              padding-right: 6px;
              color: var(--ed-text-faint);
              font-size: 11px;
              line-height: 1.7;
              user-select: none;
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
              <span style="color: {tokenColor}; white-space: pre;">{tok.t}</span>
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
    {/each}
  </div>
</div>
