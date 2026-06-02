---
name: update-state
description: Update .plan/STATE.md at the end of a coding session using the project's mandated 5-point format. Invoke at session end (or when the user types /update-state) to record what was completed, what is in progress, plan divergences, new tech debt, and what to work on next.
disable-model-invocation: true
---

# update-state

Update `.plan/STATE.md` — the project's single source of truth for session-to-session
progress. CLAUDE.md mandates this file be updated at the end of **every** coding session,
so this skill exists to make that update consistent and complete.

## When to run

- The user types `/update-state`.
- At the end of a coding session, when the user asks to wrap up / save progress.

This skill is **user-invoked only** (`disable-model-invocation: true`) — it writes a
tracked project file, so it must not fire on its own.

## Steps

1. **Gather what changed this session.** Do not guess — inspect:
   - `git log --oneline <last-recorded-commit>..HEAD` for commits since the last entry.
   - `git status` and `git diff --stat` for uncommitted work in progress.
   - The current branch name (`git branch --show-current`).
   - Re-read the conversation for decisions, divergences, and follow-ups that are not
     captured in commit messages.

2. **Read the current `.plan/STATE.md`.** Preserve its existing structure and its
   completed-work history. You are appending/updating, never rewriting history.
   The file uses these top sections:
   - `## Status:` — one line (e.g. `Active Development`).
   - `## Current Epic:` / `## Next Epic:` — update if the epic changed.
   - `## Completed` — a running, append-only bulleted list. Add new entries here in
     the established style (`- **Epic N PR-M**: ... PR #X on GitHub.`). Match the
     surrounding wording density and detail level; include what was implemented, the
     key public API/types touched, test counts, and the PR number when there is one.

3. **Apply the mandated 5-point format.** Every update must account for all five, even
   if a point is "none this session":
   1. **What was completed** → new `## Completed` bullet(s).
   2. **What is in progress** → an `## In Progress` section (create if absent); clear
      stale items that are now done.
   3. **Decisions that diverge from the plan** → a `## Divergences` section. Per
      CLAUDE.md, document divergences here — do **not** silently edit `.plan/*` spec
      files.
   4. **New issues / tech debt introduced** → a `## Tech Debt` section.
   5. **What to work on next** → a `## Next` section, and update `## Next Epic:` if it
      moved.

4. **Write the file** with the Edit tool (surgical edits to the relevant sections),
   not a full overwrite — this preserves history and minimizes the diff.

5. **Report back** a short summary of which sections you changed, and remind the user
   to commit `.plan/STATE.md` (conventional commit, e.g.
   `docs: update STATE.md for <work>`). Do not commit unless the user asks.

## Guardrails

- Never delete or rewrite past `## Completed` entries — the section is append-only.
- Never edit `.plan/PLAN.md` or `.plan/ROADMAP.md` to resolve a divergence; record the
  divergence in STATE.md instead.
- If you cannot determine real progress from git + the conversation, ask the user
  rather than inventing entries.
