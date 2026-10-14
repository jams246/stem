//! `stem` scores how easy a TypeScript codebase is for a human to follow.

#![warn(clippy::unwrap_used, clippy::expect_used)]

pub mod analyze;
pub mod cli;
pub mod config;
pub mod error;
pub mod explain;
pub mod hints;
pub mod ignore;
pub mod json;
mod jsonc;
pub mod metric;
pub mod project;
pub mod report;
pub mod scan;
pub mod score;
pub mod snapshot;

/// Version of this build of `stem`.
pub const STEM_VERSION: &str = env!("CARGO_PKG_VERSION");
/// Version of the snapshot file layout.
pub const SCHEMA_VERSION: u32 = 2;
/// Version of the counting rules, default thresholds, default ignores and scoring.
pub const METRICS_VERSION: u32 = 3;
