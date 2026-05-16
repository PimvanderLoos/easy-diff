<script lang="ts">
  /**
   * Small icon-button with a subtle border, used in the MainToolbar.
   *
   * Translates `IconBtn` from `concept-b.jsx` to Svelte.
   *
   * @example
   * ```svelte
   * <IconBtn title="Previous file" onclick={prev}>
   *   <!-- svg icon -->
   * </IconBtn>
   * ```
   */

  import type { Snippet } from "svelte";

  interface Props {
    /** Tooltip / accessible label. */
    title?: string;
    /** Whether the button is disabled. */
    disabled?: boolean;
    /** Click handler. */
    onclick?: () => void;
    /** Icon slot. */
    children: Snippet;
  }

  let { title, disabled = false, onclick, children }: Props = $props();
</script>

<button
  {title}
  {disabled}
  type="button"
  {onclick}
  style="
    width: 28px;
    height: 28px;
    border-radius: 6px;
    background: transparent;
    border: 1px solid var(--ed-border);
    color: {disabled ? 'var(--ed-text-faint)' : 'var(--ed-text-muted)'};
    display: inline-flex;
    align-items: center;
    justify-content: center;
    cursor: {disabled ? 'not-allowed' : 'pointer'};
    flex-shrink: 0;
    padding: 0;
  "
>
  {@render children()}
</button>
