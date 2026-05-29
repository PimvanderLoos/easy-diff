<script lang="ts">
  /**
   * Displays a saved draft review comment inline between diff lines.
   *
   * Shows the comment body in a yellow-tinted block with Edit and Delete
   * action buttons. Clicking Edit switches to an editable textarea in place.
   * Clicking Delete calls the `delete_comment` Tauri command and emits
   * `onDeleted`.
   *
   * @example
   * ```svelte
   * <DraftComment
   *   comment={myComment}
   *   onUpdated={(c) => updateStore(c)}
   *   onDeleted={(id) => removeFromStore(id)}
   * />
   * ```
   */

  import { invoke } from "@tauri-apps/api/core";
  import type { ReviewComment } from "../types.js";

  interface Props {
    /** The draft comment to display. */
    comment: ReviewComment;
    /** Called with the updated comment after an in-place edit. */
    onUpdated: (comment: ReviewComment) => void;
    /** Called with the deleted comment's id after deletion. */
    onDeleted: (id: number) => void;
  }

  let { comment, onUpdated, onDeleted }: Props = $props();

  let editing = $state(false);
  let editBody = $state("");
  let saving = $state(false);
  let deleting = $state(false);
  let saveError = $state<string | null>(null);

  function startEdit() {
    editBody = comment.body;
    editing = true;
  }

  function cancelEdit() {
    editing = false;
    saveError = null;
  }

  async function saveEdit() {
    const trimmed = editBody.trim();
    if (!trimmed) return;

    saving = true;
    saveError = null;
    try {
      await invoke("update_comment", { id: comment.id, body: trimmed });
      onUpdated({ ...comment, body: trimmed });
      editing = false;
    } catch (e) {
      saveError = String(e);
    } finally {
      saving = false;
    }
  }

  async function handleDelete() {
    deleting = true;
    try {
      await invoke("delete_comment", { id: comment.id });
      onDeleted(comment.id);
    } finally {
      deleting = false;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key === "Enter") {
      e.preventDefault();
      saveEdit();
    } else if (e.key === "Escape") {
      e.preventDefault();
      cancelEdit();
    }
  }

  function focusTextarea(el: HTMLTextAreaElement) {
    el.focus();
  }
</script>

<div
  style="
    background: oklch(0.97 0.06 80 / 0.18);
    border-left: 3px solid oklch(0.75 0.14 80);
    border-radius: 0 4px 4px 0;
    padding: 8px 12px;
    margin: 4px 0;
    font-family: var(--font-sans);
    font-size: 13px;
  "
>
  {#if editing}
    <!-- Edit mode: textarea + save/cancel buttons -->
    <textarea
      use:focusTextarea
      bind:value={editBody}
      onkeydown={handleKeydown}
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
        margin-bottom: 6px;
      "
    ></textarea>

    {#if saveError}
      <div style="color: var(--ed-removed); font-size: 12px; margin-bottom: 6px;">
        {saveError}
      </div>
    {/if}

    <div style="display: flex; gap: 6px; justify-content: flex-end;">
      <button
        onclick={cancelEdit}
        style="
          background: transparent;
          border: 1px solid var(--ed-border);
          border-radius: 4px;
          padding: 2px 10px;
          font-size: 12px;
          color: var(--ed-text-muted);
          cursor: pointer;
        "
      >
        Cancel
      </button>
      <button
        onclick={saveEdit}
        disabled={saving || !editBody.trim()}
        style="
          padding: 2px 10px;
          border-radius: 4px;
          border: none;
          background: var(--ed-accent, oklch(0.55 0.18 250));
          color: #fff;
          font-size: 12px;
          cursor: {saving || !editBody.trim() ? 'not-allowed' : 'pointer'};
          opacity: {saving || !editBody.trim() ? 0.55 : 1};
        "
      >
        {saving ? "Saving…" : "Save"}
      </button>
    </div>
  {:else}
    <!-- Display mode: body text + action buttons -->
    <div style="display: flex; gap: 10px; align-items: flex-start;">
      <div
        style="flex: 1; color: var(--ed-text); line-height: 1.5; white-space: pre-wrap;"
      >
        {comment.body}
      </div>

      <div style="display: flex; gap: 6px; flex-shrink: 0; padding-top: 2px;">
        <button
          onclick={startEdit}
          title="Edit comment"
          style="
            background: transparent;
            border: 1px solid var(--ed-border);
            border-radius: 4px;
            padding: 2px 8px;
            font-size: 11px;
            color: var(--ed-text-muted);
            cursor: pointer;
          "
        >
          Edit
        </button>
        <button
          onclick={handleDelete}
          disabled={deleting}
          title="Delete comment"
          style="
            background: transparent;
            border: 1px solid var(--ed-border);
            border-radius: 4px;
            padding: 2px 8px;
            font-size: 11px;
            color: var(--ed-removed);
            cursor: {deleting ? 'not-allowed' : 'pointer'};
            opacity: {deleting ? 0.55 : 1};
          "
        >
          {deleting ? "…" : "Delete"}
        </button>
      </div>
    </div>
  {/if}
</div>
