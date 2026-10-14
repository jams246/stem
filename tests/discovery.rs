//! Project discovery: tsconfig lookup, extends, references and file rules.

mod common;

use common::{TempProject, code, paths, stderr};

const FN: &str = "export function f() {\n  return 1;\n}\n";

#[test]
fn tsconfig_in_dir_should_decide_the_files() {
    let p = TempProject::new("tsconfig-in-dir");
    p.write("tsconfig.json", r#"{ "include": ["src"] }"#)
        .write("src/a.ts", FN)
        .write("other/b.ts", FN);
    assert_eq!(paths(&p.snapshot(&[])), vec!["src/a.ts"]);
}

#[test]
fn tsconfig_in_parent_should_set_the_root() {
    let p = TempProject::new("tsconfig-parent");
    p.write("tsconfig.json", "{}")
        .write("packages/app/x.ts", FN)
        .write("top.ts", FN);
    let output = p.stem(&["scan", "packages/app", "--json"]);
    assert_eq!(code(&output), 0, "{}", stderr(&output));
    let snap = p.snapshot(&["packages/app"]);
    assert_eq!(paths(&snap), vec!["packages/app/x.ts", "top.ts"]);
}

#[test]
fn extends_through_a_relative_path_should_be_followed() {
    let p = TempProject::new("extends-relative");
    p.write("tsconfig.json", r#"{ "extends": "./configs/base" }"#)
        .write("configs/base.json", r#"{ "include": ["../lib"] }"#)
        .write("lib/a.ts", FN)
        .write("src/b.ts", FN);
    assert_eq!(paths(&p.snapshot(&[])), vec!["lib/a.ts"]);
}

#[test]
fn extends_through_a_package_should_resolve_from_node_modules() {
    let p = TempProject::new("extends-package");
    p.write("tsconfig.json", r#"{ "extends": "@acme/tsconfig" }"#)
        .write(
            "node_modules/@acme/tsconfig/package.json",
            r#"{ "name": "@acme/tsconfig", "version": "1.0.0" }"#,
        )
        .write(
            "node_modules/@acme/tsconfig/tsconfig.json",
            r#"{ "include": ["../../../app"] }"#,
        )
        .write("app/a.ts", FN)
        .write("src/b.ts", FN);
    assert_eq!(paths(&p.snapshot(&[])), vec!["app/a.ts"]);
}

#[test]
fn unresolvable_extends_should_exit_2() {
    let p = TempProject::new("extends-missing");
    p.write("tsconfig.json", r#"{ "extends": "@missing/config" }"#)
        .write("a.ts", FN);
    assert_eq!(code(&p.stem(&["scan"])), 2);
}

#[test]
fn solution_style_references_should_add_each_project_once() {
    let p = TempProject::new("references");
    p.write(
        "tsconfig.json",
        r#"{ "files": [], "references": [{ "path": "./packages/a" }, { "path": "./packages/b/tsconfig.build.json" }] }"#,
    )
    .write("packages/a/tsconfig.json", r#"{ "include": ["src"] }"#)
    .write("packages/a/src/x.ts", FN)
    .write("packages/b/tsconfig.build.json", r#"{ "include": ["src", "../a/src"] }"#)
    .write("packages/b/src/y.ts", FN)
    .write("stray.ts", FN);
    let snap = p.snapshot(&[]);
    assert_eq!(
        paths(&snap),
        vec!["packages/a/src/x.ts", "packages/b/src/y.ts"]
    );
}

#[test]
fn tsconfig_with_comments_and_trailing_commas_should_parse() {
    let p = TempProject::new("jsonc");
    p.write(
        "tsconfig.json",
        "{\n  // only src\n  \"include\": [\"src\",],\n  /* done */\n}\n",
    )
    .write("src/a.ts", FN)
    .write("b.ts", FN);
    assert_eq!(paths(&p.snapshot(&[])), vec!["src/a.ts"]);
}

#[test]
fn no_tsconfig_should_scan_every_typescript_file() {
    let p = TempProject::new("no-tsconfig");
    p.write("a.ts", FN)
        .write("deep/b.tsx", FN)
        .write("c.mts", FN)
        .write("d.cts", FN);
    let snap = p.snapshot(&[]);
    assert_eq!(paths(&snap), vec!["a.ts", "c.mts", "d.cts", "deep/b.tsx"]);
}

#[test]
fn declaration_test_dist_and_javascript_files_should_be_skipped() {
    let p = TempProject::new("skipped");
    p.write("src/a.ts", FN)
        .write("src/types.d.ts", "export declare function g(): void;\n")
        .write("src/a.test.ts", FN)
        .write("src/a.spec.tsx", FN)
        .write("dist/out.ts", FN)
        .write("src/legacy.js", "function h() { return 1; }\n")
        .write("node_modules/pkg/index.ts", FN);
    let snap = p.snapshot(&[]);
    assert_eq!(paths(&snap), vec!["src/a.ts"]);
}

#[test]
fn default_exclude_should_skip_out_dir() {
    let p = TempProject::new("outdir");
    p.write(
        "tsconfig.json",
        r#"{ "compilerOptions": { "outDir": "build" } }"#,
    )
    .write("src/a.ts", FN)
    .write("build/a.ts", FN);
    assert_eq!(paths(&p.snapshot(&[])), vec!["src/a.ts"]);
}

#[cfg(unix)]
#[test]
fn symlinks_should_not_be_followed() {
    let outside = TempProject::new("symlink-target");
    outside.write("real/x.ts", FN).write("file.ts", FN);
    let p = TempProject::new("symlink");
    p.write("a.ts", FN);
    std::os::unix::fs::symlink(outside.path("real"), p.path("linked")).unwrap();
    std::os::unix::fs::symlink(outside.path("file.ts"), p.path("link.ts")).unwrap();
    assert_eq!(paths(&p.snapshot(&[])), vec!["a.ts"]);
}

#[test]
fn missing_folder_should_exit_2() {
    let p = TempProject::new("missing");
    assert_eq!(code(&p.stem(&["scan", "does-not-exist"])), 2);
}

#[test]
fn folder_without_typescript_should_exit_2() {
    let p = TempProject::new("empty");
    p.write("readme.md", "hi\n");
    assert_eq!(code(&p.stem(&["scan"])), 2);
}
