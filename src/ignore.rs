//! Built-in and user ignore patterns.

use globset::{Glob, GlobBuilder, GlobSet, GlobSetBuilder};

use crate::error::StemError;

/// Patterns that always apply. Fixed by `metricsVersion`.
pub const BUILTIN_IGNORES: [&str; 6] = [
    "**/node_modules/**",
    "**/dist/**",
    "**/*.test.ts",
    "**/*.test.tsx",
    "**/*.spec.ts",
    "**/*.spec.tsx",
];

fn glob(pattern: &str) -> Result<Glob, globset::Error> {
    GlobBuilder::new(pattern)
        .literal_separator(true)
        .backslash_escape(true)
        .build()
}

fn invalid(pattern: &str, message: impl Into<String>) -> StemError {
    StemError::Pattern {
        pattern: pattern.to_string(),
        message: message.into(),
    }
}

/// Compiles one user pattern. A pattern naming a folder also matches
/// everything inside it.
fn compile_user(pattern: &str) -> Result<GlobSet, StemError> {
    if pattern.is_empty() {
        return Err(invalid(pattern, "the pattern is empty"));
    }
    if pattern.starts_with('!') {
        return Err(invalid(pattern, "negation with ! is not supported"));
    }
    let base = pattern.trim_end_matches('/');
    let base = if base.is_empty() { pattern } else { base };
    let mut builder = GlobSetBuilder::new();
    builder.add(glob(base).map_err(|e| invalid(pattern, e.kind().to_string()))?);
    let inside = format!("{base}/**");
    builder.add(glob(&inside).map_err(|e| invalid(pattern, e.kind().to_string()))?);
    builder
        .build()
        .map_err(|e| invalid(pattern, e.kind().to_string()))
}

/// Result of filtering candidate files.
#[derive(Debug, Default)]
pub struct Filtered {
    /// Files that pass every filter, in input order.
    pub kept: Vec<String>,
    /// Number of candidates removed.
    pub ignored: usize,
    /// User patterns that matched no candidate.
    pub unmatched: Vec<String>,
}

/// Compiled built-in and user patterns.
pub struct Ignores {
    builtin: GlobSet,
    user: Vec<(String, GlobSet)>,
}

impl Ignores {
    /// Compiles the built-ins plus `patterns`.
    pub fn new(patterns: &[String]) -> Result<Ignores, StemError> {
        let mut builder = GlobSetBuilder::new();
        for pattern in BUILTIN_IGNORES {
            builder.add(glob(pattern).map_err(|e| invalid(pattern, e.kind().to_string()))?);
        }
        let builtin = builder
            .build()
            .map_err(|e| invalid("built-in", e.kind().to_string()))?;
        let user = patterns
            .iter()
            .map(|p| compile_user(p).map(|set| (p.clone(), set)))
            .collect::<Result<_, _>>()?;
        Ok(Ignores { builtin, user })
    }

    /// Applies built-ins first, then user patterns, to project-relative paths.
    pub fn filter(&self, candidates: Vec<String>) -> Filtered {
        let mut matched = vec![false; self.user.len()];
        let mut result = Filtered::default();
        for path in candidates {
            let mut ignored = self.builtin.is_match(&path);
            for (i, (_, set)) in self.user.iter().enumerate() {
                if set.is_match(&path) {
                    matched[i] = true;
                    ignored = true;
                }
            }
            if ignored {
                result.ignored += 1;
            } else {
                result.kept.push(path);
            }
        }
        result.unmatched = self
            .user
            .iter()
            .zip(matched)
            .filter(|(_, m)| !m)
            .map(|((p, _), _)| p.clone())
            .collect();
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kept(patterns: &[&str], paths: &[&str]) -> Vec<String> {
        let patterns: Vec<String> = patterns.iter().map(|p| p.to_string()).collect();
        let paths = paths.iter().map(|p| p.to_string()).collect();
        Ignores::new(&patterns).unwrap().filter(paths).kept
    }

    #[test]
    fn folder_pattern_should_ignore_everything_inside() {
        assert_eq!(
            kept(&["src/legacy"], &["src/legacy/a/b.ts", "src/a.ts"]),
            vec!["src/a.ts"]
        );
    }

    #[test]
    fn patterns_should_be_anchored_at_the_root() {
        assert_eq!(
            kept(&["legacy/**"], &["src/legacy/a.ts"]),
            vec!["src/legacy/a.ts"]
        );
    }

    #[test]
    fn leading_double_star_should_match_anywhere() {
        assert!(kept(&["**/legacy/**"], &["src/legacy/a.ts"]).is_empty());
    }

    #[test]
    fn matching_should_be_case_sensitive() {
        assert_eq!(kept(&["Src/"], &["src/a.ts"]), vec!["src/a.ts"]);
    }

    #[test]
    fn braces_and_classes_should_work() {
        let paths = ["a.gen.ts", "a.auto.ts", "b.ts", "x.ts"];
        assert_eq!(kept(&["*.{gen,auto}.ts", "[abc].ts"], &paths), vec!["x.ts"]);
    }

    #[test]
    fn star_should_not_cross_folders() {
        assert_eq!(kept(&["src/*.ts"], &["src/a/b.ts"]), vec!["src/a/b.ts"]);
    }

    #[test]
    fn invalid_pattern_should_be_rejected() {
        assert!(Ignores::new(&["src/[a".to_string()]).is_err());
    }
}
