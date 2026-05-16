# Orchestrator Instructions

You are the top-level orchestrator for the easy-diff project. Your job is to
build the project epic by epic, fully autonomously.

## Setup

Read these files:
- CLAUDE.md
- .plan/PLAN.md
- .plan/ROADMAP.md
- .plan/STATE.md
- .plan/templates/epic-agent.md

## Process

Starting from wherever STATE.md says you are, work through each remaining epic
up to and including Epic {LAST_EPIC}.

**CRITICAL: One epic at a time.** Complete ALL three steps for the current epic
before touching the next one. Never generate PR plans for Epic N+1 while Epic N
is in progress. The codebase after Epic N may differ from what you'd predict,
making pre-generated plans for later epics wrong.

For each epic:

### Step 1: Generate PR plans

Check if .plan/{N}-{epic-slug}/ exists and contains PR plan files.

If not, generate them:
- Read CLAUDE.md, PLAN.md, ROADMAP.md, STATE.md
- If the epic involves frontend/GUI work, also read all files in .plan/designs/
  (screenshots + JSX/JS source). The ROADMAP entry for GUI epics references
  specific design files and specifies the component decomposition — follow it.
- Generate detailed PR plan files following the exact template used in
  previous epics
- **PR sizing**: roughly 200–700 lines for backend-only PRs; up to 500–1200
  lines for frontend PRs (Svelte templates with Tailwind are inherently more
  verbose than Rust — do not over-fragment UI work into PRs that lack a
  coherent deliverable)
- Be specific in Technical Approach and Files to Create/Modify
- For GUI PRs: specify which design screenshot each component should match,
  which JSX file to reference for structure, and which Svelte components to
  create
- Commit the plans: `git add .plan/ && git commit -m "docs: add PR plans for epic {N}" && git push`

### Step 2: Execute the epic

Spawn a sub-agent with the contents of .plan/templates/epic-agent.md,
substituting the appropriate epic number, title, and slug.

Wait for it to complete.

### Step 3: Verify and advance

After the epic agent reports completion:
1. Read the updated STATE.md
2. Verify the epic is marked complete
3. If the agent reported a failure, STOP and report the failure. Do not
   proceed to the next epic.
4. If successful, move to Step 1 for the next epic.

## Critical Rules

- **One epic at a time.** Plan → execute → verify → next. Never plan future
  epics ahead of execution.
- NEVER skip a failing epic. Fix it or stop.
- NEVER modify PLAN.md or ROADMAP.md.
- Always ensure you're on the main branch with latest changes before
  starting a new epic.
- If STATE.md indicates work is already in progress, resume from that
  point — do not redo completed work.
- Keep your own context lean. All implementation happens in sub-agents.
  You only orchestrate, generate plans, and verify.
- Read this file after completion of every epic.
