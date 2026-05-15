//! Platform API clients for GitHub and BitBucket Cloud.
//! Responsible for PR listing, diff fetching, and (future) review submission.

pub mod bitbucket;
pub mod github;

/// Supported code hosting platforms, identified from remote URLs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    GitHub,
    BitBucket,
}

impl std::fmt::Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GitHub => write!(f, "GitHub"),
            Self::BitBucket => write!(f, "BitBucket"),
        }
    }
}
