<script lang="ts">
  /**
   * Inline comment textarea that appears below a clicked diff line.
   *
   * Shows an auto-focused textarea with Save and Cancel buttons.
   * On Save, calls the `add_comment` Tauri command and emits `onSaved`
   * with the resulting `ReviewComment`. On Cancel, emits `onCancel`.
   *
   * @example
   * ```svelte
   * <CommentInput
   *   prId="42"
   *   filePath="src/Foo.java"
   *   startLine={10}
   *   onSaved={(c) => console.log(c)}
   *   onCancel={() => {}}
   * />
   * ```
   */

  import { invoke } from "@tauri-apps/api/core";
  import type { ReviewComment } from "../types.js";

  interface Props {
    /** Platform-specific PR identifier. */
    prId: string;
    /** Relative file path being commented on. */
    filePath: string;
    /** First (or only) line of the comment anchor (1-based). */
    startLine: number;
    /** Last line of the comment anchor, or undefined for single-line. */
    endLine?: number;
    /** Called with the saved comment on successful save. */
    onSaved: (comment: ReviewComment) => void;
    /** Called when the user cancels without saving. */
    onCancel: () => void;
  }

  let { prId, filePath, startLine, endLine, onSaved, onCancel }: Props =
    $props();

  let body = $state("");
  let saving = $state(false);
  let error = $state<string | null>(null);

  /** Auto-focus the textarea when the component mounts. */
  function focusTextarea(el: HTMLTextAreaElement) {
    el.focus();
  }

  async function save() {
    const trimmed = body.trim();
    if (!trimmed) return;

    saving = true;
    error = null;
    try {
      const saved = await invoke<ReviewComment>("add_comment", {
        prId,
        filePath,
        startLine,
        endLine: endLine ?? null,
        body: trimmed,
      });
      onSaved(saved);
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    // Ctrl/Cmd+Enter saves, Escape cancels.
    if ((e.ctrlKey || e.metaKey) && e.key === "Enter") {
      e.preventDefault();
      save();
    } else if (e.key === "Escape") {
      e.preventDefault();
      onCancel();
    }
  }
</script>

<div
  style="
    background: var(--ed-panel);
    border: 1px solid var(--ed-border);
    border-radius: 6px;
    padding: 10px 12px;
    margin: 4px 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
    font-family: var(--font-sans);
  "
>
  <textarea
    use:focusTextarea
    bind:value={body}
    onkeydown={handleKeydown}
    placeholder="Leave a comment… (Ctrl+Enter to save, Esc to cancel)"
    rows={3}
    style="
      width: 100%;
      box-sizing: border-box;
      resize: vertical;
      background: var(--ed-bg);
      color: var(--ed-text);
      border: 1px solid var(--ed-border);
      border-radius: 4px;
      padding: 6px 8px;
      font-family: var(--font-sans);
      font-size: 13px;
      line-height: 1.5;
      outline: none;
    "
  ></textarea>

  {#if error}
    <div style="color: var(--ed-removed); font-size: 12px;">{error}</div>
  {/if}

  <div style="display: flex; gap: 8px; justify-content: flex-end;">
    <button
      onclick={onCancel}
      style="
        padding: 4px 12px;
        border-radius: 4px;
        border: 1px solid var(--ed-border);
        background: transparent;
        color: var(--ed-text-muted);
        font-size: 12px;
        cursor: pointer;
      "
    >
      Cancel
    </button>
    <button
      onclick={save}
      disabled={saving || !body.trim()}
      style="
        padding: 4px 12px;
        border-radius: 4px;
        border: none;
        background: var(--ed-accent, oklch(0.55 0.18 250));
        color: #fff;
        font-size: 12px;
        cursor: {saving || !body.trim() ? 'not-allowed' : 'pointer'};
        opacity: {saving || !body.trim() ? 0.55 : 1};
      "
    >
      {saving ? "Saving…" : "Save"}
    </button>
  </div>
</div>
