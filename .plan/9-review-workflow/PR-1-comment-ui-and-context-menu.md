# PR-1: Comment UI and right-click context menu

## Goal
Add inline comment creation UI: clicking a line or selecting a range opens a
comment input. Add a right-click context menu on diff lines with options to
add a comment or change category (manual override). Wire both to Tauri commands
that use the local storage from PR-0.

## Non-goals
- No submission to GitHub/BitBucket (PR-2).
- No reply threading.
- No reactions/emoji.
- No comment editing in the inspector (keep it simple — edit in the inline area).

## Success Criteria
- [ ] Clicking a line number shows an inline comment textarea below that line/range
- [ ] Comment input has Save (stores draft) and Cancel buttons
- [ ] Saved draft comments show as yellow-highlighted inline blocks with the comment
      body, edit and delete buttons
- [ ] Right-click on a diff line opens a context menu with:
      - "Add comment" (opens inline comment input)
      - "Override category →" (submenu with change types)
      - "Override tags →" (submenu with attention tags, checkboxes)
- [ ] Category overrides are reflected immediately in the UI (tag pills update)
- [ ] Tauri commands: `add_comment`, `list_comments`, `update_comment`,
      `delete_comment`, `set_category_override`, `get_category_overrides`
- [ ] Frontend types for comment and override data
- [ ] `cd frontend && npm run build` succeeds
- [ ] `cd frontend && npm run check` passes
- [ ] `cargo clippy -- -D warnings` passes

## Technical Approach

### Tauri commands (`src/gui/mod.rs`)

```rust
#[tauri::command]
fn add_comment(pr_id: String, file_path: String, start_line: u32, end_line: Option<u32>, body: String) -> Result<ReviewComment, String>

#[tauri::command]
fn list_comments(pr_id: String) -> Result<Vec<ReviewComment>, String>

#[tauri::command]
fn update_comment(id: i64, body: String) -> Result<(), String>

#[tauri::command]
fn delete_comment(id: i64) -> Result<(), String>

#[tauri::command]
fn set_category_override(pr_id: String, file_path: String, hunk_id: String, change_type: Option<String>, attention_tags: Option<Vec<String>>) -> Result<(), String>

#[tauri::command]
fn get_category_overrides(pr_id: String) -> Result<Vec<CategoryOverride>, String>
```

### Inline comment component (`CommentInput.svelte`)

Appears below a diff line when the user clicks a line number or chooses "Add
comment" from context menu. Shows:
- Textarea with auto-focus
- Save button (calls `add_comment` Tauri command)
- Cancel button (closes without saving)

### Draft comment display (`DraftComment.svelte`)

Shows between diff lines where comments exist:
- Yellow-tinted background
- Comment body text
- Edit button (re-opens textarea)
- Delete button (calls `delete_comment`)

### Context menu (`ContextMenu.svelte`)

Custom right-click menu (no browser default) on diff lines:
- "Add comment" → opens `CommentInput`
- "Override category" → submenu listing CHANGE_TYPES with radio selection
- "Override tags" → submenu listing ATTENTION_TAGS with checkboxes
- Clicking outside or pressing Escape closes

### Wiring into diff renderers

Both `InlineDiff` and `SplitDiff` need:
1. Click handler on line numbers to open comment input
2. Context menu handler (right-click) on lines
3. Insert `DraftComment` blocks between diff lines where comments exist

### Store

```typescript
export const draftComments = writable<ReviewComment[]>([]);
export const categoryOverrides = writable<Map<string, CategoryOverride>>(new Map());
```

Load on review screen mount via Tauri commands.

## Files to Create/Modify
- `src/gui/mod.rs` — add 6 Tauri commands
- `frontend/src/lib/types.ts` — add ReviewComment, CategoryOverride types
- `frontend/src/lib/stores.ts` — add comment/override stores
- `frontend/src/lib/components/CommentInput.svelte`
- `frontend/src/lib/components/DraftComment.svelte`
- `frontend/src/lib/components/ContextMenu.svelte`
- `frontend/src/lib/components/InlineDiff.svelte` — add click + context menu
- `frontend/src/lib/components/SplitDiff.svelte` — same
- `frontend/src/lib/components/ReviewScreen.svelte` — load comments on mount

## Dependencies
- Depends on: PR-0 (comment storage), Epic 8 (diff renderers)
- Blocks: PR-2 (submission needs comments to submit)

## Design reference
- No specific design screenshot for comments — follow standard code review UX
  (GitHub-style inline comments)
