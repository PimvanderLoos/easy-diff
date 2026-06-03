<script lang="ts">
  /**
   * Application title bar — three-column grid header.
   *
   * Left: brand mark (diamond SVG) that doubles as the analysis trigger on the
   *   review screen — "Analyze PR" / "Analyzing…" (spinning) / "Re-analyze".
   * Center: repo/PR breadcrumb in IBM Plex Mono.
   * Right: theme toggle + help button (?) + user avatar.
   *
   * Matches `BTitleBar` in `concept-b.jsx`.
   */

  import Avatar from "./Avatar.svelte";
  import { currentScreen, selectedPr, analysisStatus } from "../stores.js";
  import { runAnalysis } from "../analysisController.js";

  interface Props {
    /** Repository owner/name string, e.g. "atelier-io/api". */
    repoPath?: string;
    /** Current screen label shown in the breadcrumb center. */
    breadcrumb?: string;
    /** Username shown in the avatar (initials derived from it). */
    username?: string;
    /** Optional avatar image URL (e.g. the reviewer's GitHub profile picture). */
    avatarUrl?: string;
    /** Active UI theme, used to pick the toggle icon. */
    theme?: "light" | "dark";
    /** Called when the theme toggle button is clicked. */
    onToggleTheme?: () => void;
    /** Called when the help (?) button is clicked. */
    onHelp?: () => void;
  }

  let {
    repoPath = "",
    breadcrumb = "all pull requests",
    username = "",
    avatarUrl,
    theme = "light",
    onToggleTheme,
    onHelp,
  }: Props = $props();

  const repoParts = $derived(repoPath ? repoPath.split("/") : []);

  // ── Analysis trigger (review screen only) ──────────────────────────────────

  const pr = $derived($selectedPr);

  /** The brand acts as the analyze button only while reviewing a PR. */
  const isReview = $derived($currentScreen === "review" && pr !== null);

  /** Whether analysis is currently running (drives the spinning logo). */
  const analyzing = $derived($analysisStatus === "running");

  /** Button label reflecting the analysis lifecycle. */
  const analyzeLabel = $derived(
    $analysisStatus === "running"
      ? "Analyzing…"
      : $analysisStatus === "done" || $analysisStatus === "error"
        ? "Re-analyze"
        : "Analyze PR",
  );

  function handleAnalyzeClick() {
    if (pr && !analyzing) runAnalysis(pr.number);
  }
</script>

<header
  class="flex shrink-0 items-center border-b border-ed-border-subtle bg-ed-bg"
  style="
    height: 48px;
    display: grid;
    grid-template-columns: 260px 1fr 200px;
    padding: 0 18px;
  "
>
  <!-- Left: brand mark (doubles as the analyze button on the review screen) -->
  {#snippet diamond(spin: boolean)}
    <svg
      class={spin ? "ed-logo-spin" : ""}
      width="22"
      height="22"
      viewBox="0 0 22 22"
      fill="none"
      aria-hidden="true"
    >
      <path
        d="M11 2 L19 11 L11 20 L3 11 Z"
        stroke="var(--ed-text)"
        stroke-width="1.6"
        stroke-linejoin="round"
        fill="none"
      />
      <circle cx="11" cy="11" r="2.4" fill="var(--ed-text)" />
    </svg>
  {/snippet}

  {#if isReview}
    <button
      type="button"
      onclick={handleAnalyzeClick}
      disabled={analyzing}
      aria-label={analyzing ? "Analyzing pull request" : analyzeLabel}
      class="flex items-center gap-2.5 border-none bg-transparent p-0 text-ed-text"
      style="cursor: {analyzing ? 'default' : 'pointer'};"
    >
      {@render diamond(analyzing)}
      <span style="font-size: 15px; font-weight: 700; letter-spacing: -0.3px;">
        {analyzeLabel}
      </span>
    </button>
  {:else}
    <div class="flex items-center gap-2.5">
      {@render diamond(false)}
      <span
        class="text-ed-text"
        style="font-size: 15px; font-weight: 700; letter-spacing: -0.3px;"
      >
        easydiff
      </span>
    </div>
  {/if}

  <!-- Center: breadcrumb -->
  <div
    class="text-center text-ed-text-muted"
    style="font-family: var(--font-mono); font-size: 12px;"
  >
    {#if repoParts.length >= 2}
      <span>{repoParts[0]}</span>
      <span class="text-ed-text-faint mx-1.5">/</span>
      <span>{repoParts.slice(1).join("/")}</span>
      <span class="text-ed-text-faint" style="margin: 0 7px;">·</span>
    {/if}
    <span>{breadcrumb}</span>
  </div>

  <!-- Right: theme toggle + help button + avatar -->
  <div class="flex items-center justify-end gap-2.5">
    <button
      class="flex items-center justify-center rounded-full text-ed-text-muted cursor-pointer bg-transparent border-none"
      style="width: 22px; height: 22px;"
      aria-label={theme === "dark" ? "Switch to light mode" : "Switch to dark mode"}
      title={theme === "dark" ? "Switch to light mode" : "Switch to dark mode"}
      type="button"
      onclick={onToggleTheme}
    >
      {#if theme === "dark"}
        <!-- Sun: click to switch to light -->
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none" aria-hidden="true">
          <circle cx="8" cy="8" r="3.2" stroke="currentColor" stroke-width="1.4" />
          <path
            d="M8 1.2V2.6 M8 13.4V14.8 M1.2 8H2.6 M13.4 8H14.8 M3.2 3.2L4.2 4.2 M11.8 11.8L12.8 12.8 M12.8 3.2L11.8 4.2 M4.2 11.8L3.2 12.8"
            stroke="currentColor"
            stroke-width="1.4"
            stroke-linecap="round"
          />
        </svg>
      {:else}
        <!-- Moon: click to switch to dark -->
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none" aria-hidden="true">
          <path
            d="M13.5 9.3A5.6 5.6 0 0 1 6.7 2.5 5.6 5.6 0 1 0 13.5 9.3Z"
            stroke="currentColor"
            stroke-width="1.4"
            stroke-linejoin="round"
          />
        </svg>
      {/if}
    </button>
    <button
      class="flex items-center justify-center rounded-full text-ed-text-muted border border-ed-text-muted cursor-pointer bg-transparent"
      style="width: 20px; height: 20px; font-size: 11px; font-family: var(--font-sans);"
      aria-label="Help"
      title="Keyboard shortcuts"
      type="button"
      onclick={onHelp}
    >
      ?
    </button>
    {#if username}
      <Avatar name={username} size={26} imageUrl={avatarUrl} />
    {:else}
      <!-- Placeholder circle when no user info available -->
      <span
        class="inline-flex shrink-0 items-center justify-center rounded-full bg-ed-panel border border-ed-border"
        style="width: 26px; height: 26px;"
      ></span>
    {/if}
  </div>
</header>

<style>
  /* Gentle, continuous rotation of the brand mark while analysis runs. */
  @keyframes ed-logo-spin {
    to {
      transform: rotate(360deg);
    }
  }

  .ed-logo-spin {
    transform-origin: 50% 50%;
    animation: ed-logo-spin 2.5s linear infinite;
  }

  /* Respect users who prefer reduced motion. */
  @media (prefers-reduced-motion: reduce) {
    .ed-logo-spin {
      animation: none;
    }
  }
</style>
