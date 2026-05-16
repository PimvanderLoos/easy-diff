<script lang="ts">
  /**
   * Filters tab in the left rail.
   *
   * Renders two filter groups:
   * 1. Change type — single-select radio group. The selected type narrows
   *    the diff view to only show hunks of that type.
   * 2. Attention tags — multi-select checkbox group. Active tags further
   *    narrow the diff to hunks carrying those tags.
   *
   * A "clear (N)" button appears at the top when any filter is active.
   * Translates `BLeftFiltersTab` from `concept-b.jsx` to Svelte.
   */

  import SidebarLabel from "./SidebarLabel.svelte";
  import SidebarRow from "./SidebarRow.svelte";
  import {
    ATTENTION_TAGS,
    CHANGE_TYPES,
    edTagColor,
    type FilterState,
    type HunkData,
    type Classification,
  } from "../types.js";

  interface Props {
    /** Current filter state (change type + attention tags). */
    filter: FilterState;
    /** Called when the user changes a filter. */
    onFilterChange: (next: FilterState) => void;
    /**
     * Hunk data aggregated across all files, used to compute how many
     * hunks match each filter option (shown as counts in the sidebar).
     * Keyed by file path → array of hunks.
     */
    hunks: Record<string, HunkData[]>;
    /**
     * Classification data keyed by hunk id, used together with `hunks`
     * to compute per-option counts.
     */
    classifications: Record<string, Classification>;
  }

  let { filter, onFilterChange, hunks, classifications }: Props = $props();

  // ── Tag stats ─────────────────────────────────────────────────────────────

  /** Counts of hunks per change type and per attention tag. */
  const stats = $derived.by(() => {
    const types: Record<string, number> = {};
    const tags: Record<string, number> = {};

    for (const filePath in hunks) {
      for (const h of hunks[filePath]) {
        const cls = classifications[h.id];
        if (!cls) continue;

        types[cls.changeType] = (types[cls.changeType] ?? 0) + 1;

        // Aggregate tags: hunk-level + per-line tags.
        const tagSet = new Set<string>(cls.attentionTags);
        for (const line of h.lines) {
          for (const t of line.tags ?? []) tagSet.add(t);
        }
        for (const t of tagSet) {
          tags[t] = (tags[t] ?? 0) + 1;
        }
      }
    }

    return { types, tags };
  });

  /** Number of currently active filters (for the "clear" button label). */
  const activeCount = $derived(
    (filter.changeType !== "all" ? 1 : 0) + filter.attentionTags.length,
  );

  // ── Event handlers ────────────────────────────────────────────────────────

  function setType(id: string) {
    onFilterChange({ ...filter, changeType: id });
  }

  function toggleTag(id: string) {
    const cur = filter.attentionTags;
    const next = cur.includes(id)
      ? cur.filter((x) => x !== id)
      : [...cur, id];
    onFilterChange({ ...filter, attentionTags: next });
  }

  function clearFilters() {
    onFilterChange({ changeType: "all", attentionTags: [] });
  }

  // ── Theme detection ───────────────────────────────────────────────────────

  let isDark = $state(
    typeof document !== "undefined"
      ? document.documentElement.classList.contains("dark")
      : false,
  );

  $effect(() => {
    const observer = new MutationObserver(() => {
      isDark = document.documentElement.classList.contains("dark");
    });
    observer.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ["class"],
    });
    return () => observer.disconnect();
  });
</script>

<div class="px-[18px] py-4">
  <!-- Change type section header + clear button -->
  <div class="mb-1.5 flex items-baseline justify-between">
    <SidebarLabel>Change type</SidebarLabel>
    {#if activeCount > 0}
      <button
        type="button"
        onclick={clearFilters}
        class="cursor-pointer border-none bg-transparent text-[11px] text-ed-accent"
        style="font-family: var(--font-sans);"
      >
        clear ({activeCount})
      </button>
    {/if}
  </div>

  <!-- Change type radios -->
  <div class="flex flex-col gap-px">
    {#each CHANGE_TYPES.filter((c) => c.id === "all" || stats.types[c.id]) as ct}
      {@const isActive = (filter.changeType ?? "all") === ct.id}
      {@const count =
        ct.id === "all"
          ? Object.values(stats.types).reduce((a, b) => a + b, 0)
          : (stats.types[ct.id] ?? 0)}
      <SidebarRow active={isActive} onclick={() => setType(ct.id)}>
        <span class="inline-flex items-center gap-2">
          <!-- Radio dot -->
          <span
            style="
              width: 11px; height: 11px; border-radius: 50%; flex-shrink: 0;
              border: 1.4px solid {isActive ? 'var(--ed-accent)' : 'var(--ed-border)'};
              background: {isActive ? 'var(--ed-accent)' : 'transparent'};
              box-shadow: {isActive ? 'inset 0 0 0 2px var(--ed-bg)' : 'none'};
            "
          ></span>
          {ct.label}
        </span>
        <span
          class="text-ed-text-faint"
          style="font-family: var(--font-mono); font-size: 11px;"
        >
          {count}
        </span>
      </SidebarRow>
    {/each}
  </div>

  <!-- Attention tags section -->
  <div class="mt-[22px]">
    <SidebarLabel>Attention</SidebarLabel>
  </div>

  <div class="flex flex-col gap-px">
    {#each ATTENTION_TAGS as tg}
      {@const present = stats.tags[tg.id] ?? 0}
      {@const isActive = filter.attentionTags.includes(tg.id)}
      {@const color = edTagColor(tg.id, isDark)}
      <SidebarRow
        disabled={!present}
        onclick={() => present && toggleTag(tg.id)}
      >
        <span class="inline-flex items-center gap-2">
          <!-- Colored checkbox -->
          <span
            style="
              width: 11px; height: 11px; border-radius: 2px; flex-shrink: 0;
              border: 1.4px solid {isActive && color ? color : 'var(--ed-border)'};
              background: {isActive && color ? color : 'transparent'};
            "
          ></span>
          <!-- Tag color swatch -->
          <span
            style="
              width: 6px; height: 6px; border-radius: 2px; flex-shrink: 0;
              background: {color ?? 'var(--ed-text-faint)'};
            "
          ></span>
          <span style={isActive && color ? `color: ${color}` : ""}
            >{tg.label}</span
          >
        </span>
        <span
          class="text-ed-text-faint"
          style="font-family: var(--font-mono); font-size: 11px;"
        >
          {present}
        </span>
      </SidebarRow>
    {/each}
  </div>
</div>
