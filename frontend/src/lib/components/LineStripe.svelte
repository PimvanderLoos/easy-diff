<script lang="ts">
  /**
   * 3px-wide vertical accent bar in the first column of each diff line.
   *
   * When attention tags are present, splits into equal-height colored segments
   * (up to 3) using the oklch tag-color formula. When no tags apply, renders as
   * a transparent placeholder to preserve grid alignment.
   *
   * Translates `EDLineStripe` from `shared.jsx` to Svelte.
   *
   * @example
   * ```svelte
   * <LineStripe tags={["important", "security-sensitive"]} isDark={true} />
   * ```
   */

  import { edTagColor } from "../types.js";

  interface Props {
    /** Attention tag ids to color the stripe segments with. */
    tags: string[];
    /** Whether the dark theme is active (drives the oklch formula). */
    isDark: boolean;
  }

  let { tags, isDark }: Props = $props();

  const visibleTags = $derived(tags.slice(0, 3));
</script>

{#if visibleTags.length === 0}
  <span style="width: 3px; flex-shrink: 0;"></span>
{:else}
  <span
    style="
      display: flex;
      flex-direction: column;
      width: 3px;
      align-self: stretch;
      flex-shrink: 0;
      border-radius: 1px;
      overflow: hidden;
    "
  >
    {#each visibleTags as tagId (tagId)}
      <span
        style="flex: 1; background: {edTagColor(tagId, isDark) ?? 'transparent'};"
      ></span>
    {/each}
  </span>
{/if}
