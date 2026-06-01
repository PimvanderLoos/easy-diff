<script lang="ts">
  /**
   * Global error-toast stack.
   *
   * Renders the `errorToasts` store (populated by `reportError`) as a fixed,
   * bottom-right stack of dismissible notifications. Mounted once at the app
   * root so any component can surface a failure without wiring up its own UI.
   *
   * Each toast auto-dismisses after {@link AUTO_DISMISS_MS}; the user can also
   * dismiss it immediately via the close button.
   */

  import { errorToasts, dismissToast } from "../stores.js";

  /** How long a toast stays visible before auto-dismissing, in ms. */
  const AUTO_DISMISS_MS = 8000;

  // Toasts whose auto-dismiss timer has already been armed, so the effect below
  // schedules exactly one timer per toast even as the store updates.
  const armed = new Set<number>();

  $effect(() => {
    for (const toast of $errorToasts) {
      if (armed.has(toast.id)) continue;
      armed.add(toast.id);
      setTimeout(() => {
        dismissToast(toast.id);
        armed.delete(toast.id);
      }, AUTO_DISMISS_MS);
    }
  });
</script>

<div class="toast-stack" role="region" aria-label="Error notifications">
  {#each $errorToasts as toast (toast.id)}
    <div class="toast" role="alert">
      <span class="toast-message">{toast.message}</span>
      {#if toast.detail}<span class="toast-detail">{toast.detail}</span>{/if}
      <button
        class="toast-close"
        aria-label="Dismiss"
        onclick={() => dismissToast(toast.id)}>×</button
      >
    </div>
  {/each}
</div>

<style>
  .toast-stack {
    position: fixed;
    bottom: 16px;
    right: 16px;
    z-index: 1000;
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-width: 380px;
    pointer-events: none;
  }

  .toast {
    pointer-events: auto;
    display: grid;
    grid-template-columns: 1fr auto;
    column-gap: 8px;
    font-family: var(--font-sans);
    color: var(--ed-removed);
    background: color-mix(in srgb, var(--ed-removed) 12%, var(--ed-bg));
    border: 1px solid color-mix(in srgb, var(--ed-removed) 35%, transparent);
    border-radius: 6px;
    padding: 10px 12px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.18);
  }

  .toast-message {
    grid-row: 1;
    grid-column: 1;
    font-size: 13px;
    font-weight: 600;
  }

  .toast-detail {
    grid-row: 2;
    grid-column: 1;
    margin-top: 2px;
    font-size: 12px;
    color: var(--ed-text-muted);
    word-break: break-word;
  }

  .toast-close {
    grid-row: 1;
    grid-column: 2;
    align-self: start;
    background: none;
    border: none;
    cursor: pointer;
    font-size: 16px;
    line-height: 1;
    color: var(--ed-text-muted);
    padding: 0 2px;
  }

  .toast-close:hover {
    color: var(--ed-text);
  }
</style>
