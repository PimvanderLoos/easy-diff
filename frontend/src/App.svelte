<script lang="ts">
  /**
   * Root application component.
   *
   * Placeholder scaffold for Epic 7 PR-0.  Displays the easy-diff brand mark,
   * typography samples, and a dark/light theme toggle that adds/removes the
   * `dark` class on `<html>`.  All layout and colour uses design-token CSS
   * custom properties via Tailwind utility classes.
   */

  let isDark = $state(
    typeof window !== "undefined"
      ? window.matchMedia("(prefers-color-scheme: dark)").matches
      : false,
  );

  function toggleTheme(): void {
    isDark = !isDark;
    document.documentElement.classList.toggle("dark", isDark);
  }

  // Initialise <html> class on mount.
  $effect(() => {
    document.documentElement.classList.toggle("dark", isDark);
  });
</script>

<main
  class="min-h-screen bg-ed-bg text-ed-text font-sans flex flex-col items-center justify-center gap-8 p-8"
>
  <!-- Brand mark: diamond SVG matching the concept-b.jsx easydiff logo -->
  <div class="flex flex-col items-center gap-3">
    <svg
      width="48"
      height="48"
      viewBox="0 0 48 48"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      aria-hidden="true"
    >
      <path
        d="M24 4 L44 24 L24 44 L4 24 Z"
        fill="var(--ed-accent)"
        stroke="var(--ed-accent)"
        stroke-width="1"
        stroke-linejoin="round"
      />
      <path
        d="M24 14 L34 24 L24 34 L14 24 Z"
        fill="var(--ed-bg)"
        opacity="0.5"
      />
    </svg>

    <h1 class="text-2xl font-semibold tracking-tight">easy-diff</h1>
    <p class="text-ed-text-muted text-sm">LLM-powered PR review tool</p>
  </div>

  <!-- Typography and colour showcase -->
  <div
    class="w-full max-w-md rounded-lg border border-ed-border bg-ed-panel p-6 flex flex-col gap-4"
  >
    <h2 class="text-base font-semibold text-ed-text">Design tokens active</h2>

    <div class="flex flex-col gap-2 text-sm">
      <div class="flex justify-between">
        <span class="text-ed-text-muted">Background</span>
        <span
          class="font-mono text-ed-text-faint"
          style="font-family: var(--font-mono)"
          >var(--ed-bg)</span
        >
      </div>
      <div class="flex justify-between">
        <span class="text-ed-text-muted">Panel</span>
        <span
          class="font-mono text-ed-text-faint"
          style="font-family: var(--font-mono)"
          >var(--ed-panel)</span
        >
      </div>
      <div class="flex justify-between">
        <span class="text-ed-text-muted">Accent</span>
        <span class="text-ed-accent" style="font-family: var(--font-mono)"
          >var(--ed-accent)</span
        >
      </div>
    </div>

    <hr class="border-ed-border-subtle" />

    <!-- Diff colour swatches -->
    <div class="flex gap-2 text-xs">
      <span
        class="px-2 py-1 rounded text-ed-added bg-ed-added-bg border border-ed-added-gutter"
        style="font-family: var(--font-mono)">+ added</span
      >
      <span
        class="px-2 py-1 rounded text-ed-removed bg-ed-removed-bg border border-ed-removed-gutter"
        style="font-family: var(--font-mono)">- removed</span
      >
      <span
        class="px-2 py-1 rounded text-ed-risky bg-ed-risky-bg"
        style="font-family: var(--font-mono)">⚠ risky</span
      >
    </div>

    <!-- Font specimens -->
    <div class="flex flex-col gap-1 text-sm text-ed-text-muted">
      <span>IBM Plex Sans — regular body text</span>
      <span
        class="font-semibold text-ed-text"
        style="font-family: var(--font-sans)">IBM Plex Sans 600 — headings</span
      >
      <code class="text-ed-accent" style="font-family: var(--font-mono)"
        >IBM Plex Mono — code and metadata</code
      >
    </div>
  </div>

  <!-- Theme toggle -->
  <button
    onclick={toggleTheme}
    class="px-4 py-2 rounded-md border border-ed-border bg-ed-elevated text-ed-text text-sm font-medium hover:border-ed-accent hover:text-ed-accent transition-colors cursor-pointer"
    aria-label="Toggle dark/light theme"
  >
    {isDark ? "Switch to light mode" : "Switch to dark mode"}
  </button>

  <p class="text-ed-text-faint text-xs">
    Epic 7 PR-0 scaffold — Tauri v2 + Svelte + TypeScript + Tailwind CSS v4
  </p>
</main>
