<script lang="ts">
  /**
   * PR selection filter sidebar — left column of the PR selection screen.
   *
   * Shows a static "Filter" section (Assigned to me, Authored by me, All open,
   * Drafts, Approved) and a "Repository" section with the current repo name.
   * The active filter row is hardcoded to "Assigned to me" for this epic;
   * functional filtering is deferred to a later epic.
   *
   * Matches the left column of `BSelection` in `concept-b.jsx`.
   */

  import SidebarLabel from "./SidebarLabel.svelte";
  import SidebarRow from "./SidebarRow.svelte";

  interface FilterRow {
    label: string;
    count: number;
    active?: boolean;
  }

  interface Props {
    /** Repository path string e.g. "owner/repo". Shown in Repository section. */
    repoPath?: string;
    /** Total PR counts per filter row. */
    counts?: { assignedToMe: number; authoredByMe: number; allOpen: number; drafts: number; approved: number };
  }

  let { repoPath = "", counts }: Props = $props();

  const filterRows: FilterRow[] = $derived([
    { label: "Assigned to me", count: counts?.assignedToMe ?? 0, active: true },
    { label: "Authored by me", count: counts?.authoredByMe ?? 0 },
    { label: "All open", count: counts?.allOpen ?? 0 },
    { label: "Drafts", count: counts?.drafts ?? 0 },
    { label: "Approved", count: counts?.approved ?? 0 },
  ]);
</script>

<aside
  class="border-r border-ed-border-subtle bg-ed-bg"
  style="padding: 24px 18px;"
>
  <SidebarLabel>Filter</SidebarLabel>

  {#each filterRows as row (row.label)}
    <SidebarRow active={row.active ?? false}>
      <span>{row.label}</span>
      <span
        class="text-ed-text-faint"
        style="font-family: var(--font-mono); font-size: 11px;"
      >
        {row.count}
      </span>
    </SidebarRow>
  {/each}

  <div style="margin-top: 28px;">
    <SidebarLabel>Repository</SidebarLabel>
    {#if repoPath}
      <div
        class="text-ed-text"
        style="font-family: var(--font-mono); font-size: 12px; padding: 4px 0;"
      >
        {repoPath}
      </div>
    {:else}
      <div
        class="text-ed-text-faint"
        style="font-family: var(--font-mono); font-size: 12px; padding: 4px 0;"
      >
        —
      </div>
    {/if}
  </div>
</aside>
