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
   * Translates `EDInlineDiff` from `shared.jsx` to Svelte.
   *
   * @example
   * ```svelte
   * <InlineDiff
   *   lines={hunk.lines}
   *   filePath="src/Foo.java"
   *   classification={cls}
   *   isDark={true}
   * />
   * ```
   */

  import LineStripe from "./LineStripe.svelte";
  import { highlight } from "../syntax.js";
  import type { DiffLineData, Classification, FilterState } from "../types.js";

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

  /** Effective attention tags for a line: explicit overrides first, then hunk-level. */
  function lineTags(line: DiffLineData): string[] {
    if (line.tags && line.tags.length > 0) return line.tags;
    return classification?.attentionTags ?? [];
  }

  /** Whether a line passes the active filter. Context lines always pass. */
  function passes(line: DiffLineData): boolean {
    if (!filter) return true;
    if (line.type === "ctx") return true;
    if (filter.changeType && filter.changeType !== "all") {
      if ((classification?.changeType ?? "uncategorized") !== filter.changeType)
        return false;
    }
    if (filter.attentionTags && filter.attentionTags.length > 0) {
      const tags = lineTags(line);
      if (!tags.some((t) => filter!.attentionTags.includes(t))) return false;
    }
    return true;
  }

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
</script>

<div style="font-family: var(--font-mono); font-size: 12.5px; line-height: 1.7;">
  {#each lines as line, i (i)}
    {@const tags = lineTags(line)}
    {@const dim = !passes(line) && line.type !== "ctx"}
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
      <!-- old line number -->
      <span
        style="
          text-align: right;
          padding-right: 8px;
          color: var(--ed-text-faint);
          user-select: none;
          font-size: 11px;
          line-height: 1.7;
        "
      >
        {line.old ?? ""}
      </span>
      <!-- new line number -->
      <span
        style="
          text-align: right;
          padding-right: 8px;
          color: var(--ed-text-faint);
          user-select: none;
          font-size: 11px;
          line-height: 1.7;
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
  {/each}
</div>
