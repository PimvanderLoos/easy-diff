<script lang="ts">
  /**
   * Segmented control for choosing between two or more mutually exclusive options.
   *
   * Used in the MainToolbar to toggle between Unified and Split diff views.
   * Translates `SegToggle` from `concept-b.jsx` to Svelte.
   *
   * @example
   * ```svelte
   * <SegToggle
   *   options={[
   *     { value: 'inline', label: 'Unified' },
   *     { value: 'split', label: 'Split' },
   *   ]}
   *   value="inline"
   *   onchange={(v) => (diffView = v)}
   * />
   * ```
   */

  interface Option {
    value: string;
    label: string;
  }

  interface Props {
    /** Available options. */
    options: Option[];
    /** Currently selected option value. */
    value: string;
    /** Called when the user selects a different option. */
    onchange?: (value: string) => void;
  }

  let { options, value, onchange }: Props = $props();
</script>

<div
  style="
    display: inline-flex;
    border-radius: 6px;
    padding: 2px;
    background: var(--ed-panel);
    border: 1px solid var(--ed-border);
  "
>
  {#each options as opt (opt.value)}
    <button
      type="button"
      onclick={() => onchange?.(opt.value)}
      style="
        padding: 4px 12px;
        font-size: 12px;
        font-family: var(--font-sans);
        font-weight: 500;
        color: {opt.value === value ? 'var(--ed-text)' : 'var(--ed-text-muted)'};
        background: {opt.value === value ? 'var(--ed-bg)' : 'transparent'};
        border-radius: 4px;
        border: none;
        cursor: pointer;
        box-shadow: {opt.value === value ? '0 1px 1px rgba(0,0,0,0.04)' : 'none'};
      "
    >
      {opt.label}
    </button>
  {/each}
</div>
