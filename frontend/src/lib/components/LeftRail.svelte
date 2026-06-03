<script lang="ts">
  /**
   * Left rail for the review screen.
   *
   * Contains:
   * 0. "Back to PR list" button (returns to the selection screen).
   * 1. PR metadata (number, author, title).
   * 2. Progress bar showing how many files have been reviewed.
   * 3. Tab switcher (Files | Filters) and the corresponding tab content.
   * 4. "Suggest filters" CTA at the bottom (non-functional stub).
   *
   * Translates `BLeftRail` from `concept-b.jsx` to Svelte.
   */

  import FilesTab from "./FilesTab.svelte";
  import FiltersTab from "./FiltersTab.svelte";
  import type {
    FileEntry,
    FilterState,
    HunkData,
    Classification,
  } from "../types.js";

  interface Props {
    /** PR number shown in the meta section. */
    prNumber: number;
    /** PR author username. */
    prAuthor: string;
    /** PR title. */
    prTitle: string;
    /** Full list of files in the PR. */
    files: FileEntry[];
    /** Hunk data keyed by file path. */
    hunks: Record<string, HunkData[]>;
    /** Classification data keyed by hunk id. */
    classifications: Record<string, Classification>;
    /** Set of file paths currently marked as reviewed. */
    reviewedSet: Set<string>;
    /** Index of the focused file in the main panel. */
    activeFileIndex: number;
    /** Called when the user clicks a file row. */
    onSelectFile: (index: number) => void;
    /** Current filter state. */
    filter: FilterState;
    /** Called when the user changes a filter. */
    onFilterChange: (next: FilterState) => void;
    /** Called when the user clicks "back to PR list". */
    onBack: () => void;
    /**
     * Whether analysis has completed. When false, the analysis-dependent UI
     * (Filters tab, "Suggest filters") is greyed out and disabled.
     */
    analyzed?: boolean;
  }

  let {
    prNumber,
    prAuthor,
    prTitle,
    files,
    hunks,
    classifications,
    reviewedSet,
    activeFileIndex,
    onSelectFile,
    filter,
    onFilterChange,
    onBack,
    analyzed = true,
  }: Props = $props();

  // ── Tab state ─────────────────────────────────────────────────────────────

  type Tab = "files" | "filters";
  let activeTab = $state<Tab>("files");

  // Filters require analysis; fall back to the Files tab when not analysed.
  $effect(() => {
    if (!analyzed && activeTab === "filters") activeTab = "files";
  });

  // ── Progress ──────────────────────────────────────────────────────────────

  const reviewedCount = $derived(reviewedSet.size);
  const totalFiles = $derived(files.length);
  const progressPct = $derived(
    totalFiles > 0 ? (reviewedCount / totalFiles) * 100 : 0,
  );
</script>

<div
  class="flex flex-col overflow-hidden"
  style="background: var(--ed-bg);"
>
  <!-- Back to PR list -->
  <button
    type="button"
    onclick={onBack}
    class="flex items-center gap-1.5 bg-transparent border-none cursor-pointer text-ed-text-muted hover:text-ed-text"
    style="padding: 14px 18px 0; font-family: var(--font-sans); font-size: 12px;"
  >
    <svg width="13" height="13" viewBox="0 0 14 14" fill="none" aria-hidden="true">
      <path
        d="M9 3 L4 7 L9 11"
        stroke="currentColor"
        stroke-width="1.6"
        stroke-linecap="round"
        stroke-linejoin="round"
      />
    </svg>
    All pull requests
  </button>

  <!-- PR meta -->
  <div class="px-[18px] pb-4 pt-[18px]">
    <div
      class="text-ed-text-faint"
      style="font-family: var(--font-mono); font-size: 11.5px;"
    >
      #{prNumber} · @{prAuthor}
    </div>
    <div
      class="mt-1.5 font-semibold leading-snug text-ed-text"
      style="font-size: 14.5px; letter-spacing: -0.1px; font-family: var(--font-sans);"
    >
      {prTitle}
    </div>

    <!-- Progress bar -->
    <div class="mt-3.5 flex items-center gap-2.5">
      <div
        class="h-1 flex-1 overflow-hidden rounded-full"
        style="background: var(--ed-border-subtle);"
      >
        <div
          class="h-full rounded-full transition-[width] duration-300"
          style="width: {progressPct}%; background: var(--ed-added);"
        ></div>
      </div>
      <div
        class="shrink-0 text-ed-text-muted"
        style="font-family: var(--font-mono); font-size: 11px; white-space: nowrap;"
      >
        {reviewedCount}/{totalFiles} files reviewed
      </div>
    </div>
  </div>

  <!-- Tab switcher -->
  <div
    class="flex px-[18px]"
    style="border-bottom: 1px solid var(--ed-border-subtle);"
  >
    <button
      type="button"
      onclick={() => (activeTab = "files")}
      class="mr-[18px] cursor-pointer border-none bg-transparent py-2.5 text-[13px]"
      style="
        font-family: var(--font-sans);
        font-weight: {activeTab === 'files' ? 600 : 500};
        color: {activeTab === 'files' ? 'var(--ed-text)' : 'var(--ed-text-muted)'};
        border-bottom: 2px solid {activeTab === 'files' ? 'var(--ed-accent)' : 'transparent'};
        padding-bottom: 10px;
      "
    >
      Files
    </button>
    <button
      type="button"
      onclick={() => analyzed && (activeTab = "filters")}
      disabled={!analyzed}
      title={analyzed ? undefined : "Run analysis to enable filters"}
      class="border-none bg-transparent py-2.5 text-[13px]"
      style="
        font-family: var(--font-sans);
        font-weight: {activeTab === 'filters' ? 600 : 500};
        color: {activeTab === 'filters' ? 'var(--ed-text)' : 'var(--ed-text-muted)'};
        border-bottom: 2px solid {activeTab === 'filters' ? 'var(--ed-accent)' : 'transparent'};
        padding-bottom: 10px;
        cursor: {analyzed ? 'pointer' : 'not-allowed'};
        opacity: {analyzed ? 1 : 0.45};
      "
    >
      Filters
    </button>
  </div>

  <!-- Tab content (scrollable) -->
  <div class="min-h-0 flex-1 overflow-auto">
    {#if activeTab === "files"}
      <FilesTab
        {files}
        {hunks}
        {classifications}
        {reviewedSet}
        {activeFileIndex}
        {onSelectFile}
      />
    {:else}
      <FiltersTab {filter} {onFilterChange} {hunks} {classifications} />
    {/if}
  </div>

  <!-- Suggest filters CTA (non-functional stub) -->
  <div
    class="px-[18px] py-3"
    style="border-top: 1px solid var(--ed-border-subtle);"
  >
    <button
      type="button"
      disabled={!analyzed}
      title={analyzed ? undefined : "Run analysis to enable filter suggestions"}
      class="inline-flex w-full items-center justify-center gap-1.5 rounded-md bg-transparent text-[12.5px] font-medium text-ed-accent"
      style="
        padding: 8px 10px;
        border: 1px dashed var(--ed-border);
        font-family: var(--font-sans);
        white-space: nowrap;
        cursor: {analyzed ? 'pointer' : 'not-allowed'};
        opacity: {analyzed ? 1 : 0.45};
      "
    >
      <span style="font-size: 14px; line-height: 1;">+</span>
      Suggest filters
    </button>
  </div>
</div>
