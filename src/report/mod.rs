//! Text and JSON reports for `scan` and `compare`.

pub mod compare;
mod compare_json;
mod compare_text;
mod entry;
pub mod format;
mod scan_report;

pub use compare_json::compare_json;
pub use compare_text::compare_text;
pub use scan_report::{scan_json, scan_text, top_functions};
