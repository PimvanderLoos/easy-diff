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

Starting from wherever STATE.md says you are, for each remaining epic up to
and including Epic {LAST_EPIC}:

### Step 1: Generate PR plans (if they don't already exist)

Check if .plan/{N}-{epic-slug}/ exists and contains PR plan files.

If not, generate them:
- Read CLAUDE.md, PLAN.md, ROADMAP.md, STATE.md
- Generate detailed PR plan files following the exact template used in
  previous epics
- Keep each PR to roughly 200-700 lines of code changed
- Be specific in Technical Approach and Files to Create/Modify
- Commit the plans: `git add .plan/ && git commit -m "docs: add PR plans for epic {N}" && git push`

### Step 2: Execute the epic

Spawn a `sonnet 4.6 medium` sub-agent with the contents of .plan/templates/epic-agent.md,
substituting the appropriate epic number, title, and slug.

Wait for it to complete.

### Step 3: Verify and advance

After the epic agent reports completion:
1. Read the updated STATE.md
2. Verify the epic is marked complete
3. If the agent reported a failure, STOP and report the failure. Do not
   proceed to the next epic.
4. If successful, proceed to the next epic.

## Critical Rules

- NEVER skip a failing epic. Fix it or stop.
- NEVER modify PLAN.md or ROADMAP.md.
- Always ensure you're on the main branch with latest changes before
  starting a new epic.
- If STATE.md indicates work is already in progress, resume from that
  point — do not redo completed work.
- Keep your own context lean. All implementation happens in sub-agents.
  You only orchestrate, generate plans, and verify.
- Read this file after completion of every epic.
