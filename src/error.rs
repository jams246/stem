//! Errors that end a command with exit code 2.

use std::io;

/// A failure that stops `stem` before it can produce a result.
#[derive(Debug, thiserror::Error)]
pub enum StemError {
    /// The command line was wrong.
    #[error("{0}")]
    Usage(String),
    /// A path could not be read.
    #[error("cannot read {path}: {source}")]
    Read {
        path: String,
        #[source]
        source: io::Error,
    },
    /// The snapshot file could not be written.
    #[error("cannot write {path}: {source}")]
    Write {
        path: String,
        #[source]
        source: io::Error,
    },
    /// `stem.config.json` is invalid.
    #[error("invalid config {file}: {message}")]
    Config { file: String, message: String },
    /// A tsconfig file is invalid or cannot be resolved.
    #[error("invalid tsconfig {file}: {message}")]
    Tsconfig { file: String, message: String },
    /// An ignore pattern is invalid.
    #[error("invalid ignore pattern \"{pattern}\": {message}")]
    Pattern { pattern: String, message: String },
    /// Nothing to measure.
    #[error("no TypeScript files to scan in {0}")]
    NoFiles(String),
    /// The baseline cannot be used.
    #[error("cannot use baseline {path}: {message}")]
    Baseline { path: String, message: String },
    /// `stem explain` got a topic it does not know.
    #[error("unknown topic \"{topic}\". Topics: {topics}")]
    UnknownTopic { topic: String, topics: String },
}

impl StemError {
    /// Shorthand for a read failure.
    pub fn read(path: impl Into<String>, source: io::Error) -> Self {
        StemError::Read {
            path: path.into(),
            source,
        }
    }
}
