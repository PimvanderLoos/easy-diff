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
  import {
    currentScreen,
    selectedPr,
    filterState,
    reviewedFiles,
    expandedFiles,
    focusedHunkId,
    diffViewMode,
    draftComments,
    categoryOverrides,
    resetReviewSession,
    helpOpen,
    reportError,
    analysisResult,
    analysisStatus,
    analysisErrorMsg,
  } from "../stores.js";
  import { runAnalysis } from "../analysisController.js";
  import LeftRail from "./LeftRail.svelte";
  import MainToolbar from "./MainToolbar.svelte";
  import FilePanel from "./FilePanel.svelte";
  import InspectorPanel from "./InspectorPanel.svelte";
  import SubmitReviewDialog from "./SubmitReviewDialog.svelte";
  import type {
    AnalysisResult,
    FileEntry,
    FilterState,
    HunkData,
    DiffLineData,
    Classification,
    ReviewComment,
    CategoryOverride,
  } from "../types.js";

  // ── PR data (from parent store) ───────────────────────────────────────────

  const pr = $derived($selectedPr);

  // ── Analysis state (driven by the analysis stores) ────────────────────────

  /** Latest analysis result, or null until the user triggers analysis. */
  const analysis = $derived($analysisResult);

  /**
   * True whenever an analysis result is available — gates the category UI.
   * Keyed on the result, not the status, so re-running analysis (status
   * `"running"`/`"error"`) does not disable the Filters tab while a valid
   * prior result is still on screen.
   */
  const analyzed = $derived($analysisResult !== null);

  /** Error from the last failed run, shown as a dismissible banner. */
  const analysisErr = $derived($analysisErrorMsg);

  // Reset the shared analysis stores synchronously at init — before the first
  // render — so this PR never briefly inherits the prior PR's analysis state.
  // (The TitleBar sibling reads these same global stores during that render.)
  // A previously cached result is re-surfaced from `onMount` below.
  analysisResult.set(null);
  analysisStatus.set("idle");
  analysisErrorMsg.set(null);

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

  // ── Comment and override state ────────────────────────────────────────────

  /** Local reactive snapshot of the draftComments store. */
  const comments = $derived($draftComments);

  /** Local reactive snapshot of the categoryOverrides store. */
  const overrides = $derived($categoryOverrides);

  onMount(async () => {
    if (!pr) return;

    // Fetch the parsed diff first so the code renders immediately (no LLM).
    try {
      rawDiff = await invoke<RawDiffFile[]>("get_diff", {
        prNumber: pr.number,
      });
    } catch (e) {
      // Diff unavailable — file panels will show empty hunks. Surface it.
      reportError("Could not load the diff for this PR", e);
    }

    // Surface a previously cached analysis without re-running it. Analysis is
    // otherwise user-triggered via the TitleBar "Analyze" button.
    try {
      const cached = await invoke<AnalysisResult | null>("get_analysis", {
        prNumber: pr.number,
      });
      if (cached) {
        analysisResult.set(cached);
        analysisStatus.set("done");
      }
    } catch (e) {
      // A cache miss returns null (no throw); a thrown error is a real failure.
      reportError("Could not load cached analysis", e);
    }

    // Load draft comments and category overrides from the local cache.
    const prId = String(pr.number);
    try {
      const loaded = await invoke<ReviewComment[]>("list_comments", { prId });
      draftComments.set(loaded);
    } catch (e) {
      reportError("Could not load saved draft comments", e);
    }
    try {
      const loaded = await invoke<CategoryOverride[]>("get_category_overrides", {
        prId,
      });
      categoryOverrides.set(
        new Map(loaded.map((ov) => [ov.hunk_id, ov])),
      );
    } catch (e) {
      reportError("Could not load saved category overrides", e);
    }

    // Restore persisted reviewed-file state (files reviewed at the current head SHA).
    // Reset first: reviewedFiles is a shared module store reused across PR mounts,
    // so a failed restore must not leak the previously opened PR's reviewed set.
    reviewedFiles.set(new Set());
    try {
      const reviewed = await invoke<string[]>("get_reviewed_files", {
        prId,
        headSha: pr.head_sha,
      });
      reviewedFiles.set(new Set(reviewed));
    } catch (e) {
      reportError("Could not restore reviewed-file state", e);
    }
  });

  // ── Analysis trigger handlers ─────────────────────────────────────────────

  /** Re-run analysis after a failure (from the error banner's Retry button). */
  function handleRetryAnalysis() {
    if (pr) runAnalysis(pr.number);
  }

  /** Dismiss the error banner, restoring the prior status. */
  function dismissAnalysisError() {
    analysisErrorMsg.set(null);
    analysisStatus.set(analysis ? "done" : "idle");
  }

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
   * Build a stub classification from Pass2 data for each hunk, then apply any
   * manual category overrides from the `categoryOverrides` store.
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
        const ov = overrides.get(h.id);
        if (ov) {
          map[h.id] = {
            ...cls,
            changeType: ov.change_type ?? cls.changeType,
            attentionTags: ov.attention_tags ?? cls.attentionTags,
          };
        } else {
          map[h.id] = cls;
        }
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
  const expandedSet = $derived($expandedFiles);

  /**
   * Serial queue for `set_reviewed` persistence writes. Each toggle is chained
   * onto the previous one so the writes complete in enqueue order, even when the
   * user toggles rapidly. Without this, in-flight `set_reviewed` invocations can
   * complete out of order and leave the persisted SQLite state inconsistent with
   * the final UI state. The chain is never awaited, so the UI is not blocked.
   */
  let reviewedWriteQueue: Promise<unknown> = Promise.resolve();

  /**
   * Applies the reviewed state for `path` to the in-memory stores: updates the
   * reviewed set and auto-collapses (when reviewed) or re-expands (when not).
   * Used for both the optimistic update and the revert when persistence fails.
   */
  function applyReviewed(path: string, reviewed: boolean) {
    reviewedFiles.update((prev) => {
      const next = new Set(prev);
      if (reviewed) {
        next.add(path);
        // Auto-collapse when marked reviewed.
        expandedFiles.update((es) => {
          const nes = new Set(es);
          nes.delete(path);
          return nes;
        });
      } else {
        next.delete(path);
        // Re-expand when un-reviewed.
        expandedFiles.update((es) => new Set([...es, path]));
      }
      return next;
    });
  }

  function toggleReviewed(path: string) {
    const willBeReviewed = !reviewedSet.has(path);

    // Optimistically update the UI immediately for responsiveness.
    applyReviewed(path, willBeReviewed);

    if (!pr) return;

    // Persist reviewed state, keyed to the current head SHA so it survives
    // reopening the PR. Serialized via reviewedWriteQueue so the last toggle
    // wins. On failure we never fail silently: revert the optimistic UI so it
    // reflects what was actually stored, and surface the error to the user.
    const prId = String(pr.number);
    const headSha = pr.head_sha;
    reviewedWriteQueue = reviewedWriteQueue.catch(() => {}).then(() =>
      invoke("set_reviewed", {
        prId,
        filePath: path,
        headSha,
        reviewed: willBeReviewed,
      }).catch((e) => {
        applyReviewed(path, !willBeReviewed);
        reportError(
          `Could not ${willBeReviewed ? "mark" : "unmark"} "${path}" as reviewed — reverted`,
          e,
        );
      }),
    );
  }

  function toggleExpanded(path: string) {
    expandedFiles.update((prev) => {
      const next = new Set(prev);
      if (next.has(path)) {
        next.delete(path);
      } else {
        next.add(path);
      }
      return next;
    });
  }

  // ── Comment handlers ─────────────────────────────────────────────────────

  function handleCommentAdded(comment: ReviewComment) {
    draftComments.update((prev) => [...prev, comment]);
  }

  function handleCommentUpdated(comment: ReviewComment) {
    draftComments.update((prev) =>
      prev.map((c) => (c.id === comment.id ? comment : c)),
    );
  }

  function handleCommentDeleted(id: number) {
    draftComments.update((prev) => prev.filter((c) => c.id !== id));
  }

  async function handleOverrideCategory(hunkId: string, changeType: string) {
    if (!pr) return;
    const prId = String(pr.number);
    // Extract file path from hunk id ("{filePath}:{index}").
    const colonIdx = hunkId.lastIndexOf(":");
    const filePath = colonIdx >= 0 ? hunkId.slice(0, colonIdx) : hunkId;
    try {
      await invoke("set_category_override", {
        prId,
        filePath,
        hunkId,
        changeType,
        attentionTags: null,
      });
      categoryOverrides.update((prev) => {
        const next = new Map(prev);
        const existing = next.get(hunkId);
        next.set(hunkId, {
          id: existing?.id ?? 0,
          pr_id: prId,
          file_path: filePath,
          hunk_id: hunkId,
          change_type: changeType,
          attention_tags: existing?.attention_tags ?? null,
        });
        return next;
      });
    } catch (e) {
      reportError(`Could not change the category for "${filePath}"`, e);
    }
  }

  async function handleOverrideTags(hunkId: string, tags: string[]) {
    if (!pr) return;
    const prId = String(pr.number);
    const colonIdx = hunkId.lastIndexOf(":");
    const filePath = colonIdx >= 0 ? hunkId.slice(0, colonIdx) : hunkId;
    try {
      await invoke("set_category_override", {
        prId,
        filePath,
        hunkId,
        changeType: null,
        attentionTags: tags,
      });
      categoryOverrides.update((prev) => {
        const next = new Map(prev);
        const existing = next.get(hunkId);
        next.set(hunkId, {
          id: existing?.id ?? 0,
          pr_id: prId,
          file_path: filePath,
          hunk_id: hunkId,
          change_type: existing?.change_type ?? null,
          attention_tags: tags,
        });
        return next;
      });
    } catch (e) {
      reportError(`Could not change the tags for "${filePath}"`, e);
    }
  }

  // ── File navigation ───────────────────────────────────────────────────────

  let activeFileIndex = $state(0);

  function handleSelectFile(index: number) {
    activeFileIndex = index;
  }

  /** Returns to the PR list, clearing review-scoped state for the next PR. */
  function handleBack() {
    resetReviewSession();
    selectedPr.set(null);
    currentScreen.set("selection");
  }

  function handlePrev() {
    if (activeFileIndex > 0) activeFileIndex--;
  }

  function handleNext() {
    if (activeFileIndex < files.length - 1) activeFileIndex++;
  }

  // ── View mode (driven by the diffViewMode store) ──────────────────────────

  const diffView = $derived($diffViewMode);

  // ── Inspector (driven by focusedHunkId store) ─────────────────────────────

  const currentFocusedHunkId = $derived($focusedHunkId);
  const inspectorOpen = $derived(currentFocusedHunkId !== null);

  /** The file that owns the focused hunk (null when inspector is closed). */
  const focusedFile = $derived.by<FileEntry | null>(() => {
    if (!currentFocusedHunkId) return null;
    // Hunk ids are "{filePath}:{index}" — extract the file path prefix.
    const colonIdx = currentFocusedHunkId.lastIndexOf(":");
    const filePath = colonIdx >= 0
      ? currentFocusedHunkId.slice(0, colonIdx)
      : currentFocusedHunkId;
    return files.find((f) => f.path === filePath) ?? null;
  });

  /** The focused hunk itself (null when inspector is closed). */
  const focusedHunk = $derived.by<HunkData | null>(() => {
    if (!currentFocusedHunkId || !focusedFile) return null;
    return (hunks[focusedFile.path] ?? []).find(
      (h) => h.id === currentFocusedHunkId,
    ) ?? null;
  });

  /** Classification for the focused hunk (null when inspector is closed). */
  const focusedClassification = $derived<Classification | null>(
    currentFocusedHunkId ? (classifications[currentFocusedHunkId] ?? null) : null,
  );

  function handleFocusHunk(hunkId: string) {
    focusedHunkId.set(hunkId);
  }

  function handleCloseInspector() {
    focusedHunkId.set(null);
  }

  // ── Submit review dialog ──────────────────────────────────────────────────

  let submitDialogOpen = $state(false);

  const draftCount = $derived($draftComments.filter((c) => c.status === "Draft").length);

  function handleSubmitReviewClick() {
    submitDialogOpen = true;
  }

  // ── Global keyboard shortcuts ─────────────────────────────────────────────
  // Documented in KeyboardShortcutsDialog — keep the two in sync.

  /** True when a text-entry element is focused, so we should ignore shortcuts. */
  function isTypingTarget(target: EventTarget | null): boolean {
    if (!(target instanceof HTMLElement)) return false;
    const tag = target.tagName;
    return (
      tag === "INPUT" ||
      tag === "TEXTAREA" ||
      tag === "SELECT" ||
      target.isContentEditable
    );
  }

  function handleGlobalKeydown(e: KeyboardEvent) {
    // Skip shortcuts while typing, holding a modifier, or with a dialog open.
    if (
      isTypingTarget(e.target) ||
      e.ctrlKey ||
      e.metaKey ||
      e.altKey ||
      submitDialogOpen ||
      $helpOpen
    ) {
      return;
    }
    switch (e.key) {
      case "j":
        handleNext();
        break;
      case "k":
        handlePrev();
        break;
      case "v":
        diffViewMode.update((m) => (m === "inline" ? "split" : "inline"));
        break;
      case "s":
        handleSubmitReviewClick();
        break;
      case "?":
        helpOpen.set(true);
        break;
      default:
        return;
    }
    e.preventDefault();
  }

  function handleDialogClose() {
    submitDialogOpen = false;
  }

  /** After a successful submission, mark all draft comments as Submitted in the store. */
  function handleSubmitted() {
    draftComments.update((prev) =>
      prev.map((c) => (c.status === "Draft" ? { ...c, status: "Submitted" as const } : c)),
    );
    submitDialogOpen = false;
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

<svelte:window onkeydown={handleGlobalKeydown} />

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
    onBack={handleBack}
    {analyzed}
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
      {draftCount}
      onSubmitReview={handleSubmitReviewClick}
      onShowShortcuts={() => helpOpen.set(true)}
    />

    <!-- Diff stack — always shown; analysis runs separately in the background -->
    <div
      class="flex flex-1 flex-col overflow-auto"
      style="padding: 20px 24px 60px; gap: 16px;"
    >
      {#if analysisErr}
        <!-- Non-blocking analysis error: diff stays visible below. -->
        <div
          class="flex shrink-0 items-center gap-3"
          style="
            background: color-mix(in srgb, var(--ed-removed) 10%, transparent);
            border: 1px solid var(--ed-removed);
            border-radius: 6px;
            padding: 8px 12px;
            font-size: 12.5px;
            font-family: var(--font-sans);
            color: var(--ed-text);
          "
        >
          <span
            class="flex-1 min-w-0 truncate"
            title="Analysis failed: {analysisErr}"
          >Analysis failed: {analysisErr}</span>
          <button
            type="button"
            onclick={handleRetryAnalysis}
            class="shrink-0 cursor-pointer rounded border-none font-medium text-white"
            style="background: var(--ed-accent); padding: 3px 10px; font-size: 12px;"
          >
            Retry
          </button>
          <button
            type="button"
            onclick={dismissAnalysisError}
            class="shrink-0 cursor-pointer border-none bg-transparent text-ed-text-muted"
            style="font-size: 16px; line-height: 1;"
            aria-label="Dismiss"
          >
            ×
          </button>
        </div>
      {/if}
      {#if files.length > 0}
        {#each files as file (file.path)}
          <FilePanel
            {file}
            hunks={hunks[file.path] ?? []}
            classifications={classifications}
            reviewed={reviewedSet.has(file.path)}
            collapsed={!expandedSet.has(file.path)}
            focusedHunkId={currentFocusedHunkId}
            onToggleReviewed={() => toggleReviewed(file.path)}
            onToggleCollapsed={() => toggleExpanded(file.path)}
            onFocusHunk={handleFocusHunk}
            {isDark}
            {filter}
            {diffView}
            prId={pr ? String(pr.number) : ""}
            comments={comments.filter((c) => c.file_path === file.path)}
            onCommentAdded={handleCommentAdded}
            onCommentUpdated={handleCommentUpdated}
            onCommentDeleted={handleCommentDeleted}
            onOverrideCategory={handleOverrideCategory}
            onOverrideTags={handleOverrideTags}
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

  <!-- Inspector panel -->
  {#if inspectorOpen && focusedFile && focusedHunk}
    <InspectorPanel
      file={focusedFile}
      hunk={focusedHunk}
      classification={focusedClassification}
      {isDark}
      onClose={handleCloseInspector}
      onMarkReviewed={() => toggleReviewed(focusedFile!.path)}
    />
  {/if}
</div>

<!-- Submit review dialog (rendered outside the grid to avoid clipping) -->
{#if submitDialogOpen}
  <SubmitReviewDialog
    {draftCount}
    prNumber={pr?.number ?? 0}
    onClose={handleDialogClose}
    onSubmitted={handleSubmitted}
  />
{/if}
