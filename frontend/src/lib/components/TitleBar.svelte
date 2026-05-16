<script lang="ts">
  /**
   * Application title bar — three-column grid header.
   *
   * Left: brand mark (diamond SVG) + "easydiff" wordmark.
   * Center: repo/PR breadcrumb in IBM Plex Mono.
   * Right: help button (?) + user avatar placeholder.
   *
   * Matches `BTitleBar` in `concept-b.jsx`.
   */

  import Avatar from "./Avatar.svelte";

  interface Props {
    /** Repository owner/name string, e.g. "atelier-io/api". */
    repoPath?: string;
    /** Current screen label shown in the breadcrumb center. */
    breadcrumb?: string;
    /** Username shown in the avatar (initials derived from it). */
    username?: string;
  }

  let {
    repoPath = "",
    breadcrumb = "all pull requests",
    username = "",
  }: Props = $props();

  const repoParts = $derived(repoPath ? repoPath.split("/") : []);
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
  <!-- Left: brand mark + wordmark -->
  <div class="flex items-center gap-2.5">
    <svg
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
    <span
      class="text-ed-text"
      style="font-size: 15px; font-weight: 700; letter-spacing: -0.3px;"
    >
      easydiff
    </span>
  </div>

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

  <!-- Right: help button + avatar -->
  <div class="flex items-center justify-end gap-2.5">
    <button
      class="flex items-center justify-center rounded-full text-ed-text-muted border border-ed-text-muted cursor-pointer bg-transparent"
      style="width: 20px; height: 20px; font-size: 11px; font-family: var(--font-sans);"
      aria-label="Help"
      type="button"
    >
      ?
    </button>
    {#if username}
      <Avatar name={username} size={26} />
    {:else}
      <!-- Placeholder circle when no user info available -->
      <span
        class="inline-flex shrink-0 items-center justify-center rounded-full bg-ed-panel border border-ed-border"
        style="width: 26px; height: 26px;"
      ></span>
    {/if}
  </div>
</header>
