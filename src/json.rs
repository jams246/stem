//! Ordered JSON values and the fixed output layout.
//!
//! Objects keep insertion order, numbers can be written pre-formatted (scores
//! with one decimal place from integer tenths), and the layout is fixed:
//! 2-space indent, scalar-only arrays and objects on one line.

/// A JSON value with ordered object keys.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Json {
    Null,
    Bool(bool),
    Int(i64),
    /// A number already formatted, such as `72.4`.
    Number(String),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

impl Json {
    /// Builds an object from `(key, value)` pairs.
    pub fn obj<const N: usize>(pairs: [(&str, Json); N]) -> Json {
        Json::Obj(pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
    }

    pub fn str(text: impl Into<String>) -> Json {
        Json::Str(text.into())
    }

    pub fn int(value: impl TryInto<i64>) -> Json {
        Json::Int(value.try_into().unwrap_or(i64::MAX))
    }

    /// A score in tenths written with one decimal place.
    pub fn tenths(value: u32) -> Json {
        Json::Number(crate::score::tenths(value))
    }

    /// A score in tenths, or `null`.
    pub fn opt_tenths(value: Option<u32>) -> Json {
        value.map_or(Json::Null, Json::tenths)
    }

    fn is_scalar(&self) -> bool {
        !matches!(self, Json::Arr(_) | Json::Obj(_))
    }

    fn is_flat(&self) -> bool {
        match self {
            Json::Arr(items) => items.iter().all(Json::is_scalar),
            Json::Obj(pairs) => pairs.iter().all(|(_, v)| v.is_scalar()),
            _ => true,
        }
    }

    /// Renders the value with the fixed layout and a trailing newline.
    pub fn render(&self) -> String {
        let mut out = String::new();
        self.write(&mut out, 0);
        out.push('\n');
        out
    }

    fn write(&self, out: &mut String, indent: usize) {
        match self {
            Json::Null => out.push_str("null"),
            Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Json::Int(i) => out.push_str(&i.to_string()),
            Json::Number(n) => out.push_str(n),
            Json::Str(s) => write_str(out, s),
            Json::Arr(items) if items.is_empty() => out.push_str("[]"),
            Json::Obj(pairs) if pairs.is_empty() => out.push_str("{}"),
            Json::Arr(items) if self.is_flat() => {
                out.push('[');
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    item.write(out, indent);
                }
                out.push(']');
            }
            Json::Obj(pairs) if self.is_flat() => {
                out.push_str("{ ");
                for (i, (key, value)) in pairs.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    write_str(out, key);
                    out.push_str(": ");
                    value.write(out, indent);
                }
                out.push_str(" }");
            }
            Json::Arr(items) => {
                out.push('[');
                for (i, item) in items.iter().enumerate() {
                    out.push_str(if i > 0 { ",\n" } else { "\n" });
                    push_indent(out, indent + 1);
                    item.write(out, indent + 1);
                }
                out.push('\n');
                push_indent(out, indent);
                out.push(']');
            }
            Json::Obj(pairs) => {
                out.push('{');
                for (i, (key, value)) in pairs.iter().enumerate() {
                    out.push_str(if i > 0 { ",\n" } else { "\n" });
                    push_indent(out, indent + 1);
                    write_str(out, key);
                    out.push_str(": ");
                    value.write(out, indent + 1);
                }
                out.push('\n');
                push_indent(out, indent);
                out.push('}');
            }
        }
    }
}

fn push_indent(out: &mut String, indent: usize) {
    for _ in 0..indent {
        out.push_str("  ");
    }
}

fn write_str(out: &mut String, text: &str) {
    match serde_json::to_string(text) {
        Ok(quoted) => out.push_str(&quoted),
        Err(_) => out.push_str("\"\""),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_should_inline_scalar_arrays_and_objects() {
        let value = Json::obj([
            ("a", Json::Arr(vec![Json::int(1), Json::int(2)])),
            ("b", Json::obj([("x", Json::tenths(724))])),
            ("c", Json::Arr(vec![Json::obj([("k", Json::str("v"))])])),
        ]);
        let expected = "{\n  \"a\": [1, 2],\n  \"b\": { \"x\": 72.4 },\n  \"c\": [\n    { \"k\": \"v\" }\n  ]\n}\n";
        assert_eq!(value.render(), expected);
    }
}
