<script lang="ts">
  /**
   * Inspector panel — right column (340px) shown when a hunk is focused.
   *
   * Displays classification details for the focused hunk: summary, rationale,
   * category tag pill, confidence bar, status, assignee, notes, activity, and
   * actions. The close button hides the panel by clearing `focusedHunkId`.
   *
   * Translates `BInspectorPanel` from `concept-b.jsx` to Svelte.
   *
   * @example
   * ```svelte
   * <InspectorPanel
   *   file={focusedFile}
   *   hunk={focusedHunk}
   *   classification={focusedCls}
   *   isDark={true}
   *   onClose={() => focusedHunkId.set(null)}
   *   onMarkReviewed={() => toggleReviewed(focusedFile.path)}
   * />
   * ```
   */

  import TagPill from "./TagPill.svelte";
  import type { FileEntry, HunkData, Classification } from "../types.js";

  interface Props {
    /** File that owns the focused hunk. */
    file: FileEntry;
    /** The focused hunk. */
    hunk: HunkData;
    /** LLM classification for the focused hunk. */
    classification: Classification | null;
    /** Whether the dark theme is active (forwarded to TagPill). */
    isDark: boolean;
    /** Called when the user clicks the close (×) button. */
    onClose?: () => void;
    /** Called when the user clicks "Mark as reviewed" in the actions section. */
    onMarkReviewed?: () => void;
  }

  let {
    file,
    hunk,
    classification,
    isDark: _isDark,
    onClose,
    onMarkReviewed,
  }: Props = $props();

  /** Number of non-deleted lines (added + context) — drives the line range. */
  const nonDelCount = $derived(hunk.lines.filter((l) => l.type !== "del").length);
  /** End of the new-side line range shown in the header. */
  const rangeEnd = $derived(hunk.newStart + nonDelCount - 1);
  /** Display range string, e.g. "18–35". */
  const lineRange = $derived(`${hunk.newStart}–${rangeEnd}`);
  /** Number of pure additions. */
  const addedCount = $derived(hunk.lines.filter((l) => l.type === "add").length);

  const confidencePct = $derived(
    classification ? Math.round(classification.confidence * 100) : 0,
  );
  const attentionTags = $derived(classification?.attentionTags ?? []);
  const summary = $derived(classification?.summary ?? "No summary available.");
  const rationale = $derived(classification?.rationale ?? "");
</script>

<div
  style="
    overflow: auto;
    background: var(--ed-bg);
    display: flex;
    flex-direction: column;
  "
>
  <!-- Sticky header -->
  <div
    style="
      padding: 14px 20px;
      border-bottom: 1px solid var(--ed-border-subtle);
      display: flex;
      justify-content: space-between;
      align-items: center;
      position: sticky;
      top: 0;
      background: var(--ed-bg);
      z-index: 1;
    "
  >
    <div
      style="
        font-size: 14px;
        font-weight: 600;
        color: var(--ed-text);
        letter-spacing: -0.1px;
        font-family: var(--font-sans);
      "
    >
      Inspector
    </div>
    <button
      type="button"
      title="Close inspector"
      onclick={onClose}
      style="
        background: transparent;
        border: none;
        color: var(--ed-text-muted);
        cursor: pointer;
        font-size: 18px;
        line-height: 1;
        padding: 2px;
        font-family: var(--font-sans);
      "
    >
      ×
    </button>
  </div>

  <!-- Selected change -->
  <div
    style="
      padding: 16px 20px;
      border-bottom: 1px solid var(--ed-border-subtle);
    "
  >
    <div
      style="
        font-size: 13px;
        font-weight: 600;
        color: var(--ed-text);
        letter-spacing: -0.1px;
        margin-bottom: 10px;
        font-family: var(--font-sans);
      "
    >
      Selected change
    </div>
    <p
      style="
        margin: 0;
        font-size: 13.5px;
        line-height: 1.55;
        color: var(--ed-text);
        font-family: var(--font-sans);
      "
    >
      {summary}
    </p>
    <div
      style="
        margin-top: 10px;
        display: flex;
        align-items: center;
        gap: 10px;
      "
    >
      <span
        style="
          font-family: var(--font-mono);
          font-size: 12px;
          color: var(--ed-text-muted);
        "
      >
        Lines {lineRange}
      </span>
      <span
        style="
          padding: 1px 7px;
          border-radius: 4px;
          background: var(--ed-added-bg);
          color: var(--ed-added);
          font-family: var(--font-mono);
          font-size: 11px;
          font-weight: 600;
        "
      >
        +{addedCount}
      </span>
    </div>
  </div>

  <!-- Why this matters -->
  {#if rationale}
    <div
      style="
        padding: 16px 20px;
        border-bottom: 1px solid var(--ed-border-subtle);
      "
    >
      <div
        style="
          font-size: 13px;
          font-weight: 600;
          color: var(--ed-text);
          letter-spacing: -0.1px;
          margin-bottom: 10px;
          font-family: var(--font-sans);
        "
      >
        Why this matters
      </div>
      <p
        style="
          margin: 0;
          font-size: 13px;
          line-height: 1.55;
          color: var(--ed-text-muted);
          font-family: var(--font-sans);
        "
      >
        {rationale}
      </p>
    </div>
  {/if}

  <!-- Category -->
  <div
    style="
      padding: 16px 20px;
      border-bottom: 1px solid var(--ed-border-subtle);
    "
  >
    <div
      style="
        font-size: 13px;
        font-weight: 600;
        color: var(--ed-text);
        letter-spacing: -0.1px;
        margin-bottom: 10px;
        font-family: var(--font-sans);
      "
    >
      Category
    </div>
    <div style="display: flex; flex-wrap: wrap; gap: 6px;">
      {#each attentionTags.slice(0, 1) as tagId}
        <TagPill id={tagId} />
      {/each}
      {#if attentionTags.length === 0}
        <span
          style="
            font-size: 12px;
            color: var(--ed-text-faint);
            font-family: var(--font-sans);
          "
        >
          Uncategorized
        </span>
      {/if}
    </div>
  </div>

  <!-- Confidence -->
  <div
    style="
      padding: 16px 20px;
      border-bottom: 1px solid var(--ed-border-subtle);
    "
  >
    <div
      style="
        font-size: 13px;
        font-weight: 600;
        color: var(--ed-text);
        letter-spacing: -0.1px;
        margin-bottom: 10px;
        font-family: var(--font-sans);
      "
    >
      Confidence
    </div>
    <div style="display: flex; align-items: center; gap: 10px;">
      <span
        style="
          font-family: var(--font-mono);
          font-size: 13px;
          color: var(--ed-text);
          font-weight: 600;
        "
      >
        {confidencePct}%
      </span>
    </div>
    <div
      style="
        margin-top: 8px;
        height: 6px;
        border-radius: 3px;
        background: var(--ed-border-subtle);
        overflow: hidden;
      "
    >
      <div
        style="
          width: {confidencePct}%;
          height: 100%;
          background: var(--ed-added);
          border-radius: 3px;
        "
      ></div>
    </div>
  </div>

  <!-- Status row -->
  <div
    style="
      padding: 14px 20px;
      border-bottom: 1px solid var(--ed-border-subtle);
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 12px;
    "
  >
    <span
      style="
        font-size: 13px;
        color: var(--ed-text-muted);
        font-family: var(--font-sans);
      "
    >
      Status
    </span>
    <button
      type="button"
      style="
        display: inline-flex;
        align-items: center;
        gap: 6px;
        padding: 6px 10px;
        border-radius: 6px;
        background: var(--ed-bg);
        border: 1px solid var(--ed-border);
        color: var(--ed-text);
        font-size: 12.5px;
        cursor: pointer;
        font-family: var(--font-sans);
        font-weight: 500;
      "
    >
      Reviewing
      <svg width="10" height="10" viewBox="0 0 10 10" fill="none">
        <path
          d="M2 4 L5 7 L8 4"
          stroke="var(--ed-text-muted)"
          stroke-width="1.4"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
      </svg>
    </button>
  </div>

  <!-- Assignee row -->
  <div
    style="
      padding: 14px 20px;
      border-bottom: 1px solid var(--ed-border-subtle);
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 12px;
    "
  >
    <span
      style="
        font-size: 13px;
        color: var(--ed-text-muted);
        font-family: var(--font-sans);
      "
    >
      Assignee
    </span>
    <button
      type="button"
      style="
        display: inline-flex;
        align-items: center;
        gap: 8px;
        padding: 5px 10px;
        border-radius: 6px;
        background: var(--ed-bg);
        border: 1px solid var(--ed-border);
        color: var(--ed-text);
        font-size: 12.5px;
        cursor: pointer;
        font-family: var(--font-sans);
      "
    >
      <span
        style="
          width: 18px;
          height: 18px;
          border-radius: 50%;
          background: var(--ed-accent-soft);
          color: var(--ed-accent);
          font-size: 10px;
          font-weight: 700;
          display: inline-flex;
          align-items: center;
          justify-content: center;
          flex-shrink: 0;
          font-family: var(--font-sans);
        "
      >
        {file.short[0]?.toUpperCase() ?? "?"}
      </span>
      <span>Unassigned</span>
      <svg width="10" height="10" viewBox="0 0 10 10" fill="none">
        <path
          d="M2 4 L5 7 L8 4"
          stroke="var(--ed-text-muted)"
          stroke-width="1.4"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
      </svg>
    </button>
  </div>

  <!-- Notes -->
  <div
    style="
      padding: 16px 20px;
      border-bottom: 1px solid var(--ed-border-subtle);
    "
  >
    <div
      style="
        font-size: 13px;
        font-weight: 600;
        color: var(--ed-text);
        letter-spacing: -0.1px;
        margin-bottom: 10px;
        font-family: var(--font-sans);
      "
    >
      Notes
    </div>
    <div
      style="
        border: 1px solid var(--ed-border);
        border-radius: 6px;
        padding: 8px 10px;
        font-size: 13px;
        color: var(--ed-text-faint);
        font-family: var(--font-sans);
        min-height: 36px;
        background: var(--ed-bg);
      "
    >
      Add a note…
    </div>
  </div>

  <!-- Activity -->
  <div
    style="
      padding: 16px 20px;
      border-bottom: 1px solid var(--ed-border-subtle);
    "
  >
    <div
      style="
        display: flex;
        justify-content: space-between;
        align-items: center;
        gap: 10px;
        margin-bottom: 10px;
      "
    >
      <div
        style="
          font-size: 13px;
          font-weight: 600;
          color: var(--ed-text);
          letter-spacing: -0.1px;
          font-family: var(--font-sans);
          white-space: nowrap;
        "
      >
        Activity
      </div>
      <button
        type="button"
        style="
          display: inline-flex;
          align-items: center;
          gap: 6px;
          padding: 4px 8px;
          border-radius: 6px;
          background: var(--ed-bg);
          border: 1px solid var(--ed-border);
          color: var(--ed-text);
          font-size: 11.5px;
          cursor: pointer;
          font-family: var(--font-sans);
          font-weight: 500;
        "
      >
        All
        <svg width="10" height="10" viewBox="0 0 10 10" fill="none">
          <path
            d="M2 4 L5 7 L8 4"
            stroke="var(--ed-text-muted)"
            stroke-width="1.4"
            stroke-linecap="round"
            stroke-linejoin="round"
          />
        </svg>
      </button>
    </div>

    <!-- Hardcoded activity entries (Epic 9 will wire real data) -->
    <div style="display: flex; gap: 10px; padding: 6px 0; align-items: flex-start;">
      <span
        style="
          width: 22px;
          height: 22px;
          border-radius: 50%;
          background: var(--ed-accent-soft);
          color: var(--ed-accent);
          font-size: 9px;
          font-weight: 700;
          display: inline-flex;
          align-items: center;
          justify-content: center;
          flex-shrink: 0;
          font-family: var(--font-sans);
        "
      >
        LM
      </span>
      <div style="flex: 1; min-width: 0;">
        <div
          style="
            display: flex;
            justify-content: space-between;
            align-items: baseline;
            gap: 8px;
          "
        >
          <span
            style="
              font-size: 12.5px;
              font-weight: 600;
              color: var(--ed-text);
              font-family: var(--font-sans);
            "
          >
            l.mccallum
          </span>
          <span
            style="
              font-family: var(--font-mono);
              font-size: 11px;
              color: var(--ed-text-faint);
            "
          >
            just now
          </span>
        </div>
        <div
          style="
            font-size: 12.5px;
            color: var(--ed-text-muted);
            margin-top: 2px;
            line-height: 1.45;
            font-family: var(--font-sans);
          "
        >
          Opened inspector
        </div>
      </div>
    </div>
  </div>

  <!-- Actions -->
  <div style="padding: 16px 20px;">
    <div
      style="
        font-size: 13px;
        font-weight: 600;
        color: var(--ed-text);
        letter-spacing: -0.1px;
        margin-bottom: 10px;
        font-family: var(--font-sans);
      "
    >
      Actions
    </div>
    <div style="display: flex; flex-direction: column; gap: 8px;">
      <!-- Override category -->
      <button
        type="button"
        style="
          padding: 9px 12px;
          border-radius: 6px;
          background: var(--ed-bg);
          border: 1px solid var(--ed-border);
          color: var(--ed-text);
          font-size: 13px;
          cursor: pointer;
          font-weight: 500;
          display: inline-flex;
          align-items: center;
          justify-content: space-between;
          font-family: var(--font-sans);
        "
      >
        <span>Override category</span>
        <svg width="10" height="10" viewBox="0 0 10 10" fill="none">
          <path
            d="M2 4 L5 7 L8 4"
            stroke="var(--ed-text-muted)"
            stroke-width="1.4"
            stroke-linecap="round"
            stroke-linejoin="round"
          />
        </svg>
      </button>

      <!-- Mark as reviewed -->
      <button
        type="button"
        onclick={onMarkReviewed}
        style="
          padding: 9px 12px;
          border-radius: 6px;
          background: transparent;
          border: 1.5px solid var(--ed-added);
          color: var(--ed-added);
          font-size: 13px;
          cursor: pointer;
          font-weight: 600;
          font-family: var(--font-sans);
        "
      >
        Mark as reviewed
      </button>

      <!-- Create follow-up (stub) -->
      <button
        type="button"
        style="
          padding: 9px 12px;
          border-radius: 6px;
          background: transparent;
          border: 1px solid var(--ed-border);
          color: var(--ed-text-muted);
          font-size: 13px;
          cursor: pointer;
          font-weight: 500;
          font-family: var(--font-sans);
        "
      >
        Create follow-up
      </button>
    </div>
  </div>
</div>
