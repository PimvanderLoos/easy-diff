<script lang="ts">
  /**
   * Root application component.
   *
   * Manages the top-level layout: TitleBar + screen routing.
   * The active screen is driven by the `currentScreen` store:
   *   - "selection" → PrSelectionScreen
   *   - "review"    → ReviewPlaceholder
   *
   * Theme (dark/light) is driven by the `theme` store (persisted choice or OS
   * preference) and applied by adding/removing the `dark` class on `<html>`.
   */

  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import TitleBar from "./lib/components/TitleBar.svelte";
  import PrSelectionScreen from "./lib/components/PrSelectionScreen.svelte";
  import ReviewScreen from "./lib/components/ReviewScreen.svelte";
  import { currentScreen, selectedPr, theme } from "./lib/stores.js";
  import type { RepoInfo } from "./lib/types.js";

  // ── Theme ────────────────────────────────────────────────────────────────

  $effect(() => {
    document.documentElement.classList.toggle("dark", $theme === "dark");
  });

  function toggleTheme() {
    theme.update((t) => (t === "dark" ? "light" : "dark"));
  }

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
  <TitleBar
    {repoPath}
    {breadcrumb}
    username=""
    theme={$theme}
    onToggleTheme={toggleTheme}
  />

  {#if $currentScreen === "selection"}
    <PrSelectionScreen {repoInfo} />
  {:else if $currentScreen === "review"}
    <ReviewScreen />
  {/if}
</div>
