//! Determinism: byte-identical snapshots regardless of run, file creation
//! order, line endings and thread count.

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use common::{TempProject, code, fixture};

fn files(dir: &Path, base: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files(&path, base, out);
        } else {
            out.push(path.strip_prefix(base).unwrap().to_path_buf());
        }
    }
}

/// Writes the shop fixture, transforming each file and choosing the order.
fn shop(tag: &str, reverse: bool, transform: impl Fn(&str) -> String) -> TempProject {
    let src = fixture("projects/shop");
    let mut list = Vec::new();
    files(&src, &src, &mut list);
    list.sort();
    if reverse {
        list.reverse();
    }
    let p = TempProject::new(tag);
    for rel in list {
        let text = fs::read_to_string(src.join(&rel)).unwrap();
        p.write(rel.to_str().unwrap(), &transform(&text));
    }
    p
}

fn snapshot_bytes(p: &TempProject, threads: Option<&str>) -> Vec<u8> {
    let out = p.root.with_extension("det.json");
    let mut command = Command::new(env!("CARGO_BIN_EXE_stem"));
    command
        .args(["scan", "-o", out.to_str().unwrap()])
        .current_dir(&p.root);
    if let Some(threads) = threads {
        command.env("RAYON_NUM_THREADS", threads);
    } else {
        command.env_remove("RAYON_NUM_THREADS");
    }
    let output = command.output().unwrap();
    assert_eq!(code(&output), 0);
    let bytes = fs::read(&out).unwrap();
    let _ = fs::remove_file(out);
    bytes
}

fn lf(text: &str) -> String {
    text.replace("\r\n", "\n")
}

#[test]
fn scanning_twice_should_give_identical_bytes() {
    let p = shop("det-twice", false, lf);
    assert_eq!(snapshot_bytes(&p, None), snapshot_bytes(&p, None));
}

#[test]
fn reverse_creation_order_should_give_an_identical_snapshot() {
    let forward = shop("det-forward", false, lf);
    let reverse = shop("det-reverse", true, lf);
    assert_eq!(
        snapshot_bytes(&forward, None),
        snapshot_bytes(&reverse, None)
    );
}

#[test]
fn crlf_line_endings_should_give_an_identical_snapshot() {
    let plain = shop("det-lf", false, lf);
    let crlf = shop("det-crlf", false, |t| lf(t).replace('\n', "\r\n"));
    assert_eq!(snapshot_bytes(&plain, None), snapshot_bytes(&crlf, None));
}

#[test]
fn byte_order_marks_should_give_an_identical_snapshot() {
    let plain = shop("det-plain", false, lf);
    let bom = shop("det-bom", false, |t| format!("\u{feff}{}", lf(t)));
    assert_eq!(snapshot_bytes(&plain, None), snapshot_bytes(&bom, None));
}

#[test]
fn one_thread_and_all_cores_should_give_an_identical_snapshot() {
    let p = shop("det-threads", false, lf);
    assert_eq!(snapshot_bytes(&p, Some("1")), snapshot_bytes(&p, None));
}

#[test]
fn snapshot_should_hold_no_absolute_paths() {
    let p = shop("det-paths", false, lf);
    let text = String::from_utf8(snapshot_bytes(&p, None)).unwrap();
    let root = p.root.to_str().unwrap();
    assert!(!text.contains(root) && !text.contains('\\') && text.ends_with("}\n"));
}
