//! `stem.config.json`: ignore patterns and thresholds.

use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use serde_json::Value;

use crate::error::StemError;
use crate::jsonc;
use crate::metric::{Metric, Thresholds};

/// File name of the optional project config.
pub const CONFIG_FILE: &str = "stem.config.json";

/// Effective configuration: ignore patterns and thresholds.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Config {
    /// Ignore patterns, sorted by bytes and without duplicates.
    pub ignore: Vec<String>,
    pub thresholds: Thresholds,
}

impl Config {
    /// Adds `--ignore` patterns and restores the sorted order.
    pub fn with_ignores(mut self, extra: &[String]) -> Config {
        self.ignore.extend(extra.iter().cloned());
        self.ignore.sort();
        self.ignore.dedup();
        self
    }
}

fn invalid(file: &str, message: impl Into<String>) -> StemError {
    StemError::Config {
        file: file.to_string(),
        message: message.into(),
    }
}

fn parse_thresholds(value: &Value, file: &str) -> Result<Thresholds, StemError> {
    let Value::Object(map) = value else {
        return Err(invalid(file, "\"thresholds\" must be an object"));
    };
    let mut thresholds = Thresholds::default();
    for (key, values) in map {
        let Some(metric) = Metric::from_key(key) else {
            return Err(invalid(file, format!("unknown key \"thresholds.{key}\"")));
        };
        let numbers: Option<Vec<u32>> = values.as_array().and_then(|items| {
            items
                .iter()
                .map(|v| v.as_u64().and_then(|n| u32::try_from(n).ok()))
                .collect()
        });
        let triple: [u32; 3] = numbers.and_then(|n| n.try_into().ok()).ok_or_else(|| {
            invalid(
                file,
                format!("\"thresholds.{key}\" must be a list of three whole numbers"),
            )
        })?;
        if !(triple[0] < triple[1] && triple[1] < triple[2]) {
            return Err(invalid(
                file,
                format!("\"thresholds.{key}\" must be strictly increasing"),
            ));
        }
        thresholds.set(metric, triple);
    }
    Ok(thresholds)
}

/// Validates a parsed config object. `file` names the source in messages.
pub fn from_value(value: &Value, file: &str) -> Result<Config, StemError> {
    let Value::Object(map) = value else {
        return Err(invalid(file, "the config must be a JSON object"));
    };
    let mut config = Config::default();
    for (key, value) in map {
        match key.as_str() {
            "ignore" => {
                let patterns: Option<Vec<String>> = value.as_array().and_then(|items| {
                    items
                        .iter()
                        .map(|v| v.as_str().map(str::to_string))
                        .collect()
                });
                config.ignore = patterns
                    .ok_or_else(|| invalid(file, "\"ignore\" must be a list of strings"))?;
            }
            "thresholds" => config.thresholds = parse_thresholds(value, file)?,
            other => return Err(invalid(file, format!("unknown key \"{other}\""))),
        }
    }
    Ok(config.with_ignores(&[]))
}

/// Parses JSONC config text.
pub fn parse(text: &str, file: &str) -> Result<Config, StemError> {
    let value = jsonc::parse(text).map_err(|e| invalid(file, e))?;
    from_value(&value, file)
}

/// Reads `stem.config.json` from the project root, if present.
pub fn load(root: &Path) -> Result<Option<Config>, StemError> {
    match fs::read_to_string(root.join(CONFIG_FILE)) {
        Ok(text) => parse(&text, CONFIG_FILE).map(Some),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
        Err(e) => Err(StemError::read(CONFIG_FILE, e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn message(text: &str) -> String {
        parse(text, CONFIG_FILE).unwrap_err().to_string()
    }

    #[test]
    fn parse_should_keep_defaults_for_missing_metrics() {
        let config = parse("{ \"thresholds\": { \"length\": [10, 20, 40] } }", "c").unwrap();
        assert_eq!(config.thresholds.get(Metric::Parameters), [3, 4, 6]);
    }

    #[test]
    fn parse_should_sort_ignore_patterns() {
        let config = parse("{ \"ignore\": [\"b\", \"a\", \"b\"] }", "c").unwrap();
        assert_eq!(config.ignore, vec!["a", "b"]);
    }

    #[test]
    fn parse_should_name_unknown_keys() {
        assert!(message("{ \"ignores\": [] }").contains("\"ignores\""));
    }

    #[test]
    fn parse_should_reject_non_integer_thresholds() {
        assert!(
            message("{ \"thresholds\": { \"length\": [1, 2.5, 3] } }")
                .contains("thresholds.length")
        );
    }

    #[test]
    fn parse_should_reject_thresholds_that_do_not_increase() {
        assert!(
            message("{ \"thresholds\": { \"vocabulary\": [5, 5, 9] } }")
                .contains("strictly increasing")
        );
    }

    #[test]
    fn parse_should_name_unknown_metrics() {
        assert!(message("{ \"thresholds\": { \"size\": [1, 2, 3] } }").contains("thresholds.size"));
    }
}
