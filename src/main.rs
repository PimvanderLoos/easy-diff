use anyhow::{Context as _, Result};
use clap::Parser;
use std::sync::Arc;

mod analysis;
mod cache;
mod categories;
mod config;
mod diff;
mod git;
mod llm;
mod platform;
mod tui;

use analysis::{AnalysisEngine, PrContext};
use cache::CacheStore;
use platform::github::GithubClient;
use platform::{Platform, PullRequest};

/// LLM-powered PR review tool.
#[derive(Parser)]
#[command(name = "easy-diff", about = "LLM-powered PR review tool", version)]
struct Cli {
    /// Fetch and display the diff for a specific PR number.
    ///
    /// When omitted in an interactive terminal, a selection menu is shown.
    #[arg(long)]
    pr: Option<u64>,

    /// Run two-pass LLM analysis on the fetched diff and print results as JSON.
    ///
    /// In non-interactive mode (i.e. `--pr N` given without a TTY), this flag
    /// must be set explicitly. In interactive mode analysis runs automatically.
    #[arg(long)]
    analyze: bool,

    /// Bypass the analysis cache and re-run the full LLM analysis.
    ///
    /// New results are still written to the cache. Requires `--analyze`.
    #[arg(long, requires = "analyze")]
    refresh: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    // 1. Detect repo
    let repo_info = git::detect_repo_info(".")
        .context("not a git repository — run easy-diff from within a git repo")?;
    tracing::info!(platform = %repo_info.platform, owner = %repo_info.owner,
                   repo = %repo_info.repo, "detected repository");

    // 2. Load config (uses repo root for per-repo config)
    let config = config::load_config(Some(&repo_info.root))?;

    // 3. Create LLM dispatcher
    let dispatcher = Arc::new(llm::create_dispatcher(&config));
    tracing::info!(provider = dispatcher.provider_name(), "LLM provider ready");

    // 4. Check platform
    if repo_info.platform != Platform::GitHub {
        anyhow::bail!(
            "only GitHub repositories are currently supported (detected: {})",
            repo_info.platform
        );
    }

    // 5. Get token
    let token = config.github.token.ok_or_else(|| {
        anyhow::anyhow!(
            "GitHub token not configured — add [github] token = \"...\" to {}",
            config::global_config_path()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "~/.config/easy-diff/config.toml".into())
        )
    })?;

    // 6. Create client
    let client = GithubClient::new(token);

    // 7. Determine whether we are in interactive mode (TTY available)
    let is_interactive = atty::is(atty::Stream::Stdin);

    // 8. Resolve PR number — either from `--pr`, or via interactive selection
    let pr_number: u64 = if let Some(n) = cli.pr {
        n
    } else if is_interactive {
        // Fetch PR list and let the user pick
        let prs = client
            .list_open_pull_requests(&repo_info.owner, &repo_info.repo)
            .await
            .context("failed to fetch open pull requests")?;

        if prs.is_empty() {
            println!(
                "No open pull requests for {}/{}",
                repo_info.owner, repo_info.repo
            );
            return Ok(());
        }

        let idx = tui::select_pr(&prs).context("PR selection failed")?;
        prs[idx].number
    } else {
        // Non-interactive, no --pr: print list and exit (legacy behaviour)
        let prs = client
            .list_open_pull_requests(&repo_info.owner, &repo_info.repo)
            .await
            .context("failed to fetch open pull requests")?;
        print_pr_list(&repo_info.owner, &repo_info.repo, &prs);
        return Ok(());
    };

    // 9. Fetch PR metadata and diff in parallel
    let (pr_meta, diff) = tokio::try_join!(
        client.get_pull_request(&repo_info.owner, &repo_info.repo, pr_number),
        client.get_pull_request_diff(&repo_info.owner, &repo_info.repo, pr_number),
    )
    .with_context(|| format!("failed to fetch PR #{pr_number}"))?;

    // 10. Decide whether to run analysis:
    //     - always in interactive mode
    //     - only if --analyze was passed in non-interactive mode
    let should_analyze = is_interactive || cli.analyze;

    // 10. Estimate tokens — before analysis so we can warn the user early.
    let estimated_tokens = diff::estimate_tokens(&diff.diff);
    tracing::debug!(estimated_tokens, "token estimate for diff");

    if should_analyze {
        // 11. Confirm for large PRs in interactive mode.
        if is_interactive {
            let confirmed =
                tui::confirm_large_pr(estimated_tokens, config.preferences.large_pr_threshold)
                    .context("large-PR confirmation failed")?;
            if !confirmed {
                println!("Aborted.");
                return Ok(());
            }
        }

        let cache_path = repo_info.root.join(".easy-diff/cache/analysis.db");
        let cache = CacheStore::open(&cache_path)
            .with_context(|| format!("failed to open cache at {}", cache_path.display()))
            .ok();
        if cache.is_none() {
            tracing::warn!(path = %cache_path.display(), "could not open cache — analysis will proceed without caching");
        }

        let pr_ctx = PrContext {
            pr_number,
            base_sha: pr_meta.base_sha.clone(),
            head_sha: pr_meta.head_sha.clone(),
        };

        let engine = AnalysisEngine::new(
            Arc::clone(&dispatcher),
            config.preferences.large_pr_threshold,
            5,
            config.preferences.max_file_context,
            cache,
            cli.refresh,
        );
        let result = engine
            .run(&diff.diff, &pr_ctx)
            .await
            .context("analysis failed")?;

        if is_interactive {
            // Display human-readable summary, then run filter selection.
            tui::display_summary(&result.pass1);

            match tui::select_filters() {
                Ok((change_types, attention_tags)) => {
                    // Parse, filter, render, and print to stdout.
                    let parsed_files = diff::parse_diff(&diff.diff);
                    let filtered = diff::filter_files(
                        &parsed_files,
                        &result.files,
                        &change_types,
                        &attention_tags,
                    );
                    let rendered = diff::render_filtered_diff(&filtered, &result.files);
                    print!("{rendered}");
                }
                Err(e) => {
                    tracing::warn!(error = %e, "filter selection failed — printing unfiltered diff");
                    print!("{}", diff.diff);
                }
            }
        } else {
            // Non-interactive: print JSON (original --analyze behaviour)
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
    } else {
        // No analysis requested in non-interactive mode: print raw diff
        print!("{}", diff.diff);
    }

    Ok(())
}

fn print_pr_list(owner: &str, repo: &str, prs: &[PullRequest]) {
    if prs.is_empty() {
        println!("No open pull requests for {owner}/{repo}.");
        return;
    }

    println!("Open pull requests for {owner}/{repo}:\n");

    // Determine column widths from data.
    let max_num_width = prs.iter().map(|pr| digits(pr.number)).max().unwrap_or(1);

    for pr in prs {
        let num_str = format!("#{}", pr.number);
        let date = pr.updated_at.get(..10).unwrap_or(&pr.updated_at);
        println!(
            "  {num:<num_width$}  {title:<45}  {author:<20}  {date}",
            num = num_str,
            num_width = max_num_width + 1, // +1 for '#'
            title = truncate(&pr.title, 45),
            author = truncate(&pr.author, 20),
            date = date,
        );
    }
}

fn truncate(s: &str, max: usize) -> std::borrow::Cow<'_, str> {
    if s.len() <= max {
        std::borrow::Cow::Borrowed(s)
    } else {
        std::borrow::Cow::Owned(format!("{}…", &s[..max.saturating_sub(1)]))
    }
}

fn digits(n: u64) -> usize {
    if n == 0 {
        1
    } else {
        n.ilog10() as usize + 1
    }
}
