---
name: release-notes
description: Generate release notes / a CHANGELOG section from conventional commits since the last release tag. Invoke when cutting a version, tagging a release, or summarizing a batch of merged PRs for humans. Groups commits by type (feat, fix, docs, refactor, etc.) into a clean, reviewer-facing changelog.
disable-model-invocation: true
---

# release-notes

Turn the conventional-commit history since the last release into human-facing release
notes. This is a **reviewer/user-facing** artifact — distinct from `.plan/STATE.md`,
which is internal session memory (see the `update-state` skill).

## When to run

- The user types `/release-notes` (optionally with a range or version argument).
- Cutting a new version, tagging a release, or summarizing merged PRs for a release.

User-invoked only (`disable-model-invocation: true`) — it produces a release artifact,
so it must not fire on its own.

## Steps

1. **Determine the commit range.**
   - If the user gave an explicit range (`v0.1.0..HEAD`) or tag, use it.
   - Otherwise find the last release tag: `git describe --tags --abbrev=0` (fall back to
     the first commit if there are no tags) and use `<last-tag>..HEAD`.
   - State the range you chose before generating, so the user can correct it.

2. **Collect commits.** `git log <range> --pretty=format:'%s|%h'` (and `%b` if bodies
   matter). Prefer commits merged to the release branch; ignore merge commits unless
   they carry a PR title worth keeping.

3. **Group by conventional-commit type.** Map prefixes to sections, in this order:
   - `feat:` → **Features**
   - `fix:` → **Bug Fixes**
   - `perf:` → **Performance**
   - `refactor:` → **Refactoring**
   - `docs:` → **Documentation**
   - `chore:` / `build:` / `ci:` / `test:` → **Internal** (collapse; usually omit from
     user-facing notes unless the user wants the full list)
   - Anything not matching a prefix → **Other**
   - Honor scopes: `fix(gui): ...` renders under Bug Fixes with the `gui` scope shown.

4. **Flag breaking changes.** Any commit with `!` before the colon (`feat!:`) or a
   `BREAKING CHANGE:` footer goes into a top **⚠ Breaking Changes** section, called out
   first.

5. **Render the changelog.** Markdown, newest version on top. Each entry: short
   imperative description + short SHA (and PR number if present in the subject, e.g.
   `(#45)`). Drop the `type(scope):` prefix from the rendered line — the section header
   already conveys the type. Example shape:

   ```markdown
   ## v0.2.0 — <date>

   ### Features
   - Add BitBucket Cloud support (#42) — a1b2c3d

   ### Bug Fixes
   - **gui**: never fail silently on persistence errors (#45) — 3cf8c30
   ```

   Use the date the user provides, or ask — do not invent one.

6. **Output / write.** By default print the notes for the user to review. If the
   project keeps a `CHANGELOG.md`, offer to prepend the new section. Do not write or
   commit files unless the user asks.

## Guardrails

- Do not invent commits, PR numbers, or dates — derive everything from git, or ask.
- Keep `chore`/`ci`/`build` noise out of user-facing notes unless explicitly requested.
- If commits are not conventional-commit formatted, say so and fall back to a flat
  bulleted list rather than guessing categories.
