# PR-3: Inspector panel and review tracking

## Goal
Build the collapsible inspector panel (right column) and implement file review
tracking with auto-collapse behavior. Clicking a hunk opens the inspector with
its classification details; marking a file as reviewed collapses it and updates
the progress bar.

## Non-goals
- No comment submission (Epic 9).
- No right-click context menu (Epic 9).
- No "Create follow-up" functionality.
- No keyboard navigation.

## Success Criteria
- [ ] InspectorPanel.svelte: sticky header, Selected change section (summary + lines),
      Why this matters (rationale), Category (tag pill), Confidence (% + bar),
      Status (pill select), Assignee (avatar + name), Notes (text input placeholder),
      Activity (avatar + timestamped entries), Actions (Override/Reviewed/Follow-up)
- [ ] Clicking a hunk opens the inspector with that hunk's classification
- [ ] Inspector close button hides the right column (grid changes to 2-column)
- [ ] StatusPill toggles between Reviewing (outline) and Reviewed (filled green)
- [ ] Marking a file as reviewed auto-collapses it
- [ ] Unmarking re-expands the file
- [ ] Progress bar in left rail updates live (N/M files reviewed)
- [ ] Show/Reviewed/Unreviewed counts in FilesTab update from reviewedFiles store
- [ ] `cd frontend && npm run build` succeeds
- [ ] `cd frontend && npm run check` passes
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes

## Technical Approach

### InspectorPanel.svelte

Reference: `BInspectorPanel` in `concept-b.jsx`

Right column (340px) in the three-column grid. Sticky header with "Inspector"
title and close button.

Sections (each in a bordered container):
1. **Selected change** — summary text + line range + added count badge
2. **Why this matters** — rationale text
3. **Category** — TagPill for the first attention tag
4. **Confidence** — percentage + progress bar
5. **Status** — PillSelect showing "Reviewing"
6. **Assignee** — Avatar + name dropdown (non-functional)
7. **Notes** — placeholder text input
8. **Activity** — avatar + timestamped entries (hardcoded for now)
9. **Actions** — Override category (dropdown), Mark as reviewed (green border
   button), Create follow-up (muted button)

### Hunk focus

Add `focusedHunkId` to stores. Clicking a hunk header sets it. The
`HunkSection` with matching ID gets accent border + soft background.
`InspectorPanel` reads the focused hunk's classification from the analysis data.

### Review tracking

`reviewedFiles` writable store (`Set<string>`).

`toggleReviewed(path)` logic (port from `concept-b.jsx`):
- If marking as reviewed: add to set, also add to `collapsedFiles` set
- If unmarking: remove from set, remove from `collapsedFiles` set

`collapsedFiles` writable store (`Set<string>`).

`FilePanel` reads both stores to determine collapsed + reviewed state.

### Progress bar

`LeftRail` computes `reviewedCount / totalFiles` from the `reviewedFiles` store.

### FilesTab counts

Show section rows read from `reviewedFiles` store:
- All files: total
- Reviewed: reviewedFiles.size
- Unreviewed: total - reviewedFiles.size

## Files to Create/Modify
- `frontend/src/lib/components/InspectorPanel.svelte`
- `frontend/src/lib/stores.ts` — add `focusedHunkId`, `collapsedFiles`
- `frontend/src/lib/components/ReviewScreen.svelte` — wire inspector open/close
- `frontend/src/lib/components/HunkSection.svelte` — add click-to-focus
- `frontend/src/lib/components/FilePanel.svelte` — wire reviewed + collapsed
- `frontend/src/lib/components/FileHeader.svelte` — wire StatusPill toggle
- `frontend/src/lib/components/LeftRail.svelte` — progress bar from store
- `frontend/src/lib/components/FilesTab.svelte` — Show section from store

## Dependencies
- Depends on: PR-2 (filter state, split diff, DiffView)
- Blocks: nothing (epic complete after this PR)

## Design reference
- Screenshot: `03_Inspector_filteractive.png`
- Components: `BInspectorPanel`, `toggleReviewed` in `concept-b.jsx`
