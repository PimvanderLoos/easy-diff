<script lang="ts">
  /**
   * Root application component.
   *
   * Manages the top-level layout: TitleBar + screen routing.
   * The active screen is driven by the `currentScreen` store:
   *   - "selection" → PrSelectionScreen
   *   - "review"    → ReviewPlaceholder
   *
   * Theme (dark/light) is initialised from the OS preference and toggled
   * by adding/removing the `dark` class on `<html>`.
   */

  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import TitleBar from "./lib/components/TitleBar.svelte";
  import PrSelectionScreen from "./lib/components/PrSelectionScreen.svelte";
  import { currentScreen, selectedPr } from "./lib/stores.js";
  import type { RepoInfo } from "./lib/types.js";

  // ── Theme ────────────────────────────────────────────────────────────────

  let isDark = $state(
    typeof window !== "undefined"
      ? window.matchMedia("(prefers-color-scheme: dark)").matches
      : false,
  );

  $effect(() => {
    document.documentElement.classList.toggle("dark", isDark);
  });

  // ── Repo info (fetched once on mount) ───────────────────────────────────

  let repoInfo = $state<RepoInfo | null>(null);

  onMount(async () => {
    try {
      repoInfo = await invoke<RepoInfo>("get_repo_info");
    } catch {
      // Non-fatal: breadcrumb stays empty when not in a git repo.
    }
  });

  // ── Derived breadcrumb ───────────────────────────────────────────────────

  const repoPath = $derived(
    repoInfo ? `${repoInfo.owner}/${repoInfo.repo}` : "",
  );

  const breadcrumb = $derived(
    $currentScreen === "selection"
      ? "all pull requests"
      : $selectedPr
        ? `#${$selectedPr.number} ${$selectedPr.source_branch}`
        : "review",
  );
</script>

<div
  class="flex flex-col bg-ed-bg text-ed-text font-sans"
  style="height: 100vh; overflow: hidden;"
>
  <TitleBar {repoPath} {breadcrumb} username="" />

  {#if $currentScreen === "selection"}
    <PrSelectionScreen {repoInfo} />
  {:else if $currentScreen === "review"}
    <!-- Placeholder review screen — replaced in Epic 8 -->
    <div
      class="flex flex-1 items-center justify-center flex-col gap-4"
    >
      <div class="text-ed-text" style="font-size: 18px; font-weight: 500;">
        Review screen
      </div>
      {#if $selectedPr}
        <div
          class="text-ed-text-muted"
          style="font-family: var(--font-mono); font-size: 13px;"
        >
          #{$selectedPr.number} — {$selectedPr.title}
        </div>
      {/if}
      <button
        type="button"
        onclick={() => currentScreen.set("selection")}
        class="mt-4 px-4 py-2 rounded-md border border-ed-border bg-ed-elevated text-ed-text text-sm font-medium hover:border-ed-accent hover:text-ed-accent transition-colors cursor-pointer"
      >
        ← Back to PR list
      </button>
    </div>
  {/if}
</div>
