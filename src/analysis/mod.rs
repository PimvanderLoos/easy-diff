//! Two-pass analysis orchestration: Pass 1 (global PR summary) followed by
//! parallel Pass 2 (per-file categorisation and annotation).

pub mod pass1_summary;
pub mod pass2_files;
