<script lang="ts">
  /**
   * PR selection screen — two-column layout with filter sidebar and PR list.
   *
   * Left column (240px): `PrFilterSidebar` with Filter and Repository sections.
   * Right column: header with title, count, sort chips, and a bordered PR list.
   *
   * Fetches PRs via `invoke('list_pull_requests')` on mount. Shows a loading
   * indicator while fetching and an error message on failure.
   *
   * Matches `BSelection` in `concept-b.jsx`.
   */

  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import PrFilterSidebar from "./PrFilterSidebar.svelte";
  import PrRow from "./PrRow.svelte";
  import type { PullRequest, RepoInfo } from "../types.js";
  import { currentScreen, selectedPr } from "../stores.js";

  interface Props {
    /** Repository info from the backend, if already fetched. */
    repoInfo?: RepoInfo | null;
  }

  let { repoInfo = null }: Props = $props();

  let prs = $state<PullRequest[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);

  const repoPath = $derived(
    repoInfo ? `${repoInfo.owner}/${repoInfo.repo}` : "",
  );

  /** Count of non-draft PRs assigned context (all open for now). */
  const counts = $derived({
    assignedToMe: prs.length,
    authoredByMe: 0,
    allOpen: prs.length,
    drafts: 0,
    approved: 0,
  });

  onMount(async () => {
    try {
      prs = await invoke<PullRequest[]>("list_pull_requests");
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  });

  function selectPr(pr: PullRequest): void {
    selectedPr.set(pr);
    currentScreen.set("review");
  }
</script>

<div
  style="
    display: grid;
    grid-template-columns: 240px 1fr;
    flex: 1;
    overflow: hidden;
  "
>
  <!-- Left: filter sidebar -->
  <PrFilterSidebar {repoPath} {counts} />

  <!-- Right: header + PR list -->
  <div
    class="bg-ed-bg"
    style="overflow: auto; padding: 30px 40px;"
  >
    <!-- List header: title + count + sort chips -->
    <div
      class="flex items-baseline justify-between"
      style="margin-bottom: 24px;"
    >
      <div>
        <div
          class="text-ed-text"
          style="font-size: 22px; font-weight: 500; letter-spacing: -0.3px;"
        >
          Assigned to you
        </div>
        <div
          class="text-ed-text-muted"
          style="margin-top: 4px; font-family: var(--font-mono); font-size: 12px;"
        >
          {#if loading}
            Loading…
          {:else if error}
            Failed to load pull requests
          {:else}
            {prs.length} open · sorted by recency
          {/if}
        </div>
      </div>

      <!-- Sort chips: non-functional stubs per spec -->
      <div class="flex gap-1.5">
        {#each ["recent", "risk", "size"] as chip (chip)}
          <span
            class="text-ed-text-muted border border-ed-border rounded"
            style="
              font-family: var(--font-mono);
              font-size: 11px;
              padding: 3px 10px;
              background: var(--ed-elevated);
              cursor: default;
              letter-spacing: 0.2px;
            "
          >
            {chip}
          </span>
        {/each}
      </div>
    </div>

    <!-- PR list or loading/error state -->
    {#if loading}
      <div
        class="text-ed-text-muted text-center"
        style="padding: 48px 0; font-size: 14px;"
      >
        Loading pull requests…
      </div>
    {:else if error}
      <div
        class="rounded-lg border border-ed-border bg-ed-panel"
        style="padding: 24px; text-align: center;"
      >
        <div class="text-ed-text" style="font-size: 14px; font-weight: 500;">
          Could not load pull requests
        </div>
        <div
          class="text-ed-text-muted"
          style="margin-top: 8px; font-family: var(--font-mono); font-size: 12px;"
        >
          {error}
        </div>
      </div>
    {:else if prs.length === 0}
      <div
        class="text-ed-text-muted text-center"
        style="padding: 48px 0; font-size: 14px;"
      >
        No open pull requests found.
      </div>
    {:else}
      <div
        class="border border-ed-border-subtle rounded-lg overflow-hidden bg-ed-bg"
      >
        {#each prs as pr, i (pr.number)}
          <PrRow
            {pr}
            active={i === 0}
            last={i === prs.length - 1}
            onclick={() => selectPr(pr)}
          />
        {/each}
      </div>
    {/if}
  </div>
</div>
