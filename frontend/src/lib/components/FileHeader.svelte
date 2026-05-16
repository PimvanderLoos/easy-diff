<script lang="ts">
  /**
   * Clickable header row for a FilePanel.
   *
   * The entire header is a collapse/expand toggle. The StatusPill and kebab
   * menu stop click propagation so they remain independent actions.
   *
   * Translates `BFileHeader` from `concept-b.jsx` to Svelte.
   *
   * @example
   * ```svelte
   * <FileHeader
   *   file={f}
   *   reviewed={reviewedSet.has(f.path)}
   *   collapsed={collapsedSet.has(f.path)}
   *   onToggleCollapsed={() => toggleCollapsed(f.path)}
   *   onToggleReviewed={() => toggleReviewed(f.path)}
   * />
   * ```
   */

  import StatusPill from "./StatusPill.svelte";
  import type { FileEntry } from "../types.js";

  interface Props {
    /** File metadata. */
    file: FileEntry;
    /** Whether the file is currently marked as reviewed. */
    reviewed: boolean;
    /** Whether the panel is collapsed. */
    collapsed: boolean;
    /** Called to toggle collapse state. */
    onToggleCollapsed?: () => void;
    /** Called to toggle reviewed state. */
    onToggleReviewed?: () => void;
  }

  let {
    file,
    reviewed,
    collapsed,
    onToggleCollapsed,
    onToggleReviewed,
  }: Props = $props();

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      onToggleCollapsed?.();
    }
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  role="button"
  tabindex="0"
  onclick={onToggleCollapsed}
  onkeydown={handleKeydown}
  style="
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 0 16px;
    height: 60px;
    flex-shrink: 0;
    border-bottom: {collapsed ? 'none' : '1px solid var(--ed-border-subtle)'};
    background: var(--ed-bg);
    cursor: pointer;
    user-select: none;
  "
>
  <!-- Collapse chevron -->
  <svg
    width="12"
    height="12"
    viewBox="0 0 12 12"
    fill="none"
    style="
      transform: {collapsed ? 'rotate(-90deg)' : 'rotate(0deg)'};
      transition: transform 0.15s ease;
      flex-shrink: 0;
    "
  >
    <path
      d="M2.5 4 L6 7.5 L9.5 4"
      stroke="var(--ed-text-muted)"
      stroke-width="1.6"
      stroke-linecap="round"
      stroke-linejoin="round"
    />
  </svg>

  <!-- File icon -->
  <svg
    width="20"
    height="22"
    viewBox="0 0 20 22"
    fill="none"
    style="flex-shrink: 0;"
  >
    <path
      d="M3 2 H12 L17 7 V20 H3 Z"
      stroke="var(--ed-text-muted)"
      stroke-width="1.3"
      stroke-linejoin="round"
      fill="var(--ed-panel)"
    />
    <path
      d="M12 2 V7 H17"
      stroke="var(--ed-text-muted)"
      stroke-width="1.3"
      stroke-linejoin="round"
      fill="none"
    />
  </svg>

  <!-- File name + directory -->
  <div style="flex: 1; min-width: 0;">
    <div
      style="
        font-size: 14px;
        font-weight: 600;
        color: var(--ed-text);
        letter-spacing: -0.1px;
        font-family: var(--font-sans);
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
      "
    >
      {file.short}
    </div>
    <div
      style="
        margin-top: 2px;
        font-family: var(--font-mono);
        font-size: 11.5px;
        color: var(--ed-text-faint);
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
      "
    >
      {file.dir}/
    </div>
  </div>

  <!-- +/- stats -->
  <div
    style="
      font-family: var(--font-mono);
      font-size: 12.5px;
      display: flex;
      gap: 8px;
      flex-shrink: 0;
    "
  >
    <span style="color: var(--ed-added);">+{file.adds}</span>
    <span style="color: var(--ed-removed);">−{file.dels}</span>
  </div>

  <!-- Status pill — stops propagation so it doesn't toggle collapse -->
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div onclick={(e) => e.stopPropagation()}>
    <StatusPill {reviewed} onclick={onToggleReviewed} />
  </div>

  <!-- Kebab menu (stub) -->
  <button
    type="button"
    title="More options"
    onclick={(e) => e.stopPropagation()}
    style="
      background: transparent;
      border: none;
      color: var(--ed-text-muted);
      cursor: pointer;
      padding: 4px;
      line-height: 1;
      flex-shrink: 0;
    "
  >
    <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
      <circle cx="7" cy="3" r="1.2" fill="currentColor" />
      <circle cx="7" cy="7" r="1.2" fill="currentColor" />
      <circle cx="7" cy="11" r="1.2" fill="currentColor" />
    </svg>
  </button>
</div>
