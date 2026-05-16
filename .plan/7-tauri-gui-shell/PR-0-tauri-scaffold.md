# PR-0: Tauri v2 + Svelte + TypeScript + Vite scaffold with design tokens

## Goal
Set up the Tauri v2 project integrated with the existing Rust crate. Create the
Svelte + TypeScript + Vite frontend scaffold in `frontend/`. Configure Tailwind
CSS v4 with the design token palette from `shared.jsx` (`ED_TOKENS`) mapped to
CSS custom properties, supporting light and dark themes. Set up IBM Plex fonts.
Update CI to also build the frontend.

## Non-goals
- No components beyond a placeholder "Hello easy-diff" page.
- No Tauri commands yet (PR-1).
- No PR selection screen (PR-2).

## Success Criteria
- [ ] Tauri v2 project structure: `frontend/` with Svelte + TS + Vite, integrated
      with `Cargo.toml` via Tauri feature gate or workspace setup
- [ ] `cd frontend && npm install && npm run build` succeeds
- [ ] `cd frontend && npm run check` (svelte-check) passes
- [ ] Tailwind CSS v4 configured with design tokens from `ED_TOKENS` (both dark
      and light themes) as CSS custom properties
- [ ] Dark/light theme toggle: adding `class="dark"` to `<html>` switches all
      token values
- [ ] IBM Plex Sans + IBM Plex Mono loaded via `@fontsource` packages
- [ ] Tauri app launches and shows a placeholder page with correct fonts and
      theme colors
- [ ] CI workflow updated to include frontend build + check steps
- [ ] `cargo build` still succeeds (Tauri feature-gated or workspace-separated)
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes

## Technical Approach

### Tauri v2 setup

Use `cargo tauri init` or manual setup to create the Tauri v2 structure. The Rust
side lives in the existing crate; the frontend in `frontend/`.

Two approaches for integrating with the existing binary:
1. **Feature gate**: Add a `gui` feature to `Cargo.toml` that enables Tauri deps.
   Default features include CLI-only deps. `cargo build` builds CLI, `cargo build
   --features gui` builds the desktop app.
2. **Workspace member**: Create `src-tauri/` as a separate workspace member that
   depends on the main crate as a library.

Prefer approach 1 (feature gate) for simplicity — the Tauri-specific code (commands,
window setup) lives in a `src/gui/` module behind `#[cfg(feature = "gui")]`.

### Frontend scaffold (`frontend/`)

```
frontend/
├── package.json
├── tsconfig.json
├── vite.config.ts
├── svelte.config.js
├── src/
│   ├── app.css          # Tailwind directives + CSS custom properties
│   ├── app.d.ts         # SvelteKit types
│   ├── App.svelte       # Root component (placeholder)
│   ├── main.ts          # Entry point
│   └── lib/
│       └── tokens.ts    # Exported token constants (for JS-side use)
├── index.html
└── static/
```

### Design tokens (`app.css`)

Map `ED_TOKENS` from `shared.jsx` to CSS custom properties. Use Tailwind's
`@theme` or manual `@layer` to wire them:

```css
:root {
  --ed-bg: #ffffff;
  --ed-panel: #f6f8fa;
  --ed-text: #1f2328;
  /* ... all light theme tokens ... */
}

.dark {
  --ed-bg: #0d1117;
  --ed-panel: #161b22;
  --ed-text: #e6edf3;
  /* ... all dark theme tokens ... */
}
```

Tailwind config extends with `bg: 'var(--ed-bg)'`, etc.

### Fonts

```bash
npm install @fontsource/ibm-plex-sans @fontsource/ibm-plex-mono
```

Import in `app.css`:
```css
@import '@fontsource/ibm-plex-sans/400.css';
@import '@fontsource/ibm-plex-sans/500.css';
@import '@fontsource/ibm-plex-sans/600.css';
@import '@fontsource/ibm-plex-sans/700.css';
@import '@fontsource/ibm-plex-mono/400.css';
@import '@fontsource/ibm-plex-mono/500.css';
```

### CI update

Add steps to `.github/workflows/ci.yml`:
```yaml
- name: Frontend install
  working-directory: frontend
  run: npm ci
- name: Frontend build
  working-directory: frontend
  run: npm run build
- name: Frontend check
  working-directory: frontend
  run: npm run check
```

### Placeholder App.svelte

A simple page showing:
- The easydiff brand mark (diamond SVG from `concept-b.jsx`)
- "easy-diff" title in IBM Plex Sans
- A theme toggle button that adds/removes `dark` class on `<html>`
- Background and text colors using the design token CSS vars

This validates fonts, tokens, and theming work end-to-end.

## Files to Create/Modify
- `frontend/` — entire scaffold (package.json, vite.config.ts, etc.)
- `frontend/src/app.css` — Tailwind + design tokens
- `frontend/src/App.svelte` — placeholder root
- `frontend/src/main.ts` — entry point
- `Cargo.toml` — Tauri deps behind feature gate
- `src/gui/mod.rs` — minimal Tauri setup behind `#[cfg(feature = "gui")]`
- `src/main.rs` — conditional GUI launch
- `.github/workflows/ci.yml` — add frontend steps
- `tauri.conf.json` or `src-tauri/tauri.conf.json` — Tauri config

## Dependencies
- Depends on: nothing (first frontend PR)
- Blocks: PR-1 (Tauri commands), PR-2 (PR selection screen)

## Open Questions
- Feature gate vs workspace member for Tauri integration? Leaning feature gate.
- Tailwind v4 uses a different config approach (CSS-based `@theme` instead of
  `tailwind.config.js`). Need to verify the latest stable setup.
