# Epic Agent Instructions

You are implementing Epic {EPIC_NUMBER} ({EPIC_TITLE}) of the easy-diff project.
You are a senior developer experienced in Rust, and in Svelte + TypeScript when
the epic involves frontend work.

## Setup

1. Read these files to understand the project:
   - CLAUDE.md
   - .plan/PLAN.md
   - .plan/ROADMAP.md
   - .plan/STATE.md

2. Read all PR plan files in .plan/{EPIC_SLUG}/ in order (PR-0, PR-1, ...).

3. Check if .plan/designs/ exists. If this epic involves frontend/GUI work,
   read the design screenshots (PNG files) and source files (JSX/JS) before
   implementing. The JSX files are React — use them as structural and behavioral
   reference only; translate to idiomatic Svelte + Tailwind, not line-by-line
   ports.

4. Determine if this epic is backend-only or includes frontend work. This
   affects which checks you run (see Step 1 below).

## Process

For each PR plan file, in order:

### Step 1: Implement

Spawn a sub-agent with the following instructions:

---
Read CLAUDE.md, .plan/PLAN.md, .plan/STATE.md, and .plan/{EPIC_SLUG}/{PR_FILE}.

If this PR involves frontend/GUI work, also read the design files in
.plan/designs/ — the PNGs are the visual target, the JSX/JS files define
component structure, design tokens, and data shapes. Translate to idiomatic
Svelte + Tailwind; do not port React patterns (useState → Svelte reactivity,
inline styles → Tailwind utilities, React.memo → Svelte's built-in reactivity).

Implement this PR exactly as specified. Follow all conventions from CLAUDE.md.
Stay within scope — do not implement anything listed in the PR's non-goals.

After implementation, run all applicable checks:

**Always (Rust):**
1. `cargo build` — must succeed
2. `cargo test` — all tests must pass
3. `cargo clippy -- -D warnings` — must pass
4. `cargo fmt --check` — must pass

**If this PR includes frontend code (files in frontend/):**
5. `cd frontend && npm install` (only if dependencies changed)
6. `cd frontend && npm run build` — must succeed
7. `cd frontend && npm run check` — TypeScript/Svelte checks must pass

**Then verify:**
8. Re-read the PR plan's success criteria and verify each is met. Report status
   of each criterion.
9. Re-read the PR plan's non-goals and confirm none were implemented.
10. List any concerns about your own implementation.
11. If any open questions from the PR plan needed resolution, state your decision.

If any check fails, fix the issue and re-run. If you cannot fix it after 3
attempts, stop and report the failure clearly.

Do not modify any files in .plan/.

When done:
1. Stage all changes: `git add -A`
2. Commit with a conventional commit message (feat:, fix:, refactor:, etc.)
3. Push to a new branch: `git checkout -b {EPIC_SLUG}/pr-{N} && git push -u origin {EPIC_SLUG}/pr-{N}`
---

### Step 2: Create PR and wait for CI

After the sub-agent completes successfully:
1. Create a GitHub PR: `gh pr create --base main --fill`
2. Wait for CI: `gh pr checks --watch`
3. If CI fails, spawn a sub-agent to read the CI logs (`gh run view --log-failed`),
   fix the issues, push, and re-check. Max 3 attempts.
4. When CI passes, merge: `gh pr merge --squash --delete-branch`
5. Switch back to main and pull: `git checkout main && git pull`

### Step 3: Update state

Update .plan/STATE.md to reflect the PR is complete. Include what was completed,
any decisions made, concerns raised, and that the next step is the following PR
(or that the epic is complete).

Commit and push the state update directly to main.

### After all PRs in this epic are done:

1. Re-read .plan/ROADMAP.md for this epic's goals.
2. Verify each goal is met by inspecting the codebase.
3. Update .plan/STATE.md with:
   - Epic marked complete
   - All goals verified (or note which aren't met)
   - Next epic identified
4. Commit and push.

## Error Handling

- If a PR sub-agent fails after 3 attempts: stop the entire epic, update
  STATE.md with the failure details, and report back to the parent.
- If CI fails after 3 fix attempts: same — stop, document, report.
- Never skip a failing PR to work on the next one. PRs are sequential
  and may depend on each other.
