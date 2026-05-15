use anyhow::Result;
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

/// CLI args (empty for now, just --help/--version).
#[derive(Parser)]
#[command(name = "easy-diff", about = "LLM-powered PR review tool")]
struct Cli {}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let _cli = Cli::parse();
    tracing::info!("easy-diff starting");
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_harness_works() {
        assert_eq!(2 + 2, 4);
    }
}
