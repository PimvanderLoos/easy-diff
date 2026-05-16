<script lang="ts">
  /**
   * Uppercase section header for sidebar groups.
   *
   * Translates `SidebarLabel` from `concept-b.jsx` to Svelte. Renders a
   * small-caps monospace label with wide letter-spacing in the faint text
   * colour. Provides visual separation between sidebar sections.
   */

  import type { Snippet } from "svelte";

  interface Props {
    /** Label text (rendered uppercase via CSS). */
    children: Snippet;
  }

  let { children }: Props = $props();
</script>

<div
  class="mb-2 text-ed-text-faint"
  style="
    font-size: 10px;
    letter-spacing: 1.6px;
    text-transform: uppercase;
    font-family: var(--font-mono);
  "
>
  {@render children()}
</div>
