use anyhow::{Context as _, Result};
use clap::Parser;
use std::sync::Arc;

mod analysis;
mod cache;
mod categories;
mod config;
mod diff;
mod git;
#[cfg(feature = "gui")]
mod gui;
mod llm;
mod platform;
mod tui;

use analysis::{AnalysisEngine, PrContext};
use cache::CacheStore;
use platform::{create_platform_client, PullRequest};

/// LLM-powered PR review tool.
#[derive(Parser)]
#[command(name = "easy-diff", about = "LLM-powered PR review tool", version)]
struct Cli {
    /// Launch the graphical user interface.
    ///
    /// Only available when compiled with the `gui` feature.
    #[cfg(feature = "gui")]
    #[arg(long)]
    gui: bool,

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

    /// Mark all displayed files as viewed at the current head SHA after rendering.
    ///
    /// Stored in the cache database so future runs can highlight only new changes.
    #[arg(long)]
    mark_viewed: bool,

    /// Show only files that have changes since they were last viewed.
    ///
    /// Requires that at least one file has been previously marked as viewed via
    /// `--mark-viewed`. When combined with category filters, both filters apply
    /// (intersection). When all files have been reviewed, prints a notice and exits.
    #[arg(long)]
    unreviewed: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    // 1. Detect repo (optional — GUI can start without one)
    let repo_info = git::detect_repo_info(".").ok();
    if let Some(ref info) = repo_info {
        tracing::info!(platform = %info.platform, owner = %info.owner,
                       repo = %info.repo, "detected repository");
    }

    // 2. Load config (uses repo root for per-repo config when available)
    let config = config::load_config(repo_info.as_ref().map(|r| r.root.as_path()))?;

    // 3. Verify account identity — fatal if detection fails
    let default_settings = config.llm.settings_for(&config.llm.default_provider);
    let account = llm::account::detect_account(&config.llm.default_provider, default_settings)
        .await
        .context("failed to detect LLM account — check your provider config and authentication")?;
    tracing::info!(
        provider = ?config.llm.default_provider,
        account = %account,
        "LLM account verified"
    );

    // Launch GUI when requested or when no CLI-specific args are given.
    #[cfg(feature = "gui")]
    if cli.gui || (cli.pr.is_none() && !cli.analyze) {
        gui::run();
        return Ok(());
    }

    // CLI mode requires a git repo
    let repo_info =
        repo_info.context("not a git repository — run easy-diff from within a git repo")?;

    // 4. Create LLM dispatcher
    let dispatcher = Arc::new(llm::create_dispatcher(&config));

    // 4. Create platform client (GitHub or BitBucket)
    let client = create_platform_client(repo_info.platform, &config, &repo_info.host)
        .await
        .context("failed to initialize platform client")?;

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
            repo_info.root.clone(),
        );
        let result = engine
            .run(&diff.diff, &pr_ctx)
            .await
            .context("analysis failed")?;

        // Collect the paths of files that will be displayed so we can mark them
        // as viewed after rendering when --mark-viewed is set.
        let displayed_files: Vec<String>;

        // Determine whether any viewed-state data is available (cache must be open
        // and the map must be non-empty to be useful).
        let has_viewed_data = !result.has_changes_since_viewed.is_empty();

        // Print a summary line when viewed data exists.
        if has_viewed_data {
            let unreviewed_count = result
                .has_changes_since_viewed
                .values()
                .filter(|&&changed| changed)
                .count();
            let total = result.has_changes_since_viewed.len();
            println!("[review] {unreviewed_count} of {total} files have changes since last review");
        }

        if is_interactive {
            // Display human-readable summary, then run filter selection.
            tui::display_summary(&result.pass1);

            // Ask whether to show only unreviewed files (only when viewed data exists).
            let use_unreviewed = if cli.unreviewed {
                true
            } else {
                tui::select_unreviewed_filter(has_viewed_data).unwrap_or(false)
            };

            match tui::select_filters() {
                Ok((change_types, attention_tags)) => {
                    // Parse the diff into structured files.
                    let parsed_files = diff::parse_diff(&diff.diff);

                    // Apply unreviewed filter first (when active), then category filters.
                    let after_unreviewed: Vec<diff::DiffFile> = if use_unreviewed {
                        diff::filter_unreviewed(&parsed_files, &result.has_changes_since_viewed)
                            .into_iter()
                            .cloned()
                            .collect()
                    } else {
                        parsed_files
                    };

                    if use_unreviewed && after_unreviewed.is_empty() {
                        println!("All files have been reviewed at the current revision.");
                        displayed_files = Vec::new();
                    } else {
                        let filtered = diff::filter_files(
                            &after_unreviewed,
                            &result.files,
                            &change_types,
                            &attention_tags,
                        );
                        displayed_files = filtered.iter().map(|f| f.path.clone()).collect();
                        let rendered = diff::render_filtered_diff(&filtered, &result.files);
                        print!("{rendered}");
                    }
                }
                Err(e) => {
                    tracing::warn!(error = %e, "filter selection failed — printing unfiltered diff");
                    displayed_files = result.files.keys().cloned().collect();
                    print!("{}", diff.diff);
                }
            }
        } else {
            // Non-interactive: apply --unreviewed filter before printing.
            if cli.unreviewed {
                let parsed_files = diff::parse_diff(&diff.diff);
                let unreviewed: Vec<diff::DiffFile> =
                    diff::filter_unreviewed(&parsed_files, &result.has_changes_since_viewed)
                        .into_iter()
                        .cloned()
                        .collect();

                if unreviewed.is_empty() {
                    println!("All files have been reviewed at the current revision.");
                    displayed_files = Vec::new();
                } else {
                    displayed_files = unreviewed.iter().map(|f| f.path.clone()).collect();
                    let rendered = diff::render_filtered_diff(&unreviewed, &result.files);
                    print!("{rendered}");
                }
            } else {
                // Non-interactive without --unreviewed: print JSON (original --analyze behaviour)
                displayed_files = result.files.keys().cloned().collect();
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
        }

        // Mark displayed files as viewed if the flag was set.
        if cli.mark_viewed {
            match CacheStore::open(&cache_path) {
                Ok(mark_cache) => {
                    let pr_id = pr_number.to_string();
                    for path in &displayed_files {
                        if let Err(e) = mark_cache.mark_viewed(&pr_id, path, &pr_meta.head_sha) {
                            tracing::warn!(file = %path, error = %e, "failed to mark file as viewed");
                        }
                    }
                    tracing::info!(count = displayed_files.len(), "files marked as viewed");
                }
                Err(e) => {
                    tracing::warn!(error = %e, "could not open cache to mark files as viewed");
                }
            }
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
