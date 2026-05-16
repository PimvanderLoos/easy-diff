<script lang="ts">
  /**
   * Top-level review screen layout.
   *
   * Three-column CSS grid:
   *   - Left rail (284px): PR metadata, review progress, Files/Filters tabs.
   *   - Main panel (1fr): MainToolbar + diff stack (file panels).
   *   - Inspector panel (340px, optional): hunk inspector (stub for PR-3).
   *
   * Wires together:
   * - `run_analysis` / `get_analysis` Tauri commands for LLM classification.
   * - `get_diff` Tauri command for the raw parsed diff.
   * - MainToolbar for file navigation and view mode toggle.
   * - FilePanel for each file's collapsible diff card.
   *
   * Translates `BReview` from `concept-b.jsx` to Svelte.
   */

  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { selectedPr, filterState, reviewedFiles, diffViewMode } from "../stores.js";
  import LeftRail from "./LeftRail.svelte";
  import MainToolbar from "./MainToolbar.svelte";
  import FilePanel from "./FilePanel.svelte";
  import type {
    AnalysisResult,
    FileEntry,
    FilterState,
    HunkData,
    DiffLineData,
    Classification,
  } from "../types.js";

  // ── PR data (from parent store) ───────────────────────────────────────────

  const pr = $derived($selectedPr);

  // ── Analysis state ────────────────────────────────────────────────────────

  let analysis = $state<AnalysisResult | null>(null);
  let analysisError = $state<string | null>(null);
  let analysisLoading = $state(false);

  // ── Raw diff state ────────────────────────────────────────────────────────

  /**
   * Raw diff file list from the `get_diff` Tauri command.
   * Each entry has the shape serialised from Rust's `DiffFile`.
   */
  interface RawDiffLine {
    Context?: string;
    Added?: string;
    Removed?: string;
  }

  interface RawDiffHunk {
    header: string;
    old_start: number;
    old_count: number;
    new_start: number;
    new_count: number;
    lines: RawDiffLine[];
  }

  interface RawDiffFile {
    path: string;
    old_path: string | null;
    hunks: RawDiffHunk[];
    is_new: boolean;
    is_deleted: boolean;
  }

  let rawDiff = $state<RawDiffFile[]>([]);

  onMount(async () => {
    if (!pr) return;

    // Load cached analysis, or trigger a fresh run.
    try {
      const cached = await invoke<AnalysisResult | null>("get_analysis", {
        prNumber: pr.number,
      });
      if (cached) {
        analysis = cached;
      }
    } catch {
      // No cached result; proceed to run analysis.
    }

    if (!analysis) {
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
    }

    // Fetch the parsed diff independently (no LLM needed).
    try {
      rawDiff = await invoke<RawDiffFile[]>("get_diff", {
        prNumber: pr.number,
      });
    } catch {
      // Diff unavailable — file panels will show empty hunks.
    }
  });

  // ── Convert raw diff into frontend HunkData ───────────────────────────────

  /**
   * Convert a raw DiffLine (Rust enum serialised as a tagged object) into the
   * `DiffLineData` shape used by the diff renderer.
   *
   * Old/new line numbers are computed incrementally as the hunk is walked.
   */
  function convertLines(
    rawLines: RawDiffLine[],
    oldStart: number,
    newStart: number,
  ): DiffLineData[] {
    let oldN = oldStart;
    let newN = newStart;
    return rawLines.map((raw): DiffLineData => {
      if (raw.Added !== undefined) {
        const line: DiffLineData = {
          type: "add",
          old: null,
          new: newN,
          text: raw.Added,
        };
        newN++;
        return line;
      }
      if (raw.Removed !== undefined) {
        const line: DiffLineData = {
          type: "del",
          old: oldN,
          new: null,
          text: raw.Removed,
        };
        oldN++;
        return line;
      }
      // Context
      const text =
        (raw as { Context?: string }).Context ??
        Object.values(raw)[0] ??
        "";
      const line: DiffLineData = {
        type: "ctx",
        old: oldN,
        new: newN,
        text,
      };
      oldN++;
      newN++;
      return line;
    });
  }

  /** Build HunkData[] from a single RawDiffFile. */
  function buildHunks(rawFile: RawDiffFile): HunkData[] {
    return rawFile.hunks.map((rh, idx): HunkData => ({
      id: `${rawFile.path}:${idx}`,
      header: rh.header,
      oldStart: rh.old_start,
      newStart: rh.new_start,
      isNewFile: rawFile.is_new,
      lines: convertLines(rh.lines, rh.old_start, rh.new_start),
    }));
  }

  // ── Derived: FileEntry[] and hunk/classification maps ─────────────────────

  const files = $derived.by<FileEntry[]>(() => {
    if (rawDiff.length > 0) {
      return rawDiff.map((rf): FileEntry => {
        const parts = rf.path.split("/");
        const short = parts[parts.length - 1] ?? rf.path;
        const dir = parts.slice(0, -1).join("/");
        const hunkLines = rf.hunks.flatMap((h) => h.lines);
        const adds = hunkLines.filter((l) => l.Added !== undefined).length;
        const dels = hunkLines.filter((l) => l.Removed !== undefined).length;
        const analysisFile = analysis?.files[rf.path];
        const risky =
          (analysisFile?.attention_tags ?? []).includes("security-sensitive") ||
          (analysisFile?.attention_tags ?? []).includes("flawed-code");
        return {
          path: rf.path,
          short,
          dir,
          adds,
          dels,
          risky,
          reviewed: false,
          isNew: rf.is_new,
        };
      });
    }
    // Fallback: derive from analysis keys (no add/del counts).
    if (analysis) {
      return Object.keys(analysis.files).map((path): FileEntry => {
        const parts = path.split("/");
        const short = parts[parts.length - 1] ?? path;
        const dir = parts.slice(0, -1).join("/");
        return { path, short, dir, adds: 0, dels: 0, risky: false, reviewed: false };
      });
    }
    return [];
  });

  const hunks = $derived.by<Record<string, HunkData[]>>(() => {
    const map: Record<string, HunkData[]> = {};
    for (const rf of rawDiff) {
      map[rf.path] = buildHunks(rf);
    }
    return map;
  });

  /**
   * Build a stub classification from Pass2 data for each hunk.
   *
   * Real per-hunk classification (Pass 2) uses hunk ids. Since the backend
   * doesn't yet expose per-hunk ids in the analysis result, we fall back to
   * applying the file-level Pass2 data to every hunk in that file.
   */
  const classifications = $derived.by<Record<string, Classification>>(() => {
    const map: Record<string, Classification> = {};
    if (!analysis) return map;
    for (const rf of rawDiff) {
      const p2 = analysis.files[rf.path];
      if (!p2) continue;
      const cls: Classification = {
        changeType: p2.change_types[0] ?? "uncategorized",
        attentionTags: p2.attention_tags,
        confidence: 0.8,
        summary: p2.summary,
        rationale: p2.details.join(" "),
      };
      const fileHunks = buildHunks(rf);
      for (const h of fileHunks) {
        map[h.id] = cls;
      }
    }
    return map;
  });

  // ── Filter state ──────────────────────────────────────────────────────────

  const filter = $derived<FilterState>($filterState);

  function handleFilterChange(next: FilterState) {
    filterState.set(next);
  }

  // ── Review tracking ───────────────────────────────────────────────────────

  const reviewedSet = $derived($reviewedFiles);

  let collapsedSet = $state<Set<string>>(new Set());

  function toggleReviewed(path: string) {
    reviewedFiles.update((prev) => {
      const next = new Set(prev);
      const willBeReviewed = !next.has(path);
      if (willBeReviewed) {
        next.add(path);
        // Auto-collapse when marked reviewed.
        collapsedSet = new Set([...collapsedSet, path]);
      } else {
        next.delete(path);
        // Re-expand when un-reviewed.
        const cs = new Set(collapsedSet);
        cs.delete(path);
        collapsedSet = cs;
      }
      return next;
    });
  }

  function toggleCollapsed(path: string) {
    const next = new Set(collapsedSet);
    if (next.has(path)) {
      next.delete(path);
    } else {
      next.add(path);
    }
    collapsedSet = next;
  }

  // ── File navigation ───────────────────────────────────────────────────────

  let activeFileIndex = $state(0);

  function handleSelectFile(index: number) {
    activeFileIndex = index;
  }

  function handlePrev() {
    if (activeFileIndex > 0) activeFileIndex--;
  }

  function handleNext() {
    if (activeFileIndex < files.length - 1) activeFileIndex++;
  }

  // ── View mode (driven by the diffViewMode store) ──────────────────────────

  const diffView = $derived($diffViewMode);

  // ── Inspector toggle (stubbed for PR-3) ───────────────────────────────────

  const inspectorOpen = $state(false);

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
    files={files}
    hunks={hunks}
    classifications={classifications}
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
    <MainToolbar
      fileIndex={activeFileIndex}
      totalFiles={files.length}
      {diffView}
      onDiffViewChange={(v) => diffViewMode.set(v as "inline" | "split")}
      onPrev={handlePrev}
      onNext={handleNext}
    />

    <!-- Diff stack / loading / error states -->
    <div
      class="flex flex-1 flex-col overflow-auto"
      style="padding: 20px 24px 60px; gap: 16px;"
    >
      {#if analysisLoading}
        <div
          class="flex flex-1 items-center justify-center text-ed-text-muted"
          style="font-size: 14px; font-family: var(--font-sans);"
        >
          Running analysis…
        </div>
      {:else if analysisError}
        <div
          class="flex flex-1 items-center justify-center text-ed-removed"
          style="font-size: 13px; font-family: var(--font-sans);"
        >
          {analysisError}
        </div>
      {:else if files.length > 0}
        {#each files as file (file.path)}
          <FilePanel
            {file}
            hunks={hunks[file.path] ?? []}
            classifications={classifications}
            reviewed={reviewedSet.has(file.path)}
            collapsed={collapsedSet.has(file.path)}
            onToggleReviewed={() => toggleReviewed(file.path)}
            onToggleCollapsed={() => toggleCollapsed(file.path)}
            {isDark}
            {filter}
            {diffView}
          />
        {/each}
      {:else}
        <div
          class="flex flex-1 items-center justify-center text-ed-text-faint"
          style="font-size: 13px; font-family: var(--font-sans);"
        >
          No files to display
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
