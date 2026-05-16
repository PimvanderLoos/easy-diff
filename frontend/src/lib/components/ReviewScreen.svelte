<script lang="ts">
  /**
   * Top-level review screen layout.
   *
   * Three-column CSS grid:
   *   - Left rail (284px): PR metadata, review progress, Files/Filters tabs.
   *   - Main panel (1fr): toolbar + diff stack (stub for now; populated in PR-1).
   *   - Inspector panel (340px, optional): hunk inspector (populated in PR-3).
   *
   * Inspector column is hidden by default and will be toggled in PR-3.
   *
   * Translates `BReview` from `concept-b.jsx` to Svelte.
   */

  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { selectedPr, filterState, reviewedFiles } from "../stores.js";
  import LeftRail from "./LeftRail.svelte";
  import type {
    AnalysisResult,
    FileEntry,
    FilterState,
    HunkData,
    Classification,
  } from "../types.js";

  // ── PR data (from parent store) ───────────────────────────────────────────

  const pr = $derived($selectedPr);

  // ── Analysis state ────────────────────────────────────────────────────────

  let analysis = $state<AnalysisResult | null>(null);
  let analysisError = $state<string | null>(null);
  let analysisLoading = $state(false);

  onMount(async () => {
    if (!pr) return;

    // Try to load a cached result first so the UI is responsive.
    try {
      const cached = await invoke<AnalysisResult | null>("get_analysis", {
        prNumber: pr.number,
      });
      if (cached) {
        analysis = cached;
        return;
      }
    } catch {
      // No cached result; proceed to run analysis.
    }

    // No cache — trigger full analysis.
    analysisLoading = true;
    try {
      analysis = await invoke<AnalysisResult>("run_analysis", {
        prNumber: pr.number,
      });
    } catch (e) {
      analysisError = String(e);
    } finally {
      analysisLoading = false;
    }
  });

  // ── File + hunk data derived from analysis ────────────────────────────────

  /**
   * Build `FileEntry[]` from the analysis result.
   *
   * When analysis is available, enrich each file with per-file adds/dels from
   * the Pass 2 output. For now the adds/dels default to 0 (populated in PR-1
   * when the parsed diff is fetched and wired in).
   */
  const files = $derived<FileEntry[]>(
    analysis
      ? Object.keys(analysis.files).map((path) => {
          const parts = path.split("/");
          const short = parts[parts.length - 1] ?? path;
          const dir = parts.slice(0, -1).join("/");
          return {
            path,
            short,
            dir,
            adds: 0,
            dels: 0,
            risky: false,
            reviewed: false,
          };
        })
      : [],
  );

  /**
   * Empty hunk/classification maps — populated in PR-1 when `get_diff` is
   * wired in. For now they are stubs so the left rail renders correctly.
   */
  const hunks = $derived<Record<string, HunkData[]>>({});
  const classifications = $derived<Record<string, Classification>>({});

  // ── Filter state ──────────────────────────────────────────────────────────

  const filter = $derived<FilterState>($filterState);

  function handleFilterChange(next: FilterState) {
    filterState.set(next);
  }

  // ── Review tracking ───────────────────────────────────────────────────────

  const reviewedSet = $derived($reviewedFiles);

  // ── File navigation ───────────────────────────────────────────────────────

  let activeFileIndex = $state(0);

  function handleSelectFile(index: number) {
    activeFileIndex = index;
  }

  // ── Inspector toggle (stubbed for PR-3) ───────────────────────────────────

  const inspectorOpen = $state(false);
</script>

<div
  class="flex-1 overflow-hidden"
  style="
    display: grid;
    grid-template-columns: {inspectorOpen ? '284px 1fr 340px' : '284px 1fr'};
    background: var(--ed-bg);
  "
>
  <!-- Left rail -->
  <LeftRail
    prNumber={pr?.number ?? 0}
    prAuthor={pr?.author ?? ""}
    prTitle={pr?.title ?? ""}
    {files}
    {hunks}
    {classifications}
    {reviewedSet}
    {activeFileIndex}
    onSelectFile={handleSelectFile}
    {filter}
    onFilterChange={handleFilterChange}
  />

  <!-- Main diff area -->
  <div
    class="flex flex-col overflow-hidden"
    style="
      border-left: 1px solid var(--ed-border-subtle);
      {inspectorOpen ? 'border-right: 1px solid var(--ed-border-subtle);' : ''}
      background: var(--ed-panel);
    "
  >
    <!-- Stub toolbar — replaced in PR-1 -->
    <div
      class="flex shrink-0 items-center justify-between gap-3.5 px-6"
      style="
        height: 48px;
        border-bottom: 1px solid var(--ed-border-subtle);
        background: var(--ed-bg);
      "
    >
      <span
        class="text-ed-text-muted"
        style="font-family: var(--font-mono); font-size: 12px;"
      >
        {files.length} file{files.length !== 1 ? "s" : ""}
      </span>
    </div>

    <!-- Diff area placeholder -->
    <div
      class="flex flex-1 flex-col items-center justify-center gap-3 overflow-auto"
    >
      {#if analysisLoading}
        <div
          class="text-ed-text-muted"
          style="font-size: 14px; font-family: var(--font-sans);"
        >
          Running analysis…
        </div>
      {:else if analysisError}
        <div
          class="text-ed-removed"
          style="font-size: 13px; font-family: var(--font-sans);"
        >
          {analysisError}
        </div>
      {:else if analysis}
        <div
          class="text-ed-text-muted"
          style="font-size: 13px; font-family: var(--font-sans);"
        >
          Diff view — coming in PR-1
        </div>
        <div
          class="max-w-xl rounded-md px-4 py-3 text-ed-text-muted"
          style="
            font-size: 12px;
            font-family: var(--font-mono);
            background: var(--ed-elevated);
            border: 1px solid var(--ed-border-subtle);
          "
        >
          {analysis.pass1.summary}
        </div>
      {:else}
        <div
          class="text-ed-text-faint"
          style="font-size: 13px; font-family: var(--font-sans);"
        >
          No analysis yet
        </div>
      {/if}
    </div>
  </div>

  <!-- Inspector panel (stub — rendered in PR-3) -->
  {#if inspectorOpen}
    <div
      class="overflow-auto"
      style="background: var(--ed-bg);"
    >
      <div
        class="px-5 py-3.5 text-ed-text"
        style="font-size: 14px; font-weight: 600;"
      >
        Inspector
      </div>
    </div>
  {/if}
</div>
