# PR-2: PR selection screen

## Goal
Implement the PR selection screen matching `01_PRselection.png`. This is the main
entry screen showing a filter sidebar and a list of pull requests fetched from the
backend via Tauri commands.

## Non-goals
- No review screen (Epic 8) — selecting a PR navigates to a stub/placeholder.
- No functional sort controls (recent/risk/size chips are visible but non-functional).
- No functional repository switching (display only).
- No search functionality.

## Success Criteria
- [ ] Two-column layout: filter sidebar (240px) + PR list, matching `01_PRselection.png`
- [ ] `TitleBar.svelte` with brand mark, repo/PR breadcrumb, help button, user avatar
      matching `BTitleBar` in `concept-b.jsx`
- [ ] `PrFilterSidebar.svelte` with Filter section (Assigned to me, Authored by me,
      All open, Drafts, Approved) and Repository section, matching left column of
      `BSelection` in `concept-b.jsx`
- [ ] `PrRow.svelte` displaying avatar, title, branch, status badges (draft/approved/
      changes), risky count with severity dot, file stats (+/-), recency — matching
      `BPrRow` in `concept-b.jsx`
- [ ] PR list fetched from backend via `invoke('list_pull_requests')`
- [ ] First PR row has active state (accent-left border + accent background)
- [ ] Selecting a PR navigates to a placeholder review screen
- [ ] Sort chips (recent, risk, size) visible as non-functional stubs
- [ ] Dark/light theme toggle works across the entire screen
- [ ] `cd frontend && npm run build` succeeds
- [ ] `cd frontend && npm run check` passes
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes

## Technical Approach

### Screen layout (`PrSelectionScreen.svelte`)

Reference: `BSelection` in `concept-b.jsx`

Two-column CSS Grid: `grid-template-columns: 240px 1fr`.

Left column: `PrFilterSidebar.svelte` with border-right.
Right column: Header ("Assigned to you", count, sort chips) + bordered PR list.

### TitleBar component (`TitleBar.svelte`)

Reference: `BTitleBar` in `concept-b.jsx`

Three-column grid (260px 1fr 200px):
- Left: Brand mark SVG + "easydiff" title
- Center: repo path breadcrumb (from `get_repo_info()`)
- Right: help button (?) + user avatar

### Filter sidebar (`PrFilterSidebar.svelte`)

Reference: left column of `BSelection` in `concept-b.jsx`

Uses `SidebarLabel` and `SidebarRow` components from PR-1.

Filter rows are currently static — the active state is hardcoded to
"Assigned to me". Future epics may add actual filtering.

Repository section shows the current repo from `get_repo_info()`.

### PR row (`PrRow.svelte`)

Reference: `BPrRow` in `concept-b.jsx`

5-column grid: `48px 1fr 120px 140px 80px`
- Avatar (from PR-1)
- Title + branch + status tags (draft/approved/changes)
- Risky count with SeverityDot
- File stats (files count, +adds/−dels)
- Recency ("2h ago")

### Navigation

Svelte stores for screen state:
```typescript
import { writable } from 'svelte/store';
export const currentScreen = writable<'selection' | 'review'>('selection');
export const selectedPr = writable<PullRequest | null>(null);
```

Clicking a PR row sets `selectedPr` and `currentScreen = 'review'`.
The review screen is a placeholder `<div>Review screen for PR #{pr.number}</div>`.

### Data fetching

On mount, call `invoke('list_pull_requests')` and store results in a reactive
variable. Show a loading state while fetching. Handle errors with a simple
error message display.

For the PR list data shape, the backend `PullRequest` struct needs to be
extended or the frontend needs to compute derived fields:
- `files`, `adds`, `dels` — not currently in `PullRequest`. Either add to the
  Rust struct (via API enrichment) or use placeholder values.
- `risky` count — not available until analysis runs. Show `—` when not analyzed.
- `updated` relative time — compute from `updated_at` timestamp.

Decision: For now, show what the API provides. Fields not available (`files`,
`adds`, `dels`, `risky`) display as `—` or are omitted. This matches the real
UX — the tool shows what it knows.

## Files to Create/Modify
- `frontend/src/lib/components/TitleBar.svelte`
- `frontend/src/lib/components/PrSelectionScreen.svelte`
- `frontend/src/lib/components/PrFilterSidebar.svelte`
- `frontend/src/lib/components/PrRow.svelte`
- `frontend/src/lib/stores.ts` — screen/PR state stores
- `frontend/src/App.svelte` — wire up screen routing

## Dependencies
- Depends on: PR-0 (scaffold), PR-1 (Tauri commands, shared components)
- Blocks: Epic 8 (review screen replaces the placeholder)

## Design reference
- Screenshot: `.plan/designs/01_PRselection.png`
- Component structure: `BSelection`, `BPrRow`, `BTitleBar` in `concept-b.jsx`
- Data shapes: `PR_LIST` in `diff-data.js`
