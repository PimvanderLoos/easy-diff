# PR-1: Tauri commands and shared Svelte components

## Goal
Expose Tauri `invoke()` commands for `list_pull_requests`, `get_repo_info`, and
`get_config`. Build the shared primitive Svelte components (`Avatar`, `Tag`,
`SidebarRow`, `SidebarLabel`, `SeverityDot`) that will be used across screens.

## Non-goals
- No PR selection screen layout (PR-2).
- No routing/navigation.
- No review screen components (Epic 8).

## Success Criteria
- [ ] Tauri commands `list_pull_requests`, `get_repo_info`, `get_config` are
      callable from the frontend via `invoke()`
- [ ] TypeScript types for `PullRequest`, `RepoInfo`, `Config` match the Rust
      structs
- [ ] `Avatar.svelte` renders a deterministic-color monogram circle matching
      `EDAvatar` from `shared.jsx`
- [ ] `Tag.svelte` renders a restrained chip/badge matching `EDTag`
- [ ] `SidebarRow.svelte` renders a clickable row with active state
- [ ] `SidebarLabel.svelte` renders an uppercase section header
- [ ] `SeverityDot.svelte` renders a colored circle for risky/safe/trivial
- [ ] All components use design token CSS vars and respond to dark/light toggle
- [ ] `cd frontend && npm run build` succeeds
- [ ] `cd frontend && npm run check` passes
- [ ] `cargo build` succeeds (CLI mode)
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes

## Technical Approach

### Tauri commands (`src/gui/mod.rs`)

```rust
#[tauri::command]
async fn list_pull_requests() -> Result<Vec<PullRequest>, String> { ... }

#[tauri::command]
async fn get_repo_info() -> Result<RepoInfo, String> { ... }

#[tauri::command]
async fn get_config() -> Result<Config, String> { ... }
```

These wrap the existing platform client and config loading. Error handling maps
`anyhow` errors to `String` for Tauri's IPC.

Ensure `PullRequest`, `RepoInfo`, and `Config` derive `Serialize` for Tauri.

### Frontend types (`frontend/src/lib/types.ts`)

```typescript
export interface PullRequest {
  number: number;
  title: string;
  author: string;
  source_branch: string;
  target_branch: string;
  base_sha: string;
  head_sha: string;
  created_at: string;
  updated_at: string;
}

export interface RepoInfo {
  owner: string;
  repo: string;
  platform: 'GitHub' | 'BitBucket';
  remote_name: string;
  remote_url: string;
  host: string;
}
```

### Shared components (`frontend/src/lib/components/`)

Each component is a `.svelte` file using Tailwind utilities and CSS custom
properties from the token system.

**Avatar.svelte** — Reference: `EDAvatar` in `shared.jsx`
- Props: `name: string`, `size: number = 22`
- Deterministic hue from name (same hash algorithm as `shared.jsx`)
- Monogram from initials (split on `.`, space, `_`, `-`)
- Uses `oklch()` for background color

**Tag.svelte** — Reference: `EDTag` in `shared.jsx`
- Props: `color?: string`, slot for children
- IBM Plex Mono, 11px, border + subtle background

**SidebarRow.svelte** — Reference: `SidebarRow` in `concept-b.jsx`
- Props: `active: boolean = false`, `disabled: boolean = false`
- Flex row with justify-between, rounded-md, accent background when active

**SidebarLabel.svelte** — Reference: `SidebarLabel` in `concept-b.jsx`
- Props: slot for children
- 10px uppercase, letter-spacing 1.6, IBM Plex Mono

**SeverityDot.svelte** — Reference: `SeverityDot` in `shared.jsx`
- Props: `severity: 'risky' | 'safe' | 'trivial'`, `size: number = 8`
- Colored circle using token colors

### Tests

Since these are visual components, testing is limited to:
- TypeScript type checking via `svelte-check`
- Build verification

## Files to Create/Modify
- `src/gui/mod.rs` — add Tauri command functions
- `frontend/src/lib/types.ts` — TypeScript interfaces
- `frontend/src/lib/components/Avatar.svelte`
- `frontend/src/lib/components/Tag.svelte`
- `frontend/src/lib/components/SidebarRow.svelte`
- `frontend/src/lib/components/SidebarLabel.svelte`
- `frontend/src/lib/components/SeverityDot.svelte`

## Dependencies
- Depends on: PR-0 (Tauri scaffold, design tokens)
- Blocks: PR-2 (PR selection screen uses these components)
