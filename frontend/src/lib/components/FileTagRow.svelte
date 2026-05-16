<script lang="ts">
  /**
   * Collapsed summary row shown below a FileHeader when the panel is collapsed.
   *
   * Displays the hunk count and a strip of attention-tag pills so the reviewer
   * can see at-a-glance what's inside without expanding the file.
   *
   * Translates `BFileTagRow` from `concept-b.jsx` to Svelte.
   *
   * @example
   * ```svelte
   * <FileTagRow tags={["important", "security-sensitive"]} hunkCount={3} />
   * ```
   */

  import TagPill from "./TagPill.svelte";

  interface Props {
    /** Attention tag ids present anywhere in this file's hunks. */
    tags: string[];
    /** Total number of hunks in the file. */
    hunkCount: number;
  }

  let { tags, hunkCount }: Props = $props();
</script>

<div
  style="
    height: 30px;
    flex-shrink: 0;
    padding: 0 16px;
    background: var(--ed-panel);
    display: flex;
    align-items: center;
    gap: 8px;
  "
>
  <span
    style="
      font-family: var(--font-mono);
      font-size: 11.5px;
      color: var(--ed-text-faint);
      white-space: nowrap;
    "
  >
    {hunkCount}
    {hunkCount === 1 ? "hunk" : "hunks"}
  </span>
  <span
    style="width: 1px; height: 12px; background: var(--ed-border-subtle); flex-shrink: 0;"
  ></span>
  <div
    style="
      display: flex;
      align-items: center;
      gap: 6px;
      flex: 1;
      flex-wrap: nowrap;
      overflow: hidden;
    "
  >
    {#if tags.length === 0}
      <span
        style="font-size: 11.5px; color: var(--ed-text-faint); font-family: var(--font-sans);"
      >
        no attention tags
      </span>
    {:else}
      {#each tags as tagId (tagId)}
        <TagPill id={tagId} />
      {/each}
    {/if}
  </div>
</div>
