# PR-2: Review submission to GitHub

## Goal
Submit review comments and a review verdict (Approve / Request Changes / Comment)
to the GitHub REST API. Add a "Submit review" UI in the review screen.

## Non-goals
- No BitBucket submission (Epic 10).
- No reply threading.
- No bulk comment editing before submission.

## Success Criteria
- [ ] `GithubClient::submit_review()` method: creates a pull request review with
      inline comments via `POST /repos/{owner}/{repo}/pulls/{number}/reviews`
- [ ] Review body includes the Pass 1 summary (optional, user-controllable)
- [ ] Three verdict options: Approve, Request Changes, Comment
- [ ] Draft comments are converted to review comment payloads (file, line, body)
- [ ] After successful submission, comments are marked as `Submitted` in local DB
- [ ] Submit review UI: button in review screen header or toolbar area, opens a
      modal/dialog with verdict selection and optional body text
- [ ] Tauri command: `submit_review(pr_number, verdict, body, include_summary)`
- [ ] Error handling: show error toast if submission fails, don't mark as submitted
- [ ] Unit tests for review payload construction
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes
- [ ] `cd frontend && npm run build` succeeds
- [ ] `cd frontend && npm run check` passes

## Technical Approach

### GitHub API (`src/platform/github.rs`)

```rust
pub async fn submit_review(
    &self,
    owner: &str,
    repo: &str,
    pr_number: u64,
    event: ReviewEvent,
    body: Option<&str>,
    comments: Vec<ReviewCommentPayload>,
) -> Result<(), PlatformError>
```

Where:
```rust
pub enum ReviewEvent {
    Approve,
    RequestChanges,
    Comment,
}

pub struct ReviewCommentPayload {
    pub path: String,
    pub line: u32,
    pub body: String,
}
```

Uses `POST /repos/{owner}/{repo}/pulls/{pr_number}/reviews` with:
```json
{
  "event": "APPROVE" | "REQUEST_CHANGES" | "COMMENT",
  "body": "...",
  "comments": [{ "path": "...", "line": ..., "body": "..." }]
}
```

### GithubOperations trait

Add `submit_review` to the `GithubOperations` trait. Implement for both
`GithubClient` (REST API) and `GhClient` (gh CLI: `gh pr review`).

### Tauri command (`src/gui/mod.rs`)

```rust
#[tauri::command]
async fn submit_review(
    pr_number: u64,
    verdict: String,
    body: Option<String>,
    include_summary: bool,
) -> Result<(), String>
```

1. Load draft comments from cache
2. Build review comment payloads
3. If `include_summary`, prepend Pass 1 summary to body
4. Call `github_client.submit_review(...)`
5. On success, mark all comments as Submitted in cache
6. Return success/error

### Submit review dialog (`SubmitReviewDialog.svelte`)

Modal overlay with:
- Three radio buttons: Comment, Approve, Request Changes
- Optional body textarea (pre-filled with summary if toggled)
- "Include analysis summary" checkbox
- Comment count: "N draft comments will be submitted"
- Submit button (calls Tauri command)
- Cancel button
- Loading state during submission
- Error display on failure

### Tests

- `submit_review_payload_construction` — verify JSON matches GitHub API spec
- `review_event_serialization` — verify APPROVE/REQUEST_CHANGES/COMMENT strings
- `submitted_comments_marked` — verify status updates in cache after success

## Files to Create/Modify
- `src/platform/mod.rs` — add `ReviewEvent`, `ReviewCommentPayload` types,
  `submit_review` to `GithubOperations` trait
- `src/platform/github.rs` — implement `submit_review` for REST API
- `src/platform/github_gh.rs` — implement `submit_review` for gh CLI
- `src/gui/mod.rs` — add `submit_review` Tauri command
- `frontend/src/lib/components/SubmitReviewDialog.svelte`
- `frontend/src/lib/components/ReviewScreen.svelte` — add submit button

## Dependencies
- Depends on: PR-1 (comment UI, draft comments exist)
- Blocks: nothing (epic complete after this PR)
