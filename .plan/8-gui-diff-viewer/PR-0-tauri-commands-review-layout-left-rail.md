# PR-0: Tauri commands for review data + review screen layout + left rail

## Goal
Expose Tauri commands for triggering analysis and fetching results. Build the
review screen's three-column layout shell and the left rail with PR metadata,
progress bar, and tabbed Files/Filters views. After this PR, selecting a PR
from the selection screen navigates to the review layout with a populated
left rail, but the main diff area and inspector are stubs.

## Non-goals
- No diff rendering (PR-1).
- No inspector panel (PR-3).
- No filter behavior / dimming (PR-2).
- No syntax highlighting (PR-1).

## Success Criteria
- [ ] Tauri commands: `run_analysis`, `get_analysis`, `get_diff`, `get_pass1_summary`
- [ ] ReviewScreen.svelte: three-column grid (284px | 1fr | optional 340px)
- [ ] LeftRail.svelte: PR metadata, progress bar, Files/Filters tab switcher
- [ ] FilesTab.svelte: Show section (All/Reviewed/Unreviewed counts), Categories
      section (attention tags with colored swatches + change types with outline dots
      and counts), Files list with per-file +/- stats and tag pills
- [ ] FiltersTab.svelte: Change type radio group, Attention tag checkbox group,
      clear button with active filter count
- [ ] Filter state stored in Svelte store: `{ changeType: string, attentionTags: string[] }`
- [ ] Frontend TypeScript types for `AnalysisResult`, `DiffFile`, `Pass1Output`,
      `Pass2Output`, hunk/classification data shapes from `diff-data.js`
- [ ] `cd frontend && npm run build` succeeds
- [ ] `cd frontend && npm run check` passes
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes

## Technical Approach

### Tauri commands (`src/gui/mod.rs`)

```rust
#[tauri::command]
async fn run_analysis(pr_number: u64) -> Result<AnalysisResult, String> { ... }

#[tauri::command]
async fn get_analysis(pr_number: u64) -> Result<Option<AnalysisResult>, String> { ... }

#[tauri::command]
async fn get_diff(pr_number: u64) -> Result<Vec<DiffFile>, String> { ... }

#[tauri::command]
async fn get_pass1_summary(pr_number: u64) -> Result<Option<Pass1Output>, String> { ... }
```

Ensure `AnalysisResult`, `DiffFile`, `Pass1Output`, `Pass2Output` derive
`Serialize` for Tauri IPC.

### Frontend types (`frontend/src/lib/types.ts`)

Add types matching `diff-data.js` shapes:
- `HunkData` with `id`, `header`, `oldStart`, `newStart`, `lines`
- `DiffLine` with `type`, `old`, `new`, `text`, `tags`
- `Classification` with `changeType`, `attentionTags`, `confidence`, `summary`, `rationale`
- `FilterState` with `changeType`, `attentionTags`
- Constants: `CHANGE_TYPES`, `ATTENTION_TAGS` arrays

### ReviewScreen.svelte

Reference: `BReview` in `concept-b.jsx`

Three-column CSS Grid. Inspector column initially hidden (toggled in PR-3).
Main area shows a placeholder "Diff area" until PR-1.

### LeftRail.svelte

Reference: `BLeftRail` in `concept-b.jsx`

- PR meta section (number, author, title)
- Progress bar (reviewed/total)
- Tab switcher (Files | Filters) with accent underline on active tab

### FilesTab.svelte

Reference: `BLeftFilesTab` in `concept-b.jsx`

- Show section: All files / Reviewed / Unreviewed counts
- Categories section: attention tags (colored swatch + label + count),
  change types (outline dot + label + count)
- Files list: clickable rows with file name, +/- stats, tag pills

### FiltersTab.svelte

Reference: `BLeftFiltersTab` in `concept-b.jsx`

- Change type radio group with radio-dot styling
- Attention tag checkbox group with colored checkbox + swatch
- Clear button showing active filter count

### Stores (`frontend/src/lib/stores.ts`)

Add:
```typescript
export const filterState = writable<FilterState>({ changeType: 'all', attentionTags: [] });
export const reviewedFiles = writable<Set<string>>(new Set());
```

## Files to Create/Modify
- `src/gui/mod.rs` — add 4 Tauri commands
- `src/analysis/mod.rs` — ensure `AnalysisResult` derives `Serialize`
- `src/diff/mod.rs` — ensure `DiffFile` derives `Serialize`
- `frontend/src/lib/types.ts` — add review/analysis types + constants
- `frontend/src/lib/stores.ts` — add filter + review state stores
- `frontend/src/lib/components/ReviewScreen.svelte`
- `frontend/src/lib/components/LeftRail.svelte`
- `frontend/src/lib/components/FilesTab.svelte`
- `frontend/src/lib/components/FiltersTab.svelte`
- `frontend/src/lib/components/TagPill.svelte` — uppercase dotted pill
- `frontend/src/App.svelte` — wire review screen

## Dependencies
- Depends on: Epic 7 (Tauri scaffold, shared components)
- Blocks: PR-1 (diff renderer needs the layout)

## Design reference
- Screenshots: `02_Review_nofilter.png`, `03_Inspector_filteractive.png`
- Components: `BReview`, `BLeftRail`, `BLeftFilesTab`, `BLeftFiltersTab` in `concept-b.jsx`
- Data: `HUNKS`, `CLASSIFICATIONS` in `diff-data.js`
