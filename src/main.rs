use anyhow::{Context as _, Result};
use clap::Parser;

mod analysis;
mod cache;
mod categories;
mod config;
mod diff;
mod git;
mod llm;
mod platform;
mod tui;

use platform::github::GithubClient;
use platform::{Platform, PullRequest};

/// LLM-powered PR review tool.
#[derive(Parser)]
#[command(name = "easy-diff", about = "LLM-powered PR review tool", version)]
struct Cli {
    /// Fetch and display the diff for a specific PR number.
    #[arg(long)]
    pr: Option<u64>,
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
    tracing::info!(provider = ?config.llm.default_provider, "resolved LLM provider");

    // 3. Check platform
    if repo_info.platform != Platform::GitHub {
        anyhow::bail!(
            "only GitHub repositories are currently supported (detected: {})",
            repo_info.platform
        );
    }

    // 4. Get token
    let token = config.github.token.ok_or_else(|| {
        anyhow::anyhow!(
            "GitHub token not configured — add [github] token = \"...\" to {}",
            config::global_config_path()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "~/.config/easy-diff/config.toml".into())
        )
    })?;

    // 5. Create client and dispatch
    let client = GithubClient::new(token);

    match cli.pr {
        Some(pr_number) => {
            let diff = client
                .get_pull_request_diff(&repo_info.owner, &repo_info.repo, pr_number)
                .await
                .with_context(|| format!("failed to fetch diff for PR #{pr_number}"))?;
            print!("{}", diff.diff);
        }
        None => {
            let prs = client
                .list_open_pull_requests(&repo_info.owner, &repo_info.repo)
                .await
                .context("failed to fetch open pull requests")?;
            print_pr_list(&repo_info.owner, &repo_info.repo, &prs);
        }
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
