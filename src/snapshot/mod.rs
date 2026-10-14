//! The snapshot file: written by `scan -o`, read by `compare`.

mod read;
mod write;

pub use read::{Baseline, read_baseline};
pub use write::{
    config_json, files_json, function_json, parse_errors_json, snapshot_json, summary_json,
    usage_gap_json,
};
