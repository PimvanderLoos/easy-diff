<script lang="ts">
  /**
   * Dropdown-style pill button for selecting a single value from a list.
   *
   * Renders a bordered pill with an optional label prefix and a chevron.
   * Used in the MainToolbar for batch and unreviewed-filter selectors.
   * Translates `PillSelect` from `concept-b.jsx` to Svelte.
   *
   * @example
   * ```svelte
   * <PillSelect label="Batch" value="1 file" />
   * <PillSelect value="Unreviewed" />
   * ```
   */

  interface Props {
    /** Optional prefix label shown in muted text before the value. */
    label?: string;
    /** Currently selected value string. */
    value: string;
    /** Click handler — opens the dropdown (not yet implemented). */
    onclick?: () => void;
  }

  let { label, value, onclick }: Props = $props();
</script>

<button
  type="button"
  {onclick}
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
    white-space: nowrap;
    flex-shrink: 0;
  "
>
  {#if label}
    <span style="color: var(--ed-text-muted); font-size: 12px;">{label}:</span>
  {/if}
  <span style="font-weight: 500;">{value}</span>
  <!-- chevron-down icon -->
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
