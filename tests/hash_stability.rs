//! Hash stability: formatting and comments never change a function's hash;
//! token changes change exactly the function that owns them.

mod common;

use std::collections::BTreeMap;

use common::{TempProject, copy_dir, fixture};

fn shop(tag: &str) -> TempProject {
    let p = TempProject::new(tag);
    p.copy_from(&fixture("projects/shop"));
    p
}

fn hashes(snapshot: &serde_json::Value) -> BTreeMap<String, String> {
    snapshot["functions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            (
                f["id"].as_str().unwrap().into(),
                f["hash"].as_str().unwrap().into(),
            )
        })
        .collect()
}

fn changed_ids(before: &serde_json::Value, after: &serde_json::Value) -> Vec<String> {
    let (a, b) = (hashes(before), hashes(after));
    assert_eq!(a.keys().collect::<Vec<_>>(), b.keys().collect::<Vec<_>>());
    a.iter()
        .filter(|(id, h)| b[*id] != **h)
        .map(|(id, _)| id.clone())
        .collect()
}

fn edit(p: &TempProject, rel: &str, from: &str, to: &str) {
    let text = p.read(rel);
    assert!(text.contains(from), "{rel} lacks {from:?}");
    p.write(rel, &text.replace(from, to));
}

fn assert_reformat_keeps_hashes(variant: &str) {
    let p = shop(&format!("prettier-{variant}"));
    p.baseline("base.json", &[]);
    let before = p.snapshot(&[]);
    copy_dir(&fixture(&format!("prettier/{variant}")), &p.root);
    let (status, report) = p.compare_json("base.json");
    assert_eq!(
        (
            status,
            report["changedCode"]["changed"].as_u64(),
            changed_ids(&before, &p.snapshot(&[]))
        ),
        (0, Some(0), Vec::<String>::new())
    );
}

#[test]
fn prettier_defaults_should_change_no_function() {
    assert_reformat_keeps_hashes("default");
}

#[test]
fn prettier_narrow_settings_should_change_no_function() {
    assert_reformat_keeps_hashes("narrow");
}

#[test]
fn adding_and_removing_comments_should_change_no_function() {
    let p = shop("comments");
    let before = p.snapshot(&[]);
    edit(
        &p,
        "src/cart/total.ts",
        "let total = subtotal(order);",
        "// start\n  let total = /* base */ subtotal(order); // gross\n",
    );
    edit(
        &p,
        "src/orders/parse.ts",
        "export function",
        "/** Parses an order. */\nexport function",
    );
    let with_comments = p.snapshot(&[]);
    p.baseline("base.json", &[]);
    copy_dir(&fixture("projects/shop/src"), &p.path("src"));
    let (_, report) = p.compare_json("base.json");
    assert_eq!(
        (
            changed_ids(&before, &with_comments),
            report["changedCode"]["changed"].as_u64()
        ),
        (Vec::<String>::new(), Some(0))
    );
}

#[test]
fn renaming_one_local_should_change_exactly_that_function() {
    let p = shop("rename");
    let before = p.snapshot(&[]);
    let text = p.read("src/cart/total.ts");
    let start = text.find("export function applyDiscounts").unwrap();
    let end = text.find("export class").unwrap();
    let renamed = text[start..end].replace("total", "amount");
    p.write(
        "src/cart/total.ts",
        &format!("{}{}{}", &text[..start], renamed, &text[end..]),
    );
    assert_eq!(
        changed_ids(&before, &p.snapshot(&[])),
        vec!["src/cart/total.ts::applyDiscounts".to_string()]
    );
}

#[test]
fn changing_only_a_nested_callback_should_not_change_its_parent() {
    let p = shop("callback");
    let before = p.snapshot(&[]);
    edit(
        &p,
        "src/cart/total.ts",
        "`${item.sku} x${item.quantity}`",
        "`${item.sku} * ${item.quantity}`",
    );
    assert_eq!(
        changed_ids(&before, &p.snapshot(&[])),
        vec!["src/cart/total.ts::Checkout.summary.<anonymous 1>".to_string()]
    );
}
