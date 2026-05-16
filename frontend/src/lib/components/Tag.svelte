<script lang="ts">
  /**
   * Restrained chip/badge component.
   *
   * Translates `EDTag` from `shared.jsx` to Svelte. Uses IBM Plex Mono at
   * 11 px with a subtle border and background derived from the active theme.
   * The optional `color` prop overrides the text/border colour for semantic
   * tags (draft, approved, changes-requested).
   */

  import type { Snippet } from "svelte";

  interface Props {
    /** Optional colour override for text. Falls back to `--ed-text-muted`. */
    color?: string;
    /** Tag content (text or other inline elements). */
    children: Snippet;
  }

  let { color, children }: Props = $props();
</script>

<span
  class="inline-flex items-center gap-1.5 whitespace-nowrap rounded border px-1.5 py-px text-[11px] leading-none"
  style="
    font-family: var(--font-mono);
    letter-spacing: 0.2px;
    color: {color ?? 'var(--ed-text-muted)'};
    background: var(--ed-tag-bg, rgba(0,0,0,0.04));
    border-color: var(--ed-tag-border, rgba(0,0,0,0.06));
  "
>
  {@render children()}
</span>

<style>
  :global(.dark) span {
    --ed-tag-bg: rgba(255, 255, 255, 0.04);
    --ed-tag-border: rgba(255, 255, 255, 0.06);
  }
</style>
