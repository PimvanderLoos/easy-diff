<script lang="ts">
  /**
   * Modal dialog for submitting a pull request review to GitHub.
   *
   * Renders a centered overlay with:
   * - Three radio buttons for the review verdict (Comment, Approve, Request Changes).
   * - An optional body textarea for the top-level review message.
   * - An "Include analysis summary" checkbox that prepends the Pass 1 summary.
   * - A comment count line: "N draft comment(s) will be submitted".
   * - Submit and Cancel buttons.
   * - Loading state while the Tauri command is in flight.
   * - Error message on failure.
   *
   * @example
   * ```svelte
   * <SubmitReviewDialog
   *   draftCount={3}
   *   onClose={() => (open = false)}
   *   onSubmitted={() => draftComments.set([])}
   * />
   * ```
   */

  import { invoke } from "@tauri-apps/api/core";

  interface Props {
    /** Number of draft comments that will be included in the submission. */
    draftCount: number;
    /** PR number of the pull request being reviewed. */
    prNumber: number;
    /** Called when the dialog is dismissed (Cancel or after successful submit). */
    onClose?: () => void;
    /** Called after a successful submission so the parent can refresh comment state. */
    onSubmitted?: () => void;
  }

  let { draftCount, prNumber, onClose, onSubmitted }: Props = $props();

  // ── Form state ────────────────────────────────────────────────────────────

  type Verdict = "comment" | "approve" | "request_changes";

  let verdict = $state<Verdict>("comment");
  let body = $state("");
  let includeSummary = $state(false);

  // ── Submission state ──────────────────────────────────────────────────────

  let loading = $state(false);
  let error = $state<string | null>(null);

  async function handleSubmit() {
    loading = true;
    error = null;
    try {
      await invoke("submit_review", {
        prNumber,
        verdict,
        body: body.trim() || null,
        includeSummary,
      });
      onSubmitted?.();
      onClose?.();
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  function handleCancel() {
    if (!loading) onClose?.();
  }

  function handleBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget && !loading) onClose?.();
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape" && !loading) onClose?.();
  }
</script>

<!-- Modal backdrop -->
<div
  class="fixed inset-0 z-50 flex items-center justify-center"
  style="background: rgba(0,0,0,0.55);"
  role="dialog"
  tabindex="-1"
  aria-modal="true"
  aria-label="Submit review"
  onclick={handleBackdropClick}
  onkeydown={handleKeydown}
>
  <!-- Dialog card -->
  <div
    style="
      background: var(--ed-panel);
      border: 1px solid var(--ed-border-subtle);
      border-radius: 10px;
      width: 480px;
      max-width: calc(100vw - 32px);
      padding: 24px;
      display: flex;
      flex-direction: column;
      gap: 20px;
      box-shadow: 0 8px 32px rgba(0,0,0,0.28);
    "
  >
    <!-- Header -->
    <div style="display: flex; align-items: center; justify-content: space-between;">
      <h2
        style="
          margin: 0;
          font-family: var(--font-sans);
          font-size: 15px;
          font-weight: 600;
          color: var(--ed-text);
        "
      >
        Submit review
      </h2>
      <button
        onclick={handleCancel}
        disabled={loading}
        aria-label="Close dialog"
        style="
          background: none;
          border: none;
          cursor: pointer;
          padding: 4px;
          color: var(--ed-text-muted);
          font-size: 18px;
          line-height: 1;
          opacity: {loading ? 0.4 : 1};
        "
      >×</button>
    </div>

    <!-- Verdict radios -->
    <fieldset style="border: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 10px;">
      <legend
        style="
          font-family: var(--font-sans);
          font-size: 12px;
          font-weight: 600;
          color: var(--ed-text-muted);
          text-transform: uppercase;
          letter-spacing: 0.06em;
          margin-bottom: 6px;
        "
      >Verdict</legend>

      {#each ([
        { value: "comment",         label: "Comment",         desc: "Submit general feedback without approval" },
        { value: "approve",         label: "Approve",         desc: "Submit feedback and approve merging" },
        { value: "request_changes", label: "Request changes", desc: "Submit feedback that must be addressed" },
      ] as const) as opt}
        <label
          style="
            display: flex;
            align-items: flex-start;
            gap: 10px;
            cursor: {loading ? 'not-allowed' : 'pointer'};
            opacity: {loading ? 0.6 : 1};
          "
        >
          <input
            type="radio"
            name="verdict"
            value={opt.value}
            bind:group={verdict}
            disabled={loading}
            style="margin-top: 3px; accent-color: var(--ed-accent);"
          />
          <span style="display: flex; flex-direction: column; gap: 2px;">
            <span
              style="
                font-family: var(--font-sans);
                font-size: 13px;
                font-weight: 500;
                color: var(--ed-text);
              "
            >{opt.label}</span>
            <span
              style="
                font-family: var(--font-sans);
                font-size: 11px;
                color: var(--ed-text-muted);
              "
            >{opt.desc}</span>
          </span>
        </label>
      {/each}
    </fieldset>

    <!-- Body textarea -->
    <div style="display: flex; flex-direction: column; gap: 6px;">
      <label
        style="
          font-family: var(--font-sans);
          font-size: 12px;
          font-weight: 600;
          color: var(--ed-text-muted);
          text-transform: uppercase;
          letter-spacing: 0.06em;
        "
        for="review-body"
      >Review body <span style="font-weight: 400; text-transform: none;">(optional)</span></label>
      <textarea
        id="review-body"
        bind:value={body}
        disabled={loading}
        placeholder="Add a comment…"
        rows={4}
        style="
          background: var(--ed-bg);
          border: 1px solid var(--ed-border-subtle);
          border-radius: 6px;
          color: var(--ed-text);
          font-family: var(--font-mono);
          font-size: 12px;
          line-height: 1.5;
          padding: 8px 10px;
          resize: vertical;
          width: 100%;
          box-sizing: border-box;
          opacity: {loading ? 0.6 : 1};
        "
      ></textarea>
    </div>

    <!-- Include summary checkbox -->
    <label
      style="
        display: flex;
        align-items: center;
        gap: 8px;
        cursor: {loading ? 'not-allowed' : 'pointer'};
        opacity: {loading ? 0.6 : 1};
      "
    >
      <input
        type="checkbox"
        bind:checked={includeSummary}
        disabled={loading}
        style="accent-color: var(--ed-accent);"
      />
      <span
        style="
          font-family: var(--font-sans);
          font-size: 13px;
          color: var(--ed-text);
        "
      >Include analysis summary in review body</span>
    </label>

    <!-- Draft comment count -->
    <p
      style="
        margin: 0;
        font-family: var(--font-sans);
        font-size: 12px;
        color: var(--ed-text-muted);
      "
    >
      {draftCount}
      {draftCount === 1 ? "draft comment" : "draft comments"} will be submitted.
    </p>

    <!-- Error message -->
    {#if error}
      <p
        style="
          margin: 0;
          font-family: var(--font-sans);
          font-size: 12px;
          color: var(--ed-removed);
          background: color-mix(in srgb, var(--ed-removed) 10%, transparent);
          border: 1px solid color-mix(in srgb, var(--ed-removed) 30%, transparent);
          border-radius: 6px;
          padding: 8px 10px;
        "
      >
        {error}
      </p>
    {/if}

    <!-- Action buttons -->
    <div style="display: flex; justify-content: flex-end; gap: 8px;">
      <button
        onclick={handleCancel}
        disabled={loading}
        style="
          background: none;
          border: 1px solid var(--ed-border-subtle);
          border-radius: 6px;
          color: var(--ed-text);
          cursor: {loading ? 'not-allowed' : 'pointer'};
          font-family: var(--font-sans);
          font-size: 13px;
          padding: 6px 14px;
          opacity: {loading ? 0.5 : 1};
        "
      >Cancel</button>

      <button
        onclick={handleSubmit}
        disabled={loading}
        style="
          background: var(--ed-accent);
          border: none;
          border-radius: 6px;
          color: #fff;
          cursor: {loading ? 'not-allowed' : 'pointer'};
          font-family: var(--font-sans);
          font-size: 13px;
          font-weight: 500;
          padding: 6px 16px;
          opacity: {loading ? 0.7 : 1};
          display: flex;
          align-items: center;
          gap: 6px;
        "
      >
        {#if loading}
          <span style="display:inline-block;width:12px;height:12px;border:2px solid rgba(255,255,255,0.4);border-top-color:#fff;border-radius:50%;animation:spin 0.7s linear infinite;"></span>
          Submitting…
        {:else}
          Submit review
        {/if}
      </button>
    </div>
  </div>
</div>

<style>
  @keyframes spin {
    to { transform: rotate(360deg); }
  }
</style>
