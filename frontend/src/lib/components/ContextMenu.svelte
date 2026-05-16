<script lang="ts">
  /**
   * Custom right-click context menu for diff lines.
   *
   * Renders at a fixed screen position (x, y) and provides three actions:
   *   - "Add comment" — emits `onAddComment`
   *   - "Override category →" — submenu with change types (radio selection)
   *   - "Override tags →" — submenu with attention tags (checkboxes)
   *
   * Clicking outside or pressing Escape closes the menu via `onClose`.
   *
   * @example
   * ```svelte
   * <ContextMenu
   *   x={120} y={300}
   *   hunkId="src/Foo.java:0"
   *   currentChangeType="logic"
   *   currentTags={["security-sensitive"]}
   *   onAddComment={() => openCommentInput()}
   *   onOverrideCategory={(ct) => applyOverride(ct)}
   *   onOverrideTags={(tags) => applyTagOverride(tags)}
   *   onClose={() => closeMenu()}
   * />
   * ```
   */

  import { onMount } from "svelte";
  import { CHANGE_TYPES, ATTENTION_TAGS } from "../types.js";

  interface Props {
    /** Horizontal screen position for the menu's top-left corner. */
    x: number;
    /** Vertical screen position for the menu's top-left corner. */
    y: number;
    /** Hunk id the menu was opened on. */
    hunkId: string;
    /** Currently active change type for the hunk (from LLM or override). */
    currentChangeType: string;
    /** Currently active attention tags for the hunk (from LLM or override). */
    currentTags: string[];
    /** Emitted when the user picks "Add comment". */
    onAddComment: () => void;
    /** Emitted when the user picks a new change type override. */
    onOverrideCategory: (changeType: string) => void;
    /** Emitted when the user confirms a new tag selection. */
    onOverrideTags: (tags: string[]) => void;
    /** Emitted to close the menu (user clicked outside or pressed Escape). */
    onClose: () => void;
  }

  let {
    x,
    y,
    hunkId: _hunkId,
    currentChangeType,
    currentTags,
    onAddComment,
    onOverrideCategory,
    onOverrideTags,
    onClose,
  }: Props = $props();

  /** Which submenu is open: "category", "tags", or null. */
  let openSubmenu = $state<"category" | "tags" | null>(null);

  /** Local mutable tag selection (mirrors `currentTags` but can be toggled). */
  let selectedTags = $state<string[]>([...currentTags]);

  function toggleTag(tagId: string) {
    selectedTags = selectedTags.includes(tagId)
      ? selectedTags.filter((t) => t !== tagId)
      : [...selectedTags, tagId];
  }

  function applyTagOverride() {
    onOverrideTags([...selectedTags]);
    onClose();
  }

  function pickCategory(ctId: string) {
    onOverrideCategory(ctId);
    onClose();
  }

  function handleAddComment() {
    onAddComment();
    onClose();
  }

  // Close on outside click or Escape.
  onMount(() => {
    function handleClick(e: MouseEvent) {
      const target = e.target as Node;
      // The menu root is identified by data-context-menu attribute.
      const menu = document.querySelector("[data-context-menu]");
      if (menu && !menu.contains(target)) {
        onClose();
      }
    }
    function handleKey(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
    }
    // Use capture so we see the event before bubbling stops it.
    document.addEventListener("mousedown", handleClick, true);
    document.addEventListener("keydown", handleKey);
    return () => {
      document.removeEventListener("mousedown", handleClick, true);
      document.removeEventListener("keydown", handleKey);
    };
  });

  // Clamp menu position so it doesn't overflow the viewport.
  const MENU_W = 220;
  const menuX = $derived(
    Math.min(x, typeof window !== "undefined" ? window.innerWidth - MENU_W - 8 : x),
  );
  const menuY = $derived(
    Math.min(y, typeof window !== "undefined" ? window.innerHeight - 200 : y),
  );

  // Change types — exclude the "all" sentinel.
  const changeTypeOptions = CHANGE_TYPES.filter((ct) => ct.id !== "all");
</script>

<!--
  Context menu rendered via `position: fixed` so it floats above all content.
  `data-context-menu` is used by the outside-click handler above.
-->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  data-context-menu
  style="
    position: fixed;
    left: {menuX}px;
    top: {menuY}px;
    width: {MENU_W}px;
    background: var(--ed-panel);
    border: 1px solid var(--ed-border);
    border-radius: 6px;
    box-shadow: 0 4px 16px oklch(0 0 0 / 0.25);
    z-index: 9999;
    font-family: var(--font-sans);
    font-size: 13px;
    overflow: visible;
    padding: 4px 0;
  "
  onmousedown={(e) => e.stopPropagation()}
>
  <!-- Add comment -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="menu-item"
    onclick={handleAddComment}
    onkeydown={(e) => e.key === "Enter" && handleAddComment()}
    role="menuitem"
    tabindex={0}
  >
    Add comment
  </div>

  <div style="height: 1px; background: var(--ed-border-subtle); margin: 4px 0;"></div>

  <!-- Override category submenu -->
  <div style="position: relative;">
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="menu-item"
      onclick={() =>
        (openSubmenu = openSubmenu === "category" ? null : "category")}
      onkeydown={(e) =>
        e.key === "Enter" &&
        (openSubmenu = openSubmenu === "category" ? null : "category")}
      role="menuitem"
      tabindex={0}
      style="justify-content: space-between;"
    >
      <span>Override category</span>
      <span style="opacity: 0.5; font-size: 11px;">▶</span>
    </div>

    {#if openSubmenu === "category"}
      <div
        style="
          position: absolute;
          left: 100%;
          top: 0;
          width: 180px;
          background: var(--ed-panel);
          border: 1px solid var(--ed-border);
          border-radius: 6px;
          box-shadow: 0 4px 16px oklch(0 0 0 / 0.25);
          padding: 4px 0;
          z-index: 10000;
        "
      >
        {#each changeTypeOptions as ct (ct.id)}
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div
            class="menu-item"
            onclick={() => pickCategory(ct.id)}
            onkeydown={(e) => e.key === "Enter" && pickCategory(ct.id)}
            role="menuitem"
            tabindex={0}
            style="gap: 8px;"
          >
            <span
              style="
                display: inline-block;
                width: 10px;
                height: 10px;
                border-radius: 50%;
                border: 2px solid var(--ed-border);
                background: {ct.id === currentChangeType
                ? 'var(--ed-accent, oklch(0.55 0.18 250))'
                : 'transparent'};
                flex-shrink: 0;
              "
            ></span>
            {ct.label}
          </div>
        {/each}
      </div>
    {/if}
  </div>

  <!-- Override tags submenu -->
  <div style="position: relative;">
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="menu-item"
      onclick={() => (openSubmenu = openSubmenu === "tags" ? null : "tags")}
      onkeydown={(e) =>
        e.key === "Enter" &&
        (openSubmenu = openSubmenu === "tags" ? null : "tags")}
      role="menuitem"
      tabindex={0}
      style="justify-content: space-between;"
    >
      <span>Override tags</span>
      <span style="opacity: 0.5; font-size: 11px;">▶</span>
    </div>

    {#if openSubmenu === "tags"}
      <div
        style="
          position: absolute;
          left: 100%;
          top: 0;
          width: 200px;
          background: var(--ed-panel);
          border: 1px solid var(--ed-border);
          border-radius: 6px;
          box-shadow: 0 4px 16px oklch(0 0 0 / 0.25);
          padding: 4px 0;
          z-index: 10000;
        "
      >
        {#each ATTENTION_TAGS as tag (tag.id)}
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div
            class="menu-item"
            onclick={() => toggleTag(tag.id)}
            onkeydown={(e) => e.key === "Enter" && toggleTag(tag.id)}
            role="menuitem"
            tabindex={0}
            style="gap: 8px;"
          >
            <span
              style="
                display: inline-block;
                width: 10px;
                height: 10px;
                border-radius: 2px;
                border: 2px solid var(--ed-border);
                background: {selectedTags.includes(tag.id)
                ? 'var(--ed-accent, oklch(0.55 0.18 250))'
                : 'transparent'};
                flex-shrink: 0;
              "
            ></span>
            {tag.label}
          </div>
        {/each}

        <div style="height: 1px; background: var(--ed-border-subtle); margin: 4px 0;"></div>

        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          class="menu-item"
          onclick={applyTagOverride}
          onkeydown={(e) => e.key === "Enter" && applyTagOverride()}
          role="menuitem"
          tabindex={0}
          style="color: var(--ed-accent, oklch(0.55 0.18 250)); font-weight: 500;"
        >
          Apply
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  .menu-item {
    display: flex;
    align-items: center;
    padding: 6px 12px;
    cursor: pointer;
    color: var(--ed-text);
    border-radius: 4px;
    margin: 1px 4px;
    user-select: none;
    transition: background 0.1s;
  }
  .menu-item:hover,
  .menu-item:focus {
    background: var(--ed-hover, oklch(0.5 0 0 / 0.08));
    outline: none;
  }
</style>
