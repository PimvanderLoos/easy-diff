<script lang="ts">
  /**
   * Reusable sidebar list item with active/disabled state.
   *
   * Translates `SidebarRow` from `concept-b.jsx` to Svelte. Renders as a
   * full-width button with `justify-between` so a label and a count badge can
   * be placed as children side by side.
   */

  import type { Snippet } from "svelte";

  interface Props {
    /** Highlight with accent background and text colour. Default: false. */
    active?: boolean;
    /** Reduce opacity and disable pointer events. Default: false. */
    disabled?: boolean;
    /** Click handler — omit to render a non-interactive row. */
    onclick?: () => void;
    /** Row content (typically a label + count). */
    children: Snippet;
  }

  let { active = false, disabled = false, onclick, children }: Props = $props();
</script>

<button
  type="button"
  {disabled}
  {onclick}
  class="flex w-full items-center justify-between gap-2 overflow-hidden whitespace-nowrap rounded-md px-2.5 py-1.5 text-left text-[13px] leading-snug transition-colors"
  class:bg-ed-accent-soft={active}
  class:text-ed-accent={active}
  class:text-ed-text={!active}
  class:opacity-40={disabled}
  class:cursor-not-allowed={disabled}
  class:cursor-pointer={!disabled}
  style="
    font-family: var(--font-sans);
    background: {active ? 'var(--ed-accent-soft)' : 'transparent'};
    border: none;
  "
>
  {@render children()}
</button>
