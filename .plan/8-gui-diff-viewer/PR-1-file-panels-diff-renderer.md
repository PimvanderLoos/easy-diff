# PR-1: Main toolbar, file panels, hunk sections, and inline diff renderer

## Goal
Build the main diff viewing area: toolbar with file navigation and view toggle,
collapsible file panels with headers and status pills, hunk header strips with
tag pills and confidence, and the inline (unified) diff renderer with syntax
highlighting and line stripes. After this PR, the review screen shows actual
diff content.

## Non-goals
- No split diff view (PR-2).
- No filter dimming behavior (PR-2).
- No inspector panel (PR-3).

## Success Criteria
- [ ] MainToolbar.svelte: batch pill-select, file counter with prev/next, Unified/Split
      toggle (Split non-functional yet), overflow menu, keyboard shortcut button
- [ ] FilePanel.svelte: rounded card with collapse/expand
- [ ] FileHeader.svelte: chevron, file icon, name+dir, +/- stats, status pill, kebab
- [ ] FileTagRow.svelte: collapsed summary with hunk count + tag pills
- [ ] HunkSection.svelte: hunk header strip with line range, tag pills, confidence %
- [ ] InlineDiff.svelte: 5-column CSS Grid (stripe | old# | new# | sign | code)
- [ ] SyntaxHighlighter.ts: regex-based tokenizer for Java + YAML
- [ ] LineStripe.svelte: 3px vertical bar colored by attention tags
- [ ] StatusPill.svelte: Reviewing/Reviewed toggle
- [ ] SegToggle.svelte: segmented control for Unified/Split
- [ ] All components use design tokens and respond to dark/light toggle
- [ ] `cd frontend && npm run build` succeeds
- [ ] `cd frontend && npm run check` passes

## Technical Approach

### MainToolbar.svelte

Reference: `BMainToolbar` in `concept-b.jsx`

Flex layout with three groups:
- Left: PillSelect (batch, unreviewed)
- Center: file navigation (prev/next arrows + "1 of N" counter)
- Right: SegToggle (Unified/Split), overflow button, keyboard shortcuts button

### FilePanel.svelte

Reference: `BFilePanel` in `concept-b.jsx`

Rounded card container. Manages `collapsed` state (driven from parent or local).
When collapsed, shows `FileTagRow` instead of hunks.

### FileHeader.svelte

Reference: `BFileHeader` in `concept-b.jsx`

Click-to-collapse header. Entire header is clickable for toggle; StatusPill and
kebab menu stop propagation for independent actions.

Components: `CollapseChevron`, `FileIcon` (SVG), `StatusPill`.

### HunkSection.svelte

Reference: `BHunkSection` in `concept-b.jsx`

Hunk header strip shows `@@ L{start}-L{end}`, tag pills (`TagPill` from PR-0),
and confidence percentage. When focused (future inspector use), gets accent
border and soft background.

### InlineDiff.svelte

Reference: `EDInlineDiff` in `shared.jsx`

5-column CSS Grid: `3px 44px 44px 18px 1fr`
- Column 1: `LineStripe` (3px colored bar)
- Columns 2-3: old/new line numbers (right-aligned, text-faint)
- Column 4: sign (+/−/space)
- Column 5: syntax-highlighted code

Each line gets background color from type (addedBg/removedBg/transparent).

### SyntaxHighlighter.ts

Reference: `edHighlightJava` and `edHighlightYaml` in `shared.jsx`

Port the regex-based tokenizer. Returns `{ text: string, class: string }[]`.
Token classes: keyword, type, string, number, comment, anno, method, punct.

Map token classes to syntax colors from design tokens via CSS:
```css
.sx-keyword { color: var(--ed-sx-keyword); }
```

### LineStripe.svelte

Reference: `EDLineStripe` in `shared.jsx`

3px-wide column that splits vertically into colored segments for each tag
(max 3). Uses `edTagColor()` for color computation.

## Files to Create/Modify
- `frontend/src/lib/components/MainToolbar.svelte`
- `frontend/src/lib/components/FilePanel.svelte`
- `frontend/src/lib/components/FileHeader.svelte`
- `frontend/src/lib/components/FileTagRow.svelte`
- `frontend/src/lib/components/HunkSection.svelte`
- `frontend/src/lib/components/InlineDiff.svelte`
- `frontend/src/lib/components/LineStripe.svelte`
- `frontend/src/lib/components/StatusPill.svelte`
- `frontend/src/lib/components/SegToggle.svelte`
- `frontend/src/lib/components/PillSelect.svelte`
- `frontend/src/lib/components/IconBtn.svelte`
- `frontend/src/lib/syntax.ts` — syntax highlighter
- `frontend/src/lib/components/ReviewScreen.svelte` — wire in toolbar + file panels

## Dependencies
- Depends on: PR-0 (review layout, types, stores)
- Blocks: PR-2 (split diff, filter dimming)

## Design reference
- Screenshots: `02_Review_nofilter.png`
- Components: `BMainToolbar`, `BFilePanel`, `BFileHeader`, `BHunkSection` in `concept-b.jsx`
- Diff renderer: `EDInlineDiff`, `EDLineStripe`, `edHighlightJava` in `shared.jsx`
