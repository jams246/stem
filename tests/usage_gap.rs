//! Usage gap counts source separation while respecting binding identity.

use stem::analyze::{FileOutcome, FunctionMetrics, analyze_source};
use stem::metric::Metric;

fn function(source: &str, name: &str) -> FunctionMetrics {
    let FileOutcome::Functions(functions) = analyze_source("gap.tsx", source) else {
        panic!("could not parse {source}");
    };
    functions.into_iter().find(|f| f.name() == name).unwrap()
}

#[test]
fn syntax_and_scope_cases_should_count_only_whole_intervening_statements() {
    let cases = [
        ("let x = 0; x++; x += 1; return x;", 0),
        ("let x = 0; one(); two(); x++;", 2),
        ("let x = 0; if (flag) { one(); two(); } use(x);", 3),
        ("let x = 0; function helper() { one(); two(); } use(x);", 1),
        ("let x = 0; type T = typeof x; use(x);", 1),
        ("let x = 0; interface T { value: typeof x }; use(x);", 1),
        ("let x = 0; { let x = 1; use(x); } use(x);", 2),
        ("let x = 0; other.x; use(x);", 1),
        ("let x = 0; ({ x }); use(x);", 0),
        ("let x = 0; other[x]; use(x);", 0),
        ("let x = 0; one(); x++; two(); x = 3;", 1),
        ("let x = {}; one(); x.value = 1; two(); x.read();", 1),
        ("const { a, b } = data; one(); use(a, b);", 1),
        ("var x = 1; one(); var x; two(); use(x);", 1),
        ("use(x); one(); var x = 1;", 1),
        ("for (let x = 0; x < 5; x++) { one(); use(x); }", 1),
        ("for (const x of list) { one(); use(x); }", 1),
        (
            "let x = 0; for (let y = 0; y < 5; y++) { one(); } use(x);",
            2,
        ),
        ("let x = 0; label: { ; one(); } use(x);", 1),
        ("let x = 0; use(x, x, x);", 0),
        ("let unused = 0; one(); two();", 0),
        (
            "try { one(); } catch (err) { two(); three(); use(err); }",
            0,
        ),
        ("let x = 0; const cb = () => x;", 0),
        ("let x = 0; const cb = () => x; use(x);", 1),
        (
            "const items = []; const add = x => items.push(x); add(1); add(2); return items;",
            3,
        ),
        (
            "let x = 0; class C { field = use(x); static { use(x); } } use(x);",
            1,
        ),
        (
            "let x = 0; namespace N { export const value = use(x); } use(x);",
            1,
        ),
        ("const Component = make(); one(); return <Component />;", 1),
        (
            "const amount = 0; one(); return <div amount={amount} />;",
            1,
        ),
        (
            "const amount = 0; one(); return <div amount='literal' />;",
            0,
        ),
    ];
    for (body, expected) in cases {
        let f = function(&format!("function f() {{ {body} }}"), "f");
        assert_eq!(f.value(Metric::UsageGap), expected, "{body}");
        assert_eq!(f.usage_gap_evidence.is_some(), expected > 0, "{body}");
    }
}

#[test]
fn evidence_should_locate_the_largest_gap_and_break_ties_in_source_order() {
    let f = function(
        "function f() {\nconst a = 0, b = 1;\none();\ntwo();\nuse(a, b);\n}",
        "f",
    );
    let e = f.usage_gap_evidence.unwrap();
    assert_eq!(
        (e.name.as_str(), e.declaration_line, e.from_line, e.to_line),
        ("a", 2, 2, 5)
    );
}

#[test]
fn regrouping_should_change_only_usage_gap() {
    let before = function(
        "function f() {\nlet amount = subtotal;\none();\ntwo();\namount -= discount;\namount += delivery;\n}",
        "f",
    );
    let after = function(
        "function f() {\none();\ntwo();\nlet amount = subtotal;\namount -= discount;\namount += delivery;\n}",
        "f",
    );
    assert_eq!(
        (&before.values[..4], before.value(Metric::UsageGap)),
        (&after.values[..4], 2)
    );
    assert_eq!(after.value(Metric::UsageGap), 0);
}

#[test]
fn nested_functions_should_measure_their_own_locals_only() {
    let source =
        "function f(input) { let x = 0; function g() { const y = 0; one(); use(y, x, input); } }";
    assert_eq!(
        (
            function(source, "f").value(Metric::UsageGap),
            function(source, "f.g").value(Metric::UsageGap)
        ),
        (0, 1)
    );
}

#[test]
fn parameters_and_outer_bindings_should_not_contribute() {
    let f = function(
        "const global = 1; function f(input) { one(); two(); return input + global; }",
        "f",
    );
    assert_eq!((f.value(Metric::UsageGap), f.usage_gap_evidence), (0, None));
}

#[test]
fn redeclared_parameters_and_function_names_should_stay_excluded() {
    let parameter = function(
        "function f(input) { var input; work(); return input; }",
        "f",
    );
    assert_eq!(parameter.value(Metric::UsageGap), 0);
    let named = function(
        "function f() { var helper; function helper() {} work(); helper(); }",
        "f",
    );
    assert_eq!(named.value(Metric::UsageGap), 0);
}

#[test]
fn methods_and_arrow_fields_should_measure_their_own_bodies() {
    let source = "class C { method(p) { const x = 0; work(); return x + p; } arrow = () => { const y = 0; work(); return y; } }";
    assert_eq!(function(source, "C.method").value(Metric::UsageGap), 1);
    assert_eq!(function(source, "C.arrow").value(Metric::UsageGap), 1);
}

#[test]
fn formatting_should_preserve_gap_and_hash_but_update_locations() {
    let before = function("function f() { const x = 0; work(); use(x); }", "f");
    let after = function(
        "function f() {\r\n const x = 0\r\n // comment\r\n\r\n work(\r\n )\r\n use(x)\r\n}",
        "f",
    );
    assert_eq!(
        (before.value(Metric::UsageGap), before.hash),
        (after.value(Metric::UsageGap), after.hash)
    );
    assert_eq!(after.usage_gap_evidence.unwrap().to_line, 7);
}
