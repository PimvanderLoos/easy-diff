<script lang="ts">
  /**
   * Files tab in the left rail.
   *
   * Three sections:
   * 1. **Show** — All / Reviewed / Unreviewed file counts with clickable rows.
   * 2. **Categories** — Attention tags (colored swatches + counts) and change
   *    types (outline dots + counts) aggregated across all hunks.
   * 3. **Files** — Clickable file list with +/- stats and tag pills.
   *
   * Translates `BLeftFilesTab` from `concept-b.jsx` to Svelte.
   */

  import SidebarLabel from "./SidebarLabel.svelte";
  import SidebarRow from "./SidebarRow.svelte";
  import TagPill from "./TagPill.svelte";
  import {
    ATTENTION_TAGS,
    CHANGE_TYPES,
    edTagColor,
    type FileEntry,
    type HunkData,
    type Classification,
  } from "../types.js";

  interface Props {
    /** List of files in the PR. */
    files: FileEntry[];
    /** Hunk data keyed by file path. */
    hunks: Record<string, HunkData[]>;
    /** Classification data keyed by hunk id. */
    classifications: Record<string, Classification>;
    /** Set of file paths currently marked as reviewed. */
    reviewedSet: Set<string>;
    /** Index of the currently focused file in the main panel. */
    activeFileIndex: number;
    /** Called when the user clicks a file row. */
    onSelectFile: (index: number) => void;
  }

  let {
    files,
    hunks,
    classifications,
    reviewedSet,
    activeFileIndex,
    onSelectFile,
  }: Props = $props();

  // ── Tag stats aggregation ─────────────────────────────────────────────────

  const stats = $derived.by(() => {
    const types: Record<string, number> = {};
    const tags: Record<string, number> = {};

    for (const filePath in hunks) {
      for (const h of hunks[filePath]) {
        const cls = classifications[h.id];
        if (!cls) continue;

        types[cls.changeType] = (types[cls.changeType] ?? 0) + 1;

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

  /** Attention tags actually present in this PR (non-zero count). */
  const presentTags = $derived(
    ATTENTION_TAGS.filter((tg) => stats.tags[tg.id]),
  );

  /** Change types actually present (excludes the "all" sentinel). */
  const presentTypes = $derived(
    CHANGE_TYPES.filter((c) => c.id !== "all" && stats.types[c.id]),
  );

  // ── Show section counts ───────────────────────────────────────────────────

  const total = $derived(files.length);
  const reviewedCount = $derived(reviewedSet.size);
  const unreviewedCount = $derived(total - reviewedCount);

  const showRows = $derived([
    { id: "all", label: "All files", count: total, active: true },
    { id: "reviewed", label: "Reviewed", count: reviewedCount, active: false },
    {
      id: "unreviewed",
      label: "Unreviewed",
      count: unreviewedCount,
      active: false,
    },
  ]);

  // ── Per-file attention tags ───────────────────────────────────────────────

  /** Collects unique attention tags across all hunks in a file. */
  function fileAttentionTags(filePath: string): string[] {
    const fileHunks = hunks[filePath] ?? [];
    const set = new Set<string>();
    for (const h of fileHunks) {
      const cls = classifications[h.id];
      if (cls) {
        for (const tg of cls.attentionTags) set.add(tg);
      }
    }
    return [...set];
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
  <!-- Show section -->
  <div class="mb-1 flex items-center justify-between">
    <SidebarLabel>Show</SidebarLabel>
    <button
      type="button"
      title="Search"
      class="cursor-pointer border-none bg-transparent p-0.5 text-ed-text-faint"
    >
      <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
        <circle cx="6" cy="6" r="4" stroke="currentColor" stroke-width="1.4" />
        <path
          d="M9 9 L12 12"
          stroke="currentColor"
          stroke-width="1.4"
          stroke-linecap="round"
        />
      </svg>
    </button>
  </div>

  {#each showRows as row}
    <SidebarRow active={row.active}>
      <span>{row.label}</span>
      <span
        class="text-ed-text-faint"
        style="font-family: var(--font-mono); font-size: 11px;"
      >
        {row.count}
      </span>
    </SidebarRow>
  {/each}

  <!-- Categories section -->
  <div class="mt-[22px]">
    <SidebarLabel>Categories</SidebarLabel>
  </div>

  <!-- Attention tags with colored swatches -->
  {#each presentTags as tg}
    {@const color = edTagColor(tg.id, isDark)}
    <SidebarRow>
      <span class="inline-flex items-center gap-2">
        <span
          style="
            width: 8px; height: 8px; border-radius: 2px;
            background: {color ?? 'var(--ed-text-faint)'};
            flex-shrink: 0;
          "
        ></span>
        <span>{tg.label}</span>
      </span>
      <span
        class="text-ed-text-faint"
        style="font-family: var(--font-mono); font-size: 11px;"
      >
        {stats.tags[tg.id] ?? 0}
      </span>
    </SidebarRow>
  {/each}

  <!-- Change types with outline dots -->
  {#each presentTypes as ct}
    <SidebarRow>
      <span class="inline-flex items-center gap-2">
        <span
          class="inline-block flex-shrink-0"
          style="
            width: 8px; height: 8px; border-radius: 50%;
            border: 1.4px solid var(--ed-text-faint);
          "
        ></span>
        <span>{ct.label}</span>
      </span>
      <span
        class="text-ed-text-faint"
        style="font-family: var(--font-mono); font-size: 11px;"
      >
        {stats.types[ct.id] ?? 0}
      </span>
    </SidebarRow>
  {/each}

  <!-- Files section -->
  <div class="mt-[22px]">
    <SidebarLabel>Files</SidebarLabel>
  </div>

  {#each files as file, i}
    {@const isActive = i === activeFileIndex}
    {@const isReviewed = reviewedSet.has(file.path)}
    {@const fileTags = fileAttentionTags(file.path)}
    <button
      type="button"
      onclick={() => onSelectFile(i)}
      class="mb-0.5 w-full cursor-pointer overflow-hidden rounded-md border-none px-2.5 py-2 text-left"
      style="
        margin: 0 -10px 2px;
        width: calc(100% + 20px);
        background: {isActive ? 'var(--ed-accent-soft)' : 'transparent'};
        border-left: 2px solid {isActive ? 'var(--ed-accent)' : 'transparent'};
      "
    >
      <div
        class="overflow-hidden text-ellipsis whitespace-nowrap text-[12.5px] font-medium"
        style="
          font-family: var(--font-sans);
          color: {isActive ? 'var(--ed-accent)' : 'var(--ed-text)'};
          text-decoration: {isReviewed ? 'line-through' : 'none'};
          opacity: {isReviewed ? 0.6 : 1};
        "
      >
        {file.short}
      </div>
      <div class="mt-1 flex flex-wrap items-center gap-2">
        <span
          class="text-ed-added"
          style="font-family: var(--font-mono); font-size: 10.5px;"
        >
          +{file.adds}
        </span>
        <span
          class="text-ed-removed"
          style="font-family: var(--font-mono); font-size: 10.5px;"
        >
          −{file.dels}
        </span>
        {#each fileTags.slice(0, 2) as tgId}
          <TagPill id={tgId} mini />
        {/each}
      </div>
    </button>
  {/each}
</div>
