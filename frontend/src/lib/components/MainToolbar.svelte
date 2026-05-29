<script lang="ts">
  /**
   * Main toolbar for the review screen diff area.
   *
   * Three groups:
   * - Left: (empty for now).
   * - Center: file navigation (prev / next arrow buttons + "N of M" counter).
   * - Right: SegToggle (Unified / Split), overflow menu, keyboard-shortcuts
   *   button, and the Submit-review button.
   *
   * Translates `BMainToolbar` from `concept-b.jsx` to Svelte.
   *
   * @example
   * ```svelte
   * <MainToolbar
   *   fileIndex={0}
   *   totalFiles={5}
   *   diffView="inline"
   *   onDiffViewChange={(v) => (diffView = v)}
   *   onPrev={prev}
   *   onNext={next}
   * />
   * ```
   */

  import SegToggle from "./SegToggle.svelte";
  import IconBtn from "./IconBtn.svelte";

  interface Props {
    /** Zero-based index of the currently focused file. */
    fileIndex: number;
    /** Total number of files in the PR. */
    totalFiles: number;
    /** Current diff view mode. */
    diffView: "inline" | "split";
    /** Called when the view toggle changes. */
    onDiffViewChange?: (view: "inline" | "split") => void;
    /** Called when the user clicks "previous file". */
    onPrev?: () => void;
    /** Called when the user clicks "next file". */
    onNext?: () => void;
    /** Number of draft comments, shown in the Submit-review button label. */
    draftCount?: number;
    /** Called when the user clicks "Submit review". */
    onSubmitReview?: () => void;
  }

  let {
    fileIndex,
    totalFiles,
    diffView,
    onDiffViewChange,
    onPrev,
    onNext,
    draftCount = 0,
    onSubmitReview,
  }: Props = $props();

  const atFirst = $derived(fileIndex <= 0);
  const atLast = $derived(fileIndex >= totalFiles - 1);
</script>

<div
  style="
    flex-shrink: 0;
    padding: 10px 24px;
    border-bottom: 1px solid var(--ed-border-subtle);
    background: var(--ed-bg);
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 14px;
  "
>
  <!-- Left: empty spacer keeps the center file-nav balanced via space-between -->
  <div style="display: flex; align-items: center;"></div>

  <!-- Center: file navigation -->
  <div style="display: flex; align-items: center; gap: 10px;">
    <IconBtn title="Previous file" disabled={atFirst} onclick={onPrev}>
      <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
        <path
          d="M9 3 L4 7 L9 11"
          stroke="currentColor"
          stroke-width="1.6"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
      </svg>
    </IconBtn>
    <span
      style="font-family: var(--font-mono); font-size: 12.5px; color: var(--ed-text);"
    >
      {fileIndex + 1}
      <span style="color: var(--ed-text-faint);">of {totalFiles}</span>
    </span>
    <IconBtn title="Next file" disabled={atLast} onclick={onNext}>
      <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
        <path
          d="M5 3 L10 7 L5 11"
          stroke="currentColor"
          stroke-width="1.6"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
      </svg>
    </IconBtn>
  </div>

  <!-- Right: view toggle + overflow + keyboard shortcuts -->
  <div style="display: flex; align-items: center; gap: 8px;">
    <SegToggle
      options={[
        { value: "inline", label: "Unified" },
        { value: "split", label: "Split" },
      ]}
      value={diffView}
      onchange={(v) => onDiffViewChange?.(v as "inline" | "split")}
    />
    <IconBtn title="More options">
      <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
        <circle cx="3" cy="7" r="1.2" fill="currentColor" />
        <circle cx="7" cy="7" r="1.2" fill="currentColor" />
        <circle cx="11" cy="7" r="1.2" fill="currentColor" />
      </svg>
    </IconBtn>
    <IconBtn title="Keyboard shortcuts">
      <svg width="16" height="14" viewBox="0 0 16 14" fill="none">
        <rect
          x="1"
          y="3"
          width="14"
          height="9"
          rx="1.4"
          stroke="currentColor"
          stroke-width="1.2"
          fill="none"
        />
        <path
          d="M4 7 L4 7 M7 7 L7 7 M10 7 L10 7 M4 9.5 L11 9.5"
          stroke="currentColor"
          stroke-width="1.4"
          stroke-linecap="round"
        />
      </svg>
    </IconBtn>
    <button
      onclick={onSubmitReview}
      style="
        background: var(--ed-accent);
        border: none;
        border-radius: 6px;
        color: #fff;
        cursor: pointer;
        font-family: var(--font-sans);
        font-size: 12px;
        font-weight: 500;
        padding: 5px 12px;
        white-space: nowrap;
      "
    >
      Submit review{draftCount > 0 ? ` (${draftCount})` : ""}
    </button>
  </div>
</div>
