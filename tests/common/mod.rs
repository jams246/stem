//! Shared helpers: throwaway projects and running the `stem` binary.
#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU32, Ordering};

static COUNTER: AtomicU32 = AtomicU32::new(0);

/// A folder under the test target directory, removed on drop.
pub struct TempProject {
    pub root: PathBuf,
}

impl TempProject {
    pub fn new(tag: &str) -> TempProject {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let root = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("{tag}-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        TempProject { root }
    }

    pub fn write(&self, rel: &str, content: &str) -> &Self {
        let path = self.root.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
        self
    }

    pub fn read(&self, rel: &str) -> String {
        fs::read_to_string(self.root.join(rel)).unwrap()
    }

    pub fn remove(&self, rel: &str) {
        fs::remove_file(self.root.join(rel)).unwrap();
    }

    pub fn path(&self, rel: &str) -> PathBuf {
        self.root.join(rel)
    }

    /// Runs `stem` with `args` from the project root.
    pub fn stem(&self, args: &[&str]) -> Output {
        stem_in(&self.root, args)
    }

    /// Copies every file of a fixture folder into the project.
    pub fn copy_from(&self, fixture: &Path) -> &Self {
        copy_dir(fixture, &self.root);
        self
    }
}

impl Drop for TempProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

pub fn copy_dir(from: &Path, to: &Path) {
    let mut entries: Vec<_> = fs::read_dir(from).unwrap().map(|e| e.unwrap()).collect();
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            fs::create_dir_all(&target).unwrap();
            copy_dir(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

pub fn fixture(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(rel)
}

pub fn stem_in(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_stem"))
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap()
}

pub fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

pub fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).unwrap()
}

pub fn code(output: &Output) -> i32 {
    output.status.code().unwrap_or(-1)
}

pub fn json(output: &Output) -> serde_json::Value {
    serde_json::from_slice(&output.stdout).unwrap()
}

/// Validates `instance` against a schema shipped in `schemas/`.
pub fn assert_schema(schema_file: &str, instance: &serde_json::Value) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("schemas")
        .join(schema_file);
    let schema: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    let errors: Vec<String> = validator
        .iter_errors(instance)
        .map(|e| e.to_string())
        .collect();
    assert!(errors.is_empty(), "{schema_file}: {}", errors.join("\n"));
}

impl TempProject {
    /// Runs `scan --json` with `args`, checks exit 0 and the schema.
    pub fn scan_json(&self, args: &[&str]) -> serde_json::Value {
        let mut all = vec!["scan", "--json"];
        all.extend_from_slice(args);
        let output = self.stem(&all);
        assert_eq!(code(&output), 0, "{}", stderr(&output));
        let value = json(&output);
        assert_schema("scan-report.schema.json", &value);
        value
    }

    /// Runs `scan -o` into a file next to the project and returns the snapshot.
    pub fn snapshot(&self, args: &[&str]) -> serde_json::Value {
        let file = self.root.with_extension("snapshot.json");
        let file_arg = file.to_str().unwrap().to_string();
        let mut all = vec!["scan", "-o", file_arg.as_str()];
        all.extend_from_slice(args);
        let output = self.stem(&all);
        assert_eq!(code(&output), 0, "{}", stderr(&output));
        let value: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&file).unwrap()).unwrap();
        let _ = fs::remove_file(&file);
        assert_schema("snapshot.schema.json", &value);
        value
    }

    /// Writes a baseline snapshot to `rel` (inside the project).
    pub fn baseline(&self, rel: &str, args: &[&str]) {
        let mut all = vec!["scan", "-o", rel];
        all.extend_from_slice(args);
        let output = self.stem(&all);
        assert_eq!(code(&output), 0, "{}", stderr(&output));
        assert_schema(
            "snapshot.schema.json",
            &serde_json::from_str(&self.read(rel)).unwrap(),
        );
    }

    /// Runs `compare --json` and validates the report.
    pub fn compare_json(&self, baseline: &str) -> (i32, serde_json::Value) {
        let output = self.stem(&["compare", baseline, "--json"]);
        let status = code(&output);
        assert!(status == 0 || status == 1, "{}", stderr(&output));
        let value = json(&output);
        assert_schema("compare-report.schema.json", &value);
        (status, value)
    }
}

/// Function IDs in a snapshot.
pub fn ids(snapshot: &serde_json::Value) -> Vec<String> {
    snapshot["functions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["id"].as_str().unwrap().to_string())
        .collect()
}

/// Paths of the files that contributed functions.
pub fn paths(snapshot: &serde_json::Value) -> Vec<String> {
    let mut paths: Vec<String> = ids(snapshot)
        .iter()
        .map(|id| id.split_once("::").unwrap().0.to_string())
        .collect();
    paths.dedup();
    paths
}

/// A function with `n` flat `if`s: cognitive complexity `n`.
pub fn flat_ifs(name: &str, n: usize) -> String {
    let body: String = (0..n).map(|i| format!("  if (x === {i}) {{}}\n")).collect();
    format!("export function {name}(x: number) {{\n{body}}}\n")
}
