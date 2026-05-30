<script lang="ts">
  /**
   * Modal dialog listing the application's global keyboard shortcuts.
   *
   * The shortcuts themselves are handled by the global `keydown` listener in
   * `ReviewScreen.svelte`; this dialog only documents them. Dismissed by the
   * close button, a backdrop click, or the Escape key.
   *
   * @example
   * ```svelte
   * <KeyboardShortcutsDialog onClose={() => helpOpen.set(false)} />
   * ```
   */

  interface Props {
    /** Called when the dialog is dismissed. */
    onClose?: () => void;
  }

  let { onClose }: Props = $props();

  /** Displayed shortcut list. Keep in sync with ReviewScreen's keydown handler. */
  const shortcuts: Array<{ keys: string[]; desc: string }> = [
    { keys: ["j"], desc: "Next file" },
    { keys: ["k"], desc: "Previous file" },
    { keys: ["v"], desc: "Toggle unified / split view" },
    { keys: ["s"], desc: "Open the submit-review dialog" },
    { keys: ["?"], desc: "Show this help" },
    { keys: ["Esc"], desc: "Close dialogs" },
  ];

  function handleBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) onClose?.();
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") onClose?.();
  }
</script>

<!-- Esc closes regardless of where focus is (opener button stays focused). -->
<svelte:window onkeydown={handleKeydown} />

<!-- Modal backdrop -->
<div
  class="fixed inset-0 z-50 flex items-center justify-center"
  style="background: rgba(0,0,0,0.55);"
  role="dialog"
  tabindex="-1"
  aria-modal="true"
  aria-label="Keyboard shortcuts"
  onclick={handleBackdropClick}
  onkeydown={handleKeydown}
>
  <!-- Dialog card -->
  <div
    style="
      background: var(--ed-panel);
      border: 1px solid var(--ed-border-subtle);
      border-radius: 10px;
      width: 360px;
      max-width: calc(100vw - 32px);
      padding: 24px;
      display: flex;
      flex-direction: column;
      gap: 16px;
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
        Keyboard shortcuts
      </h2>
      <button
        onclick={() => onClose?.()}
        aria-label="Close dialog"
        style="
          background: none;
          border: none;
          cursor: pointer;
          padding: 4px;
          color: var(--ed-text-muted);
          font-size: 18px;
          line-height: 1;
        "
      >×</button>
    </div>

    <!-- Shortcut rows -->
    <div style="display: flex; flex-direction: column; gap: 10px;">
      {#each shortcuts as row (row.desc)}
        <div style="display: flex; align-items: center; justify-content: space-between; gap: 12px;">
          <span
            style="font-family: var(--font-sans); font-size: 13px; color: var(--ed-text);"
          >{row.desc}</span>
          <span style="display: flex; align-items: center; gap: 4px;">
            {#each row.keys as key (key)}
              <kbd
                style="
                  font-family: var(--font-mono);
                  font-size: 11px;
                  color: var(--ed-text-muted);
                  background: var(--ed-bg);
                  border: 1px solid var(--ed-border-subtle);
                  border-radius: 4px;
                  padding: 2px 6px;
                  min-width: 16px;
                  text-align: center;
                "
              >{key}</kbd>
            {/each}
          </span>
        </div>
      {/each}
    </div>
  </div>
</div>
