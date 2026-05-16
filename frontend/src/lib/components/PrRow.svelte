<script lang="ts">
  /**
   * Single PR list row with avatar, title, branch, risky count, file stats,
   * and recency timestamp.
   *
   * Fields not yet returned by the `list_pull_requests` backend command
   * (`files`, `adds`, `dels`, risky count, draft/approval status) display
   * as "—". They will be filled in once the backend exposes enriched PR data.
   *
   * Matches `BPrRow` in `concept-b.jsx`.
   *
   * Grid: 48px | 1fr | 120px | 140px | 80px
   */

  import Avatar from "./Avatar.svelte";
  import type { PullRequest } from "../types.js";

  interface Props {
    /** The pull request data from the backend. */
    pr: PullRequest;
    /** Whether this row is the currently active/selected PR. */
    active?: boolean;
    /** Whether this is the last row (suppresses bottom border). */
    last?: boolean;
    /** Click handler for selecting the PR. */
    onclick?: () => void;
  }

  let { pr, active = false, last = false, onclick }: Props = $props();

  /**
   * Formats an ISO 8601 timestamp into a compact relative string like
   * "2h ago", "3d ago". Returns "—" when the input is empty or invalid.
   */
  function relativeTime(iso: string): string {
    if (!iso) return "—";
    const ts = new Date(iso).getTime();
    if (Number.isNaN(ts)) return "—";
    const delta = Math.max(0, Date.now() - ts);
    const minutes = Math.floor(delta / 60_000);
    if (minutes < 60) return `${Math.max(1, minutes)}m`;
    const hours = Math.floor(delta / 3_600_000);
    if (hours < 24) return `${hours}h`;
    const days = Math.floor(delta / 86_400_000);
    if (days < 7) return `${days}d`;
    return `${Math.floor(days / 7)}w`;
  }

  const recency = $derived(relativeTime(pr.updated_at));
</script>

<!-- svelte-ignore a11y_interactive_supports_focus -->
<div
  role="button"
  tabindex="0"
  {onclick}
  onkeydown={(e) => {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      onclick?.();
    }
  }}
  class="cursor-pointer"
  style="
    display: grid;
    grid-template-columns: 48px 1fr 120px 140px 80px;
    gap: 16px;
    align-items: center;
    padding: 16px 20px;
    border-bottom: {last ? 'none' : '1px solid var(--ed-border-subtle)'};
    background: {active ? 'var(--ed-accent-soft)' : 'transparent'};
    border-left: {active ? '2px solid var(--ed-accent)' : '2px solid transparent'};
    transition: background 0.1s;
  "
>
  <!-- Avatar -->
  <div class="flex items-center justify-center">
    <Avatar name={pr.author} size={28} />
  </div>

  <!-- Title + branch metadata -->
  <div style="min-width: 0;">
    <div
      class="flex items-center gap-2.5 text-ed-text"
      style="font-size: 14px; font-weight: 500; min-width: 0;"
    >
      <span style="overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">
        {pr.title}
      </span>
    </div>
    <div
      class="text-ed-text-muted"
      style="
        margin-top: 4px;
        font-family: var(--font-mono);
        font-size: 11.5px;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
      "
    >
      #{pr.number} · {pr.source_branch} · @{pr.author}
    </div>
  </div>

  <!-- Risky count — not yet available from backend; shown after analysis runs -->
  <div style="font-family: var(--font-mono); font-size: 12px;">
    <span class="text-ed-text-faint">—</span>
  </div>

  <!-- File stats — not yet returned by list_pull_requests -->
  <div style="font-family: var(--font-mono); font-size: 12px;">
    <span class="text-ed-text-faint">—</span>
  </div>

  <!-- Recency -->
  <div
    class="text-ed-text-faint text-right"
    style="font-family: var(--font-mono); font-size: 11.5px;"
  >
    {recency} ago
  </div>
</div>
