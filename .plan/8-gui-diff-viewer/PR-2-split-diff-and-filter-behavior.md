# PR-2: Split diff view and filter dimming behavior

## Goal
Add the side-by-side (split) diff view and implement the filter behavior that
dims non-matching lines when filters are active. This is the core UX: when
the user selects a change type or attention tags in the Filters tab, non-matching
add/del lines are rendered at reduced opacity while context lines stay full.

## Non-goals
- No inspector panel (PR-3).
- No "Suggest filters" button functionality.
- No keyboard navigation.

## Success Criteria
- [ ] SplitDiff.svelte: two-column grid, each side has (stripe | line# | sign | code)
- [ ] Adjacent del/add lines are paired side-by-side
- [ ] SegToggle switches between inline and split views
- [ ] DiffView.svelte: delegates to InlineDiff or SplitDiff based on view mode
- [ ] Filter dimming: non-passing add/del lines at opacity 0.32, saturate(0.65)
- [ ] Context lines always show at full opacity regardless of filter
- [ ] A line passes if it matches changeType (or "all") AND has at least one
      selected attention tag (or no tags selected)
- [ ] Filter counts in FilesTab update live as filters toggle
- [ ] Clear button in FiltersTab resets filter state
- [ ] `cd frontend && npm run build` succeeds
- [ ] `cd frontend && npm run check` passes

## Technical Approach

### SplitDiff.svelte

Reference: `EDSplitDiff` in `shared.jsx`

Two-column grid. Each side has its own 4-column grid: `3px 36px 14px 1fr`.

Line pairing logic:
1. Walk lines sequentially
2. If current is `del` and next is `add`: pair them (left=del, right=add)
3. If `del` only: left=del, right=empty
4. If `add` only: left=empty, right=add
5. If `ctx`: both sides show the same line

### DiffView.svelte

Simple wrapper that delegates:
```svelte
{#if view === 'split'}
  <SplitDiff {lines} {theme} {filePath} {classification} {filter} />
{:else}
  <InlineDiff {lines} {theme} {filePath} {classification} {filter} />
{/if}
```

### Filter logic (`frontend/src/lib/filters.ts`)

Port `edLinePasses` and `edLineTags` from `shared.jsx`:

```typescript
export function linePasses(line: DiffLine, classification: Classification | null, filter: FilterState): boolean {
  if (line.type === 'ctx') return true;
  if (filter.changeType && filter.changeType !== 'all') {
    if ((classification?.changeType || 'uncategorized') !== filter.changeType) return false;
  }
  if (filter.attentionTags.length > 0) {
    const tags = lineTags(line, classification);
    if (!tags.some(t => filter.attentionTags.includes(t))) return false;
  }
  return true;
}
```

### Wiring

- `InlineDiff` and `SplitDiff` both receive `filter` prop and apply dimming
- `ReviewScreen` passes `filterState` store value to diff components
- `FilesTab` computes category counts from analysis data
- `FiltersTab` dispatches changes to `filterState` store

### SegToggle integration

Wire `SegToggle` in `MainToolbar` to a `diffViewMode` writable store.
`HunkSection` passes the current mode to its diff renderer.

## Files to Create/Modify
- `frontend/src/lib/components/SplitDiff.svelte`
- `frontend/src/lib/components/DiffView.svelte`
- `frontend/src/lib/filters.ts` — filter logic functions
- `frontend/src/lib/stores.ts` — add `diffViewMode` store
- `frontend/src/lib/components/InlineDiff.svelte` — add filter dimming
- `frontend/src/lib/components/HunkSection.svelte` — pass filter + view mode
- `frontend/src/lib/components/MainToolbar.svelte` — wire SegToggle
- `frontend/src/lib/components/FilesTab.svelte` — live filter counts
- `frontend/src/lib/components/FiltersTab.svelte` — wire clear button

## Dependencies
- Depends on: PR-1 (inline diff, hunk sections, toolbar)
- Blocks: PR-3 (inspector references filter state)

## Design reference
- Screenshot: `03_Inspector_filteractive.png` (shows dimmed lines)
- Components: `EDSplitDiff`, `edLinePasses`, `edLineTags` in `shared.jsx`
