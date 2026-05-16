<script lang="ts">
  /**
   * Reviewing / Reviewed toggle pill for a file header.
   *
   * Clicking transitions between the two states. Translates `BStatusPill` from
   * `concept-b.jsx` to Svelte.
   *
   * @example
   * ```svelte
   * <StatusPill reviewed={false} onclick={() => toggleReviewed(file.path)} />
   * ```
   */

  interface Props {
    /** Whether the file is currently marked as reviewed. */
    reviewed: boolean;
    /** Called when the user clicks the pill to toggle the state. */
    onclick?: () => void;
  }

  let { reviewed, onclick }: Props = $props();

  const label = $derived(reviewed ? "Reviewed" : "Reviewing");
  const titleText = $derived(
    reviewed ? "Mark as unreviewed" : "Mark as reviewed",
  );
</script>

<button
  type="button"
  title={titleText}
  {onclick}
  style="
    display: inline-flex;
    align-items: center;
    gap: 7px;
    padding: 5px 10px;
    border-radius: 6px;
    background: {reviewed ? 'var(--ed-added-bg)' : 'transparent'};
    border: 1px solid var(--ed-added);
    color: var(--ed-added);
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    flex-shrink: 0;
    font-family: var(--font-sans);
  "
>
  <!-- Checkbox mark -->
  <span
    style="
      width: 12px;
      height: 12px;
      border-radius: 3px;
      border: 1.5px solid var(--ed-added);
      background: {reviewed ? 'var(--ed-added)' : 'transparent'};
      display: inline-flex;
      align-items: center;
      justify-content: center;
      flex-shrink: 0;
    "
  >
    {#if reviewed}
      <svg width="8" height="8" viewBox="0 0 8 8" fill="none">
        <path
          d="M1.5 4 L3.3 5.8 L6.5 2"
          stroke="#fff"
          stroke-width="1.6"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
      </svg>
    {/if}
  </span>
  {label}
  <!-- chevron-down -->
  <svg width="10" height="10" viewBox="0 0 10 10" fill="none">
    <path
      d="M2 4 L5 7 L8 4"
      stroke="var(--ed-added)"
      stroke-width="1.4"
      stroke-linecap="round"
      stroke-linejoin="round"
    />
  </svg>
</button>
